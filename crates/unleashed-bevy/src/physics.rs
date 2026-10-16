//! Static stage collision built from the original Havok rigid bodies.

use bevy::prelude::*;
use parry3d::{
    math::{Pose, Vector},
    query::{self, Ray, RayCast, ShapeCastOptions},
    shape::{Ball, TriMesh},
};
use unleashed_assets::collision::havok::CollisionShape;

use crate::Stage;

/// Gap kept between Sonic's collision sphere and the ground.
pub const SKIN: f32 = 0.02;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        let stage = &app.world().resource::<Stage>().0;
        let world = CollisionWorld::build(stage);
        app.insert_resource(world);
    }
}

#[derive(Resource)]
pub struct CollisionWorld {
    solid: TriMesh,
    kill_mesh: Option<TriMesh>,
    /// (center, half extents) of Havok box kill volumes.
    kill_boxes: Vec<(Vec3, Vec3)>,
    pub lowest_point: f32,
    pub triangle_count: usize,
    pub debug_triangles: Vec<[Vec3; 3]>,
}

pub struct SweepHit {
    /// Fraction of the requested motion travelled before contact.
    pub fraction: f32,
    pub point: Vec3,
    /// Surface normal facing the swept sphere.
    pub normal: Vec3,
}

fn pv(v: Vec3) -> Vector {
    Vector::new(v.x, v.y, v.z)
}

fn bv(v: Vector) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

impl CollisionWorld {
    fn build(stage: &unleashed_assets::stage::day::DayStage) -> Self {
        let mut solid_v = Vec::new();
        let mut solid_i = Vec::new();
        let mut kill_v = Vec::new();
        let mut kill_i = Vec::new();
        let mut kill_boxes = Vec::new();
        let mut lowest_point = f32::MAX;
        let mut debug_triangles = Vec::new();
        for body in &stage.collision {
            match &body.shape {
                CollisionShape::Mesh {
                    vertices,
                    triangles,
                } => {
                    let (verts, idx) = if body.is_kill_volume() {
                        (&mut kill_v, &mut kill_i)
                    } else {
                        (&mut solid_v, &mut solid_i)
                    };
                    let base = verts.len() as u32;
                    for v in vertices {
                        lowest_point = lowest_point.min(v[1]);
                        verts.push(Vector::new(v[0], v[1], v[2]));
                    }
                    for t in triangles {
                        idx.push([base + t[0], base + t[1], base + t[2]]);
                        if !body.is_kill_volume() {
                            debug_triangles.push(t.map(|i| Vec3::from(vertices[i as usize])));
                        }
                    }
                }
                CollisionShape::Box {
                    center,
                    half_extents,
                } => kill_boxes.push((Vec3::from(*center), Vec3::from(*half_extents))),
            }
        }
        let triangle_count = solid_i.len();
        let solid = TriMesh::new(solid_v, solid_i).expect("stage collision mesh");
        let kill_mesh = (!kill_i.is_empty()).then(|| TriMesh::new(kill_v, kill_i).expect("kill mesh"));
        info!(
            "collision: {triangle_count} solid triangles, kill mesh: {}, kill boxes: {}",
            kill_mesh.is_some(),
            kill_boxes.len()
        );
        Self {
            solid,
            kill_mesh,
            kill_boxes,
            lowest_point,
            triangle_count,
            debug_triangles,
        }
    }

    /// Plain solid triangles (no kill volumes): another world's collision, e.g. the
    /// Minecraft world's block faces.
    pub fn from_triangles(triangles: &[[[f32; 3]; 3]]) -> Self {
        let mut verts = Vec::with_capacity(triangles.len() * 3);
        let mut idx = Vec::with_capacity(triangles.len());
        let mut lowest_point = f32::MAX;
        for t in triangles {
            let base = verts.len() as u32;
            for v in t {
                lowest_point = lowest_point.min(v[1]);
                verts.push(Vector::new(v[0], v[1], v[2]));
            }
            idx.push([base, base + 1, base + 2]);
        }
        if idx.is_empty() {
            // TriMesh needs a triangle; one far below the world stands in for none.
            verts = vec![Vector::new(0.0, -1e6, 0.0), Vector::new(1.0, -1e6, 0.0), Vector::new(0.0, -1e6, 1.0)];
            idx = vec![[0, 1, 2]];
        }
        Self {
            solid: TriMesh::new(verts, idx).expect("triangle collision mesh"),
            kill_mesh: None,
            kill_boxes: Vec::new(),
            lowest_point,
            triangle_count: triangles.len(),
            debug_triangles: Vec::new(),
        }
    }

    /// Sweeps a sphere from `center` along `motion`.
    pub fn sweep(&self, center: Vec3, radius: f32, motion: Vec3) -> Option<SweepHit> {
        if motion.length_squared() < 1e-12 {
            return None;
        }
        let ball = Ball::new(radius);
        let options = ShapeCastOptions {
            max_time_of_impact: 1.0,
            target_distance: 0.0,
            stop_at_penetration: false,
            compute_impact_geometry_on_penetration: true,
        };
        let hit = query::cast_shapes(
            &Pose::from_translation(pv(center)),
            pv(motion),
            &ball,
            &Pose::identity(),
            Vector::ZERO,
            &self.solid,
            options,
        )
        .ok()
        .flatten()?;
        let fraction = hit.time_of_impact.clamp(0.0, 1.0);
        let at = center + motion * fraction;
        let point = bv(hit.witness2);
        let normal = (at - point).try_normalize().unwrap_or(-bv(hit.normal1));
        Some(SweepHit {
            fraction,
            point,
            normal,
        })
    }

    /// Ray cast against solid collision; returns (distance, normal facing the ray).
    pub fn ray(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<(f32, Vec3)> {
        let ray = Ray::new(pv(origin), pv(dir));
        let hit = self
            .solid
            .cast_ray_and_get_normal(&Pose::identity(), &ray, max, false)?;
        let mut normal = bv(hit.normal);
        if normal.dot(dir) > 0.0 {
            normal = -normal;
        }
        Some((hit.time_of_impact, normal))
    }

    /// Pushes a sphere out of any geometry it overlaps.
    pub fn depenetrate(&self, center: Vec3, radius: f32) -> (Vec3, Option<Vec3>) {
        let ball = Ball::new(radius);
        let mut c = center;
        let mut pushed = None;
        for _ in 0..4 {
            let Ok(Some(contact)) = query::contact(
                &Pose::from_translation(pv(c)),
                &ball,
                &Pose::identity(),
                &self.solid,
                0.0,
            ) else {
                break;
            };
            if contact.dist >= -1e-3 {
                break;
            }
            let n = (c - bv(contact.point2))
                .try_normalize()
                .unwrap_or(-bv(contact.normal1));
            c += n * (-contact.dist + SKIN * 0.5);
            pushed = Some(n);
        }
        (c, pushed)
    }

    pub fn in_kill_volume(&self, center: Vec3, radius: f32) -> bool {
        if center.y < self.lowest_point - 30.0 {
            return true;
        }
        for (c, h) in &self.kill_boxes {
            let d = (center - *c).abs();
            if d.x <= h.x && d.z <= h.z && center.y - radius <= c.y + h.y {
                return true;
            }
        }
        if let Some(kill) = &self.kill_mesh {
            let ball = Ball::new(radius * 0.8);
            if let Ok(hit) = query::intersection_test(
                &Pose::from_translation(pv(center)),
                &ball,
                &Pose::identity(),
                kill,
            ) {
                return hit.intersecting;
            }
        }
        false
    }
}

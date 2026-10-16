//! A Minecraft world for Sonic. MinecraftOSS (third_party/) generates and streams a
//! seeded vanilla 26.3 world; this crate turns it into what a host engine needs:
//! section meshes and the texture atlas to draw, sky and block light to light Sonic,
//! the day cycle, and block collision as triangles for Sonic's physics.
//!
//! Positions are Minecraft block coordinates (one block = one metre, as Sonic's own
//! units are) minus `origin`, the player spawn, so floats stay small.

pub mod setup;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::Atlas;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::HandcraftedScene;
use minecraft_terrain::sections::CullCamera;
use minecraft_terrain::terrain::{Dimension, FrameUpdate, TerrainStream};
use minecraftoss_core::registries::{DataPaths, Registries};
use minecraftoss_core::{BlockStateId, ChunkPos};

/// Block position as the terrain crate takes it.
pub type BlockPos = (i32, i32, i32);

pub use minecraft_terrain::mesh::{SectionMesh, SectionVertex};
pub use minecraft_terrain::sections::SectionPos;

/// Chunks a section mesh fades in over, as the vanilla viewer's default.
const FADE_MILLIS: u64 = 750;

pub struct McWorld {
    pub stream: TerrainStream,
    pub scene: HandcraftedScene,
    pub packs: PackStack,
    pub atlas: Arc<Atlas>,
    pub registries: Arc<Registries>,
    pub day: DayCycle,
    pub seed: i64,
    /// Block point that is position zero.
    pub origin: [f64; 3],
    light: SkyLight,
    /// Build collision triangles for loaded chunks (`collision_near`). A host that takes
    /// its collision from `section_boxes` instead turns this off.
    pub triangle_collision: bool,
    /// Collision triangles of each loaded chunk, relative to `origin`.
    chunk_triangles: HashMap<(i32, i32), Vec<[[f32; 3]; 3]>>,
    /// Boxes of each block state seen; `full` when the shape is the whole cube.
    shapes: HashMap<BlockStateId, Arc<Shape>>,
}

/// A sloped collision slab over a one-block step (section_boxes): an oriented box.
#[derive(Clone, Copy, Debug)]
pub struct Ramp {
    pub center: [f32; 3],
    pub half_extents: [f32; 3],
    /// x, y, z, w
    pub rotation: [f32; 4],
}

/// How far a ramp's slab reaches below its sloped face (into the blocks under it).
const RAMP_THICKNESS: f32 = 0.6;

struct Shape {
    boxes: Vec<[f64; 6]>,
    full: bool,
}

impl McWorld {
    /// Opens the world (Minecraft's files under `files`, from setup::ensure).
    pub fn load(files: &Path, seed: i64, view_distance: i32) -> Result<Self, String> {
        let paths = DataPaths::under(files);
        let registries = Arc::new(Registries::load(&paths)?);
        let packs = PackStack::open(vec![files.join(setup::RESOURCE_PACK)]).map_err(|e| e.to_string())?;
        let stream = TerrainStream::for_dimension(registries.clone(), seed, view_distance, Dimension::Overworld, None)
            .map_err(|e| e.to_string())?;
        let build = minecraft_terrain::mesh::build(&HandcraftedScene::default(), &packs).map_err(|e| e.to_string())?;
        let scene = HandcraftedScene::streamed(stream.states.clone());
        let (x, y, z) = stream.player_spawn;
        Ok(Self {
            stream,
            scene,
            packs,
            atlas: build.atlas,
            registries,
            day: DayCycle::default(),
            seed,
            origin: [x, y, z],
            light: SkyLight::streamed(),
            triangle_collision: true,
            chunk_triangles: HashMap::new(),
            shapes: HashMap::new(),
        })
    }

    /// A world position (relative to origin) as a block position.
    pub fn block_at(&self, p: [f32; 3]) -> BlockPos {
        let w = |i: usize| (p[i] as f64 + self.origin[i]).floor() as i32;
        (w(0), w(1), w(2))
    }

    /// The spawn chunk has generated (Sonic can be placed on it).
    pub fn spawn_ready(&self) -> bool {
        let chunk = ((self.origin[0].floor() as i32) >> 4, (self.origin[2].floor() as i32) >> 4);
        self.scene.generated_chunk(chunk).is_some()
    }

    /// One server tick around `player`: streams chunks in and out. True when the
    /// collision changed.
    pub fn tick(&mut self, player: [f32; 3]) -> bool {
        let at = self.block_at(player);
        let (loaded, forgotten) = self.stream.server_tick(at, &mut self.scene);
        let changed = !loaded.is_empty() || !forgotten.is_empty();
        for pos in forgotten {
            self.chunk_triangles.remove(&(pos.x, pos.z));
        }
        for chunk in loaded {
            if !self.triangle_collision {
                continue;
            }
            let tris = self.chunk_collision(chunk.pos);
            self.chunk_triangles.insert((chunk.pos.x, chunk.pos.z), tris);
        }
        changed
    }

    /// One render frame: finished section meshes and light from the workers.
    pub fn frame(&mut self, camera: &CullCamera) -> FrameUpdate {
        let update = self.stream.frame(&self.scene, camera, FADE_MILLIS, &self.atlas, &self.packs);
        for (chunk, column) in &update.lights {
            self.light.set_chunk_column(*chunk, column.clone());
        }
        update
    }

    /// Sky and block light levels (0..15) at a position.
    pub fn light_at(&self, p: [f32; 3]) -> (u8, u8) {
        let at = self.block_at(p);
        (self.light.get(at), self.light.get_block(at))
    }

    /// Collision triangles of the loaded chunks within `radius` chunks of `center`.
    pub fn collision_near(&self, center: [f32; 3], radius: i32) -> Vec<[[f32; 3]; 3]> {
        let at = self.block_at(center);
        let (cx, cz) = (at.0 >> 4, at.2 >> 4);
        let mut out = Vec::new();
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                if let Some(t) = self.chunk_triangles.get(&(cx + dx, cz + dz)) {
                    out.extend_from_slice(t);
                }
            }
        }
        out
    }

    /// Block y of the highest block with collision in the column of block (x, z); None
    /// until its chunk has generated.
    pub fn top_solid_y(&mut self, x: i32, z: i32) -> Option<i32> {
        let chunk = self.scene.generated_chunk((x >> 4, z >> 4)).cloned()?;
        (chunk.min_y()..chunk.min_y() + chunk.height())
            .rev()
            .find(|&y| !self.shape(chunk.block((x & 15) as usize, y, (z & 15) as usize)).boxes.is_empty())
    }

    /// The collision of chunk section (cx, sy, cz) (16 blocks a side, sy = block y >> 4)
    /// as boxes [min x, y, z, max x, y, z] relative to `origin`; None until the chunk has
    /// generated. Full cubes that can be touched (a face neighbour is not a full cube)
    /// are merged into rectangles within their layer, so a floor is a few boxes with no
    /// seams; every other shape keeps its own boxes.
    ///
    /// With them come ramps: wherever a walkable block top meets a block exactly one
    /// higher (with room above both), a 45 degree slab from the far edge of the lower
    /// block's top to the top edge of the higher one, so something that cannot step up
    /// a 1 m ledge (Sonic's character proxy) runs up it. Ramps along one line of steps
    /// are a single slab.
    pub fn section_boxes(&mut self, cx: i32, sy: i32, cz: i32) -> Option<(Vec<[f32; 6]>, Vec<Ramp>)> {
        let centre = self.scene.generated_chunk((cx, cz)).cloned()?;
        let sides: Vec<_> =
            [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().map(|(dx, dz)| self.scene.generated_chunk((cx + dx, cz + dz)).cloned()).collect();
        let y0 = sy * 16;
        let state_at = |x: i32, y: i32, z: i32| {
            let chunk = match (x, z) {
                (-1, _) => sides[0].as_ref()?,
                (16, _) => sides[1].as_ref()?,
                (_, -1) => sides[2].as_ref()?,
                (_, 16) => sides[3].as_ref()?,
                _ => &centre,
            };
            (y >= chunk.min_y() && y < chunk.min_y() + chunk.height()).then(|| chunk.block((x & 15) as usize, y, (z & 15) as usize))
        };
        // Full-cube flags of the section and the one-block shell around it.
        let at = |x: i32, y: i32, z: i32| ((y + 1) * 18 * 18 + (z + 1) * 18 + (x + 1)) as usize;
        let mut full = vec![false; 18 * 18 * 18];
        let mut out = Vec::new();
        let rel = |x: f64, y: f64, z: f64, o: &[f64; 3]| [(cx as f64 * 16.0 + x - o[0]) as f32, (y0 as f64 + y - o[1]) as f32, (cz as f64 * 16.0 + z - o[2]) as f32];
        let origin = self.origin;
        for y in -1..17 {
            for z in -1..17 {
                for x in -1..17 {
                    let outside = [x, y, z].iter().filter(|v| !(0..16).contains(*v)).count();
                    if outside > 1 {
                        continue;
                    }
                    let Some(state) = state_at(x, y0 + y, z) else { continue };
                    let shape = self.shape(state);
                    if shape.full {
                        full[at(x, y, z)] = true;
                    } else if outside == 0 {
                        for b in &shape.boxes {
                            let (lo, hi) = (rel(x as f64 + b[0], y as f64 + b[1], z as f64 + b[2], &origin), rel(x as f64 + b[3], y as f64 + b[4], z as f64 + b[5], &origin));
                            out.push([lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]]);
                        }
                    }
                }
            }
        }
        for y in 0..16 {
            let touchable = |x: i32, z: i32| {
                full[at(x, y, z)]
                    && !(full[at(x - 1, y, z)] && full[at(x + 1, y, z)] && full[at(x, y - 1, z)] && full[at(x, y + 1, z)] && full[at(x, y, z - 1)] && full[at(x, y, z + 1)])
            };
            let mut used = [[false; 16]; 16];
            for z in 0..16 {
                for x in 0..16 {
                    if used[z as usize][x as usize] || !touchable(x, z) {
                        continue;
                    }
                    let free = |x: i32, z: i32, used: &[[bool; 16]; 16]| x < 16 && z < 16 && !used[z as usize][x as usize] && touchable(x, z);
                    let mut w = 1;
                    while free(x + w, z, &used) {
                        w += 1;
                    }
                    let mut d = 1;
                    while (0..w).all(|i| free(x + i, z + d, &used)) {
                        d += 1;
                    }
                    for dz in 0..d {
                        for dx in 0..w {
                            used[(z + dz) as usize][(x + dx) as usize] = true;
                        }
                    }
                    let (lo, hi) = (rel(x as f64, y as f64, z as f64, &origin), rel((x + w) as f64, y as f64 + 1.0, (z + d) as f64, &origin));
                    out.push([lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]]);
                }
            }
        }
        // Ramps. A step at (x, y, z) toward (dx, dz): this block's top is free, the
        // neighbour one up is a full cube, and that one's top is free too.
        let mut ramps = Vec::new();
        let filled = |x: i32, y: i32, z: i32| y <= 16 && full[at(x, y, z)];
        for y in 0..16 {
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let step = |x: i32, z: i32| {
                    filled(x, y, z) && !filled(x, y + 1, z) && filled(x + dx, y + 1, z + dz) && !filled(x + dx, y + 2, z + dz)
                };
                // Runs along the line of the step (across the direction of travel).
                for a in 0..16 {
                    let cell = |b: i32| if dx != 0 { (a, b) } else { (b, a) };
                    let mut b = 0;
                    while b < 16 {
                        let (x, z) = cell(b);
                        if !step(x, z) {
                            b += 1;
                            continue;
                        }
                        let start = b;
                        while b < 16 && step(cell(b).0, cell(b).1) {
                            b += 1;
                        }
                        let len = (b - start) as f64;
                        let across = start as f64 + len * 0.5;
                        // Middle of the sloped face: half a block along and up from the lower top.
                        let (mx, mz) = if dx != 0 { (x as f64 + 0.5, across) } else { (across, z as f64 + 0.5) };
                        let along = glam::Vec3::new(dx as f32, 1.0, dz as f32).normalize();
                        let normal = glam::Vec3::new(-dx as f32, 1.0, -dz as f32).normalize();
                        let rotation = glam::Quat::from_mat3(&glam::Mat3::from_cols(along, normal, along.cross(normal)));
                        let face = rel(mx, y as f64 + 1.5, mz, &origin);
                        let center = glam::Vec3::from(face) - normal * (RAMP_THICKNESS * 0.5);
                        ramps.push(Ramp {
                            center: center.to_array(),
                            half_extents: [std::f32::consts::FRAC_1_SQRT_2, RAMP_THICKNESS * 0.5, len as f32 * 0.5],
                            rotation: rotation.to_array(),
                        });
                    }
                }
            }
        }
        Some((out, ramps))
    }

    fn shape(&mut self, state: BlockStateId) -> Arc<Shape> {
        let blocks = &self.registries.blocks;
        self.shapes
            .entry(state)
            .or_insert_with(|| {
                let boxes = blocks.collision_boxes(state);
                let full = boxes.len() == 1 && boxes[0] == [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
                Arc::new(Shape { boxes, full })
            })
            .clone()
    }

    fn full_at(&mut self, x: i32, y: i32, z: i32) -> bool {
        let Some(chunk) = self.scene.generated_chunk((x >> 4, z >> 4)).cloned() else {
            return false;
        };
        if y < chunk.min_y() || y >= chunk.min_y() + chunk.height() {
            return false;
        }
        let state = chunk.block((x & 15) as usize, y, (z & 15) as usize);
        self.shape(state).full
    }

    /// Box faces of a chunk's blocks; faces shared by two full cubes are left out.
    fn chunk_collision(&mut self, pos: ChunkPos) -> Vec<[[f32; 3]; 3]> {
        let Some(chunk) = self.scene.generated_chunk((pos.x, pos.z)).cloned() else { return Vec::new() };
        let (min_y, height) = (chunk.min_y(), chunk.height());
        let mut tris = Vec::new();
        for y in min_y..min_y + height {
            for z in 0..16 {
                for x in 0..16 {
                    let state = chunk.block(x, y, z);
                    let shape = self.shape(state);
                    if shape.boxes.is_empty() {
                        continue;
                    }
                    let (wx, wz) = (pos.x * 16 + x as i32, pos.z * 16 + z as i32);
                    for b in &shape.boxes {
                        // Face order: -x +x -y +y -z +z; a full cube's face toward another
                        // full cube cannot be touched.
                        let neighbours = [(-1, 0, 0), (1, 0, 0), (0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1)];
                        for (face, (nx, ny, nz)) in neighbours.into_iter().enumerate() {
                            if shape.full && self.full_at(wx + nx, y + ny, wz + nz) {
                                continue;
                            }
                            let base = [wx as f64, y as f64, wz as f64];
                            push_face(&mut tris, b, face, base, self.origin);
                        }
                    }
                }
            }
        }
        tris
    }
}

/// Two triangles of one face of box `b` at block `base`, wound outward.
fn push_face(out: &mut Vec<[[f32; 3]; 3]>, b: &[f64; 6], face: usize, base: [f64; 3], origin: [f64; 3]) {
    let p = |x: f64, y: f64, z: f64| -> [f32; 3] {
        [(base[0] + x - origin[0]) as f32, (base[1] + y - origin[1]) as f32, (base[2] + z - origin[2]) as f32]
    };
    let [x0, y0, z0, x1, y1, z1] = *b;
    let q = match face {
        0 => [p(x0, y0, z0), p(x0, y0, z1), p(x0, y1, z1), p(x0, y1, z0)],
        1 => [p(x1, y0, z0), p(x1, y1, z0), p(x1, y1, z1), p(x1, y0, z1)],
        2 => [p(x0, y0, z0), p(x1, y0, z0), p(x1, y0, z1), p(x0, y0, z1)],
        3 => [p(x0, y1, z0), p(x0, y1, z1), p(x1, y1, z1), p(x1, y1, z0)],
        4 => [p(x0, y0, z0), p(x0, y1, z0), p(x1, y1, z0), p(x1, y0, z0)],
        _ => [p(x0, y0, z1), p(x1, y0, z1), p(x1, y1, z1), p(x0, y1, z1)],
    };
    out.push([q[0], q[1], q[2]]);
    out.push([q[0], q[2], q[3]]);
}

pub use minecraft_terrain::day_cycle::SkyState;
pub use minecraft_terrain::sections::CullCamera as Camera;

/// The sky and lightmap inputs for one moment of the day cycle, as plain arrays.
#[derive(Clone, Copy, Debug)]
pub struct Sky {
    pub sky: [f32; 3],
    pub fog: [f32; 3],
    /// Toward the sun.
    pub sun_direction: [f32; 3],
    pub light: [f32; 3],
    pub ambient: [f32; 3],
    pub sky_light_color: [f32; 3],
    pub sky_light_factor: f32,
    pub block_light_tint: [f32; 3],
    pub block_factor: f32,
    pub fog_start: f32,
    pub fog_end: f32,
}

impl McWorld {
    pub fn sky(&self) -> Sky {
        let s = self.day.sample();
        Sky {
            sky: s.sky.to_array(),
            fog: s.fog.to_array(),
            sun_direction: s.sun_direction.to_array(),
            light: s.light.to_array(),
            ambient: s.ambient.to_array(),
            sky_light_color: s.sky_light_color.to_array(),
            sky_light_factor: s.sky_light_factor,
            block_light_tint: s.block_light_tint.to_array(),
            block_factor: s.block_factor,
            fog_start: s.fog_start,
            fog_end: s.fog_end,
        }
    }

    /// Section culling camera from a position relative to origin.
    pub fn cull_camera(&self, position: [f32; 3], forward: [f32; 3], fov_degrees: f32, aspect: f32) -> CullCamera {
        let f = glam::Vec3::from_array(forward).normalize_or(glam::Vec3::Z);
        CullCamera {
            position: glam::DVec3::new(
                position[0] as f64 + self.origin[0],
                position[1] as f64 + self.origin[1],
                position[2] as f64 + self.origin[2],
            ),
            forward: f,
            fov_degrees,
            aspect,
            // Minecraft yaw: 0 facing +Z, turning toward -X.
            yaw_degrees: (-f.x).atan2(f.z).to_degrees(),
            pitch_degrees: (-f.y).asin().to_degrees(),
        }
    }
}

/// Vanilla's lightmap for sky / block light levels (0..15), the formula the terrain
/// shader uses: brightness(level) = f / (4 - 3f), inputs from the day cycle, then the
/// default-gamma (0.5) lerp. Used to light Sonic as Minecraft lights entities.
pub fn lightmap(sky: &Sky, sky_level: f32, block_level: f32) -> [f32; 3] {
    let brightness = |l: f32| {
        let f = (l / 15.0).clamp(0.0, 1.0);
        f / (4.0 - 3.0 * f)
    };
    let (bs, bb) = (brightness(sky_level) * sky.sky_light_factor, brightness(block_level) * sky.block_factor);
    let raw: [f32; 3] = std::array::from_fn(|i| {
        (sky.ambient[i] + sky.sky_light_color[i] * bs + sky.block_light_tint[i] * bb).clamp(0.0, 1.0)
    });
    let m = raw[0].max(raw[1]).max(raw[2]);
    std::array::from_fn(|i| {
        let gamma = if m > 0.0 { raw[i] * ((1.0 - (1.0 - m).powi(4)) / m) } else { 0.0 };
        raw[i] + (gamma - raw[i]) * 0.5
    })
}

//! Side-view (`@SV`) paths from the stage's `*.path.xml`: two parallel rails
//! whose midline Sonic is locked to in 2D sections (ChangeMode_3Dto2D).

use bevy::prelude::*;
use unleashed_assets::path::{PathDocument, parse_path_xml};

use crate::Stage;

pub struct PathsPlugin;

impl Plugin for PathsPlugin {
    fn build(&self, app: &mut App) {
        let stage = &app.world().resource::<Stage>().0;
        let mut side = Vec::new();
        for name in stage.db.names().filter(|n| n.ends_with(".path.xml") && *n != "StageGuidePath.path.xml") {
            let Some(doc) = stage.db.get(name).and_then(|b| parse_path_xml(b).ok()) else {
                warn!("{name}: unreadable path file");
                continue;
            };
            side.extend(SidePath::from_document(&doc));
        }
        info!("side-view paths: {}", side.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "));
        app.insert_resource(StagePaths { side });
    }
}

#[derive(Resource, Default)]
pub struct StagePaths {
    pub side: Vec<SidePath>,
}

impl StagePaths {
    /// The side path closest to `p` (within `max` metres) and its nearest sample.
    pub fn nearest_side(&self, p: Vec3, max: f32) -> Option<(usize, usize)> {
        self.side
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let k = path.nearest(p, usize::MAX);
                (i, k, path.center[k].distance(p))
            })
            .filter(|(_, _, d)| *d < max)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(i, k, _)| (i, k))
    }
}

pub struct SidePath {
    pub name: String,
    /// Midline between the two rails.
    pub center: Vec<Vec3>,
    /// Unit vector from rail 0 to rail 1 at each sample.
    pub side: Vec<Vec3>,
}

impl SidePath {
    fn from_document(doc: &PathDocument) -> Vec<Self> {
        let mut out = Vec::new();
        for node in doc.nodes.iter().filter(|n| n.name.ends_with("@SV")) {
            let Some(geometry) = doc.geometry(node.geometry_id()) else { continue };
            let [a, b] = [0, 1].map(|i| geometry.splines.get(i));
            let (Some(a), Some(b)) = (a, b) else { continue };
            if a.knots.len() != b.knots.len() || a.knots.len() < 2 {
                continue;
            }
            let rotation = Quat::from_array(node.rotate).normalize();
            let scale = Vec3::from(node.scale);
            let translate = Vec3::from(node.translate);
            let to_world = |p: [f32; 3]| translate + rotation * (Vec3::from(p) * scale);
            let bezier = |k0: &unleashed_assets::path::PathKnot, k1: &unleashed_assets::path::PathKnot, t: f32| {
                let (p0, p1, p2, p3) = (to_world(k0.point), to_world(k0.out_vec), to_world(k1.in_vec), to_world(k1.point));
                let u = 1.0 - t;
                p0 * u * u * u + p1 * 3.0 * u * u * t + p2 * 3.0 * u * t * t + p3 * t * t * t
            };
            let (mut center, mut side) = (Vec::new(), Vec::new());
            for i in 0..a.knots.len() - 1 {
                // Same parameter steps on both rails keeps the samples paired.
                let length = to_world(a.knots[i].point).distance(to_world(a.knots[i + 1].point));
                let steps = (length / 1.0).ceil().max(1.0) as usize;
                let last = i == a.knots.len() - 2;
                for s in 0..steps + usize::from(last) {
                    let t = s as f32 / steps as f32;
                    let (pa, pb) = (bezier(&a.knots[i], &a.knots[i + 1], t), bezier(&b.knots[i], &b.knots[i + 1], t));
                    center.push((pa + pb) * 0.5);
                    side.push((pb - pa).normalize_or(Vec3::X));
                }
            }
            out.push(Self { name: node.name.clone(), center, side });
        }
        out
    }

    /// Closest sample, searching around `hint` (`usize::MAX` = everywhere).
    pub fn nearest(&self, p: Vec3, hint: usize) -> usize {
        let range = if hint == usize::MAX {
            0..self.center.len()
        } else {
            hint.saturating_sub(10)..(hint + 30).min(self.center.len())
        };
        range
            .min_by(|a, b| self.center[*a].distance_squared(p).total_cmp(&self.center[*b].distance_squared(p)))
            .unwrap_or(0)
    }

    /// (midline point, forward tangent, side) at sample `i`.
    pub fn frame(&self, i: usize) -> (Vec3, Vec3, Vec3) {
        let n = self.center.len();
        let (lo, hi) = (i.saturating_sub(1), (i + 1).min(n - 1));
        let tangent = (self.center[hi] - self.center[lo]).normalize_or(Vec3::NEG_Z);
        (self.center[i], tangent, self.side[i])
    }
}

//! The stage guide path (`StageGuidePath.path.xml`): the designer-authored
//! spline from start to goal, sampled into a polyline.

use bevy::prelude::*;
use unleashed_assets::path::{PathDocument, parse_path_xml};

use crate::Stage;

pub struct GuidePlugin;

impl Plugin for GuidePlugin {
    fn build(&self, app: &mut App) {
        let stage = &app.world().resource::<Stage>().0;
        let guide = stage
            .db
            .get("StageGuidePath.path.xml")
            .and_then(|bytes| parse_path_xml(bytes).ok())
            .map(|doc| GuidePath::from_document(&doc))
            .unwrap_or_default();
        info!("guide path: {} samples", guide.points.len());
        app.insert_resource(guide);
    }
}

#[derive(Resource, Default)]
pub struct GuidePath {
    pub points: Vec<Vec3>,
}

impl GuidePath {
    fn from_document(doc: &PathDocument) -> Self {
        let mut points = Vec::new();
        for node in &doc.nodes {
            let Some(geometry) = doc.geometry(node.geometry_id()) else {
                continue;
            };
            let rotation = Quat::from_array(node.rotate).normalize();
            let scale = Vec3::from(node.scale);
            let translate = Vec3::from(node.translate);
            let to_world = |p: [f32; 3]| translate + rotation * (Vec3::from(p) * scale);
            for spline in &geometry.splines {
                for pair in spline.knots.windows(2) {
                    let (a, b) = (&pair[0], &pair[1]);
                    let (p0, p1, p2, p3) = (
                        to_world(a.point),
                        to_world(a.out_vec),
                        to_world(b.in_vec),
                        to_world(b.point),
                    );
                    let length = p0.distance(p3).max(1.0);
                    let steps = (length / 2.0).ceil() as usize;
                    for i in 0..steps {
                        let t = i as f32 / steps as f32;
                        let u = 1.0 - t;
                        points.push(
                            p0 * u * u * u + p1 * 3.0 * u * u * t + p2 * 3.0 * u * t * t + p3 * t * t * t,
                        );
                    }
                }
                if let Some(last) = spline.knots.last() {
                    points.push(to_world(last.point));
                }
            }
        }
        Self { points }
    }

    /// Index of the closest sample, searching a window around `hint`.
    pub fn nearest(&self, p: Vec3, hint: usize) -> usize {
        let lo = hint.saturating_sub(20);
        let hi = (hint + 60).min(self.points.len());
        let window = if hint == usize::MAX { 0..self.points.len() } else { lo..hi };
        window
            .min_by(|a, b| {
                self.points[*a]
                    .distance_squared(p)
                    .total_cmp(&self.points[*b].distance_squared(p))
            })
            .unwrap_or(0)
    }
}

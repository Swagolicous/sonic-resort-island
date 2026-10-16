//! usage: boxes_near x z r — Havok box bodies whose footprint is within r of (x, z).
use anyhow::Result;
use unleashed_assets::{collision::havok::CollisionShape, stage::day::load_day_stage};
fn main() -> Result<()> {
    let a: Vec<f32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    for body in &stage.collision {
        if let CollisionShape::Box { center, half_extents } = &body.shape {
            let dx = ((center[0] - a[0]).abs() - half_extents[0]).max(0.0);
            let dz = ((center[2] - a[1]).abs() - half_extents[2]).max(0.0);
            if dx.hypot(dz) < a[2] {
                println!("{} surface={:?} kill={} center={:?} half={:?} top={:.2}", body.name, body.surface_type,
                    body.is_kill_volume(), center, half_extents, center[1] + half_extents[1]);
            }
        }
    }
    Ok(())
}

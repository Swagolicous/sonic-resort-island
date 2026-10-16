//! usage: col_near x y z r — collision triangles within r of a point (body, normal, height span)
use anyhow::Result;
use unleashed_assets::{collision::havok::CollisionShape, stage::day::load_day_stage};
fn main() -> Result<()> {
    let a: Vec<f32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    for b in &stage.collision {
        let CollisionShape::Mesh { vertices, triangles } = &b.shape else { continue };
        for t in triangles {
            let p = t.map(|i| vertices[i as usize]);
            let c = [0, 1, 2].map(|k| (p[0][k] + p[1][k] + p[2][k]) / 3.0);
            let d = ((c[0] - a[0]).powi(2) + (c[1] - a[1]).powi(2) + (c[2] - a[2]).powi(2)).sqrt();
            if d > a[3] { continue }
            let e1 = [0, 1, 2].map(|k| p[1][k] - p[0][k]);
            let e2 = [0, 1, 2].map(|k| p[2][k] - p[0][k]);
            let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
            let ys = p.map(|v| v[1]);
            println!("{:36} d={d:5.2} n=({:5.2},{:5.2},{:5.2}) y={:.2}..{:.2} z={:.2}..{:.2}", b.name, n[0] / l, n[1] / l, n[2] / l,
                ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max),
                p.iter().map(|v| v[2]).fold(f32::MAX, f32::min), p.iter().map(|v| v[2]).fold(f32::MIN, f32::max));
        }
    }
    Ok(())
}

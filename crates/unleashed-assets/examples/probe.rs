//! Lists terrain instances/materials with geometry within `r` of a point.
//! usage: probe x y z r
use anyhow::Result;
use std::collections::BTreeSet;
use unleashed_assets::{mirage::material::*, stage::day::load_day_stage};

fn main() -> Result<()> {
    let a: Vec<f32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let (p, r) = ([a[0], a[1], a[2]], a[3]);
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    let mut seen = BTreeSet::new();
    for inst in &stage.instances {
        let Some(model) = stage.models.get(&inst.model) else { continue };
        let m = inst.matrix;
        for s in &model.submeshes {
            let hit = s.positions.iter().any(|v| {
                let w = [0, 1, 2].map(|i| m[i * 4] * v[0] + m[i * 4 + 1] * v[1] + m[i * 4 + 2] * v[2] + m[i * 4 + 3]);
                (w[0] - p[0]).powi(2) + (w[1] - p[1]).powi(2) + (w[2] - p[2]).powi(2) < r * r
            });
            if hit && seen.insert((inst.model.clone(), s.material.clone())) {
                let mat = stage.db.get(&format!("{}.material", s.material)).map(parse_material);
                let tex = match &mat {
                    Some(Ok(mt)) => stage.db.get(&format!("{}.texset", mt.texset)).and_then(|b| parse_texset(b).ok()).unwrap_or_default()
                        .iter().filter_map(|t| stage.db.get(&format!("{t}.texture")).and_then(|b| parse_texture(b).ok()))
                        .map(|t| format!("{}@{}={}", t.kind, t.texcoord, t.dds)).collect::<Vec<_>>(),
                    _ => vec![],
                };
                let shader = mat.and_then(|m| m.ok()).map(|m| m.shader).unwrap_or_default();
                println!("{} | {} | {:?} | {shader} | {:?} | colors={}", inst.model, s.material, s.slot, tex, s.colors.first().map(|c| format!("{c:?}")).unwrap_or_default());
            }
        }
    }
    Ok(())
}

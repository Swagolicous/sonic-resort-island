//! Flags terrain submeshes whose TEXCOORD0 is missing or degenerate.
use anyhow::Result;
use std::collections::BTreeMap;
use unleashed_assets::stage::day::load_day_stage;

fn main() -> Result<()> {
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    let mut by_material: BTreeMap<String, (usize, usize, f32, String)> = BTreeMap::new();
    for (name, model) in &stage.models {
        for s in &model.submeshes {
            let e = by_material.entry(s.material.clone()).or_insert((0, 0, f32::MAX, String::new()));
            e.0 += 1;
            let uv = &s.uvs[0];
            let ext = if uv.is_empty() { 0.0 } else {
                let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                for t in uv { for k in 0..2 { lo[k] = lo[k].min(t[k]); hi[k] = hi[k].max(t[k]); } }
                (hi[0] - lo[0]).max(hi[1] - lo[1])
            };
            if ext < 0.05 { e.1 += 1; e.3 = name.clone(); }
            e.2 = e.2.min(ext);
        }
    }
    for (m, (n, degenerate, min_ext, example)) in by_material {
        if degenerate > 0 { println!("{m}: {degenerate}/{n} degenerate uv0 (min extent {min_ext}) e.g. {example}"); }
    }
    Ok(())
}

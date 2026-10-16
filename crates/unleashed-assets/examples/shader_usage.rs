//! Counts terrain submeshes per material shader name (Windmill Isle Act 1).
use anyhow::Result;
use std::collections::BTreeMap;
use unleashed_assets::{mirage::material::parse_material, stage::day::load_day_stage};
fn main() -> Result<()> {
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for model in stage.models.values() {
        for s in &model.submeshes {
            let shader = stage.db.get(&format!("{}.material", s.material))
                .and_then(|b| parse_material(b).ok()).map(|m| m.shader).unwrap_or_else(|| "?".into());
            *count.entry(shader).or_default() += 1;
        }
    }
    let mut v: Vec<_> = count.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    for (s, n) in v { println!("{n:5} {s}"); }
    Ok(())
}

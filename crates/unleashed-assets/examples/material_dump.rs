//! Dumps params and texture slots of the first material using each shader (or SHADER env).
use anyhow::Result;
use std::collections::BTreeSet;
use unleashed_assets::{mirage::material::{parse_material, parse_texset, parse_texture}, stage::day::load_day_stage};
fn main() -> Result<()> {
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    let mut seen = BTreeSet::new();
    let want = std::env::var("SHADER").ok();
    for model in stage.models.values() {
        for s in &model.submeshes {
            let Some(Ok(m)) = stage.db.get(&format!("{}.material", s.material)).map(parse_material) else { continue };
            if want.as_deref().is_some_and(|w| w != m.shader) || !seen.insert(m.shader.clone()) { continue }
            println!("== {} ({}) alpha_threshold={} double_sided={} additive={}", m.shader, s.material, m.alpha_threshold, m.double_sided, m.additive);
            for p in &m.params { println!("   param {} = {:?}", p.name, p.values); }
            if let Some(Ok(names)) = stage.db.get(&format!("{}.texset", m.texset)).map(parse_texset) {
                for n in names {
                    if let Some(Ok(t)) = stage.db.get(&format!("{n}.texture")).map(parse_texture) {
                        println!("   tex {:10} uv{} {}", t.kind, t.texcoord, t.dds);
                    }
                }
            }
        }
    }
    Ok(())
}

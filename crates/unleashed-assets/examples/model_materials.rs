//! Lists submesh materials, shaders and texture slots for one model (MODEL env, from the stage db).
use anyhow::Result;
use unleashed_assets::{mirage::{material::{parse_material, parse_texset, parse_texture}, mesh::parse_model}, stage::day::load_day_stage};
fn main() -> Result<()> {
    let common: Vec<&str> = vec!["SystemCommonCore","SystemCommon","ActionCommon","#ActionCommon","SonicActionCommon","#SonicActionCommon","SonicActionCommonGeneral","#SonicActionCommonGeneral","Sonic","#Sonic"];
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &common)?;
    for name in std::env::args().skip(1) {
        let model = parse_model(stage.db.get(&format!("{name}.model")).expect("model"))?;
        println!("== {name}");
        let mut seen = std::collections::BTreeSet::new();
        for s in &model.submeshes {
            if !seen.insert(s.material.clone()) { continue }
            let Some(Ok(m)) = stage.db.get(&format!("{}.material", s.material)).map(parse_material) else { println!("  {} (missing)", s.material); continue };
            let tex: Vec<String> = stage.db.get(&format!("{}.texset", m.texset)).map(parse_texset).and_then(Result::ok).unwrap_or_default().iter()
                .filter_map(|n| stage.db.get(&format!("{n}.texture")).map(parse_texture).and_then(Result::ok)).map(|t| format!("{}:{}", t.kind, t.dds)).collect();
            println!("  {:40} {:24} {:?}", s.material, m.shader, tex);
            for p in &m.params { if p.name != "emissive" { print!("    {}={:?}", p.name, p.values.first()); } }
            println!();
        }
    }
    Ok(())
}

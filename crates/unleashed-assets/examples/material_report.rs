//! Lists materials used by stage terrain/objects with their texture bindings.
use anyhow::Result;
use std::collections::BTreeMap;
use unleashed_assets::{mirage::material::*, mirage::mesh::parse_model, stage::day::load_day_stage};

fn main() -> Result<()> {
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &["SonicActionCommonGeneral", "EvilActionCommonGeneral"])?;
    let mut used = BTreeMap::new();
    for m in stage.models.values() { for s in &m.submeshes { *used.entry(s.material.clone()).or_insert(0) += 1; } }
    for extra in ["cmn_obj_ms_ringnormalM_000.model", "cmn_obj_kt3_springredM_000.model", "cmn_obj_ms_superringM_000.model"] {
        match parse_model(stage.db.get(extra).unwrap()) {
            Ok(m) => for s in &m.submeshes { *used.entry(s.material.clone()).or_insert(0) += 1000; },
            Err(e) => println!("{extra}: {e:#}"),
        }
    }
    let filter = std::env::args().nth(1).unwrap_or_default();
    for (name, n) in used {
        let Some(Ok(mat)) = stage.db.get(&format!("{name}.material")).map(parse_material) else { println!("{name}: NO MATERIAL"); continue };
        let mut tex = Vec::new();
        if let Some(Ok(set)) = stage.db.get(&format!("{}.texset", mat.texset)).map(parse_texset) {
            for t in set { if let Some(Ok(r)) = stage.db.get(&format!("{t}.texture")).map(parse_texture) { tex.push(format!("{}@{}={}", r.kind, r.texcoord, r.dds)); } else { tex.push(format!("?{t}")) } }
        } else { tex.push(format!("NO TEXSET {}", mat.texset)); }
        let has_d0 = tex.iter().any(|t| t.starts_with("diffuse@0"));
        if filter.is_empty() || (filter == "nod0" && !has_d0) || name.contains(&filter) || mat.shader.contains(&filter) {
            println!("{name} x{n} shader={} {:?}", mat.shader, tex);
        }
    }
    Ok(())
}

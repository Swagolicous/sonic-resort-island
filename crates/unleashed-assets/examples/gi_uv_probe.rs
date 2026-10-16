//! Per UV channel: how many terrain submeshes have it, and how many stay inside [0,1].
use anyhow::Result;
use unleashed_assets::stage::day::load_day_stage;
fn main() -> Result<()> {
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    let mut stats = [[0usize; 2]; 4];
    for model in stage.models.values() {
        for s in &model.submeshes {
            for (c, uv) in s.uvs.iter().enumerate().take(4) {
                if uv.is_empty() { continue }
                stats[c][0] += 1;
                if uv.iter().all(|t| t[0] >= -0.01 && t[0] <= 1.01 && t[1] >= -0.01 && t[1] <= 1.01) { stats[c][1] += 1; }
            }
        }
    }
    for (c, [n, inside]) in stats.iter().enumerate() { println!("TEXCOORD{c}: {n} submeshes, {inside} within [0,1]"); }
    Ok(())
}

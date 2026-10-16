//! Loads the stage's GI atlases and checks every terrain instance has a tile.
use anyhow::Result;
use unleashed_assets::{gi::load_gi_atlases, stage::day::load_day_stage};
fn main() -> Result<()> {
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    let gi = load_gi_atlases(&stage.root.join("game/Packed/ActD_MykonosAct1/Stage.pfd"))?;
    let missing: Vec<_> = stage.instances.iter().filter(|i| !gi.entries.contains_key(&i.name)).map(|i| i.name.clone()).collect();
    println!("{} atlases, {} entries, {} instances, {} without GI", gi.textures.len(), gi.entries.len(), stage.instances.len(), missing.len());
    for m in missing.iter().take(10) { println!("  missing {m}"); }
    if let Some(i) = stage.instances.first() { println!("{} -> {:?}", i.name, gi.entries.get(&i.name)); }
    Ok(())
}

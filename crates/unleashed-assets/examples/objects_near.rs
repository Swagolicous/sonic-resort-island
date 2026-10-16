//! usage: objects_near x y z r  — lists set objects near a point.
use anyhow::Result;
use unleashed_assets::stage::day::load_day_stage;
fn main() -> Result<()> {
    let a: Vec<f32> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    for o in &stage.objects {
        let d = ((o.position[0]-a[0]).powi(2)+(o.position[1]-a[1]).powi(2)+(o.position[2]-a[2]).powi(2)).sqrt();
        let want = std::env::var("KIND").ok(); if (d < a[3] || want.as_deref().is_some_and(|k| o.kind.contains(k))) && !o.kind.starts_with("ObjCamera") && o.kind != "Ring" && want.as_deref().is_none_or(|k| o.kind.contains(k)) {
            let mut keys: Vec<_> = o.params.iter().filter(|(k,_)| !k.contains('/') && *k != "SetObjectID").map(|(k,v)| format!("{k}={v}")).collect();
            keys.sort();
            println!("{:>6.1}m {} #{} pos={:?} rot={:?} {}", d, o.kind, o.id, o.position, o.rotation, keys.join(" "));
        }
    }
    Ok(())
}

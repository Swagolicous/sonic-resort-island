//! Decodes Sonic's skeleton + an animation and compares with the model bones.
use anyhow::Result;
use unleashed_assets::{animation::*, mirage::mesh::parse_model, resources::ResourceDb};

fn main() -> Result<()> {
    let mut db = ResourceDb::new();
    db.load_arl("../game".as_ref(), "Sonic")?;
    let skel = parse_skeleton(db.get("SonicRoot.skl.hkx").unwrap())?;
    let model = parse_model(db.get("SonicRoot.model").unwrap())?;
    println!("skeleton {} bones, model {} bones", skel.bones.len(), model.bones.len());
    let same = skel.bones.iter().zip(&model.bones).filter(|(a, b)| **a == b.name).count();
    println!("name/order matches: {same}");
    for i in [0usize, 4, 8, 53] {
        let b = &model.bones[i];
        println!("model bone {i} {} parent {} m={:?}", b.name, b.parent, b.matrix.map(|v| (v * 1000.0).round() / 1000.0));
    }
    let (mut ok, mut bad) = (0, 0);
    let mut names: Vec<&str> = db.names().filter(|n| n.ends_with(".anm.hkx")).collect();
    names.sort();
    for n in &names {
        match parse_animation(db.get(n).unwrap()) {
            Ok(a) => { ok += 1; let mut out = Vec::new(); a.sample(a.duration * 0.5, &mut out); assert_eq!(out.len(), a.track_count()); }
            Err(e) => { bad += 1; println!("{n}: {e:#}"); }
        }
    }
    println!("animations ok {ok} failed {bad}");
    let a = parse_animation(db.get("sonic_dash.anm.hkx").unwrap())?;
    let mut out = Vec::new();
    for t in [0.0, 0.3, 0.6] {
        a.sample(t, &mut out);
        println!("t={t} hips t={:?} r={:?} | head r={:?}", out[4].translation, out[4].rotation, out[8].rotation);
    }
    Ok(())
}

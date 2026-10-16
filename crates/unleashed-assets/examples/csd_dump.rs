//! usage: csd_dump <archive stem> <file.yncp>
use anyhow::Result;
use unleashed_assets::{csd::parse_project, resources::ResourceDb};
fn main() -> Result<()> {
    let mut a = std::env::args().skip(1);
    let (stem, file) = (a.next().unwrap(), a.next().unwrap());
    let mut db = ResourceDb::new();
    db.load_arl("../game".as_ref(), &stem)?;
    let p = parse_project(db.get(&file).unwrap())?;
    println!("project {} textures {:?}", p.name, p.textures);
    println!("fonts {:?}", p.fonts.keys().collect::<Vec<_>>());
    for s in &p.scenes {
        println!("scene {:24} z={:4} casts={:3} subimages={:3} anims={:?}", s.name, s.z_index, s.casts.len(), s.sub_images.len(),
            s.animations.iter().map(|a| format!("{}[{}-{}]/{}tracks", a.name, a.start, a.end, a.tracks.len())).collect::<Vec<_>>());
    }
    if let Ok(scene) = std::env::var("SCENE") {
        let s = p.scene(&scene).unwrap();
        for (i, c) in s.casts.iter().enumerate() {
            println!("{i:3} {:16} parent={:?} kind={:?} flags={:08x}/{:08x}/{:08x} t=({:.4},{:.4}) s=({:.3},{:.3}) size={:?} subs={:?} color={:08x} hide={} text={:?}",
                c.name, c.parent, c.kind, c.flags[0], c.flags[1], c.flags[2], c.info.translation[0], c.info.translation[1], c.info.scale[0], c.info.scale[1], c.size, &c.sub_images[..c.sub_images.len().min(3)], c.info.color, c.info.hide, c.text);
        }
    }
    if let (Ok(scene), Ok(anim)) = (std::env::var("SCENE"), std::env::var("ANIM")) {
        let s = p.scene(&scene).unwrap();
        let a = s.animation(&anim).unwrap();
        for t in &a.tracks {
            println!("  {:20} {:?} {:?}", s.casts[t.cast].name, t.property, t.keys.iter().map(|k| (k.frame, (k.value * 1000.0).round() / 1000.0)).collect::<Vec<_>>());
        }
    }
    Ok(())
}
#[allow(dead_code)]
fn unused() {}

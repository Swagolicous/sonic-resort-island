//! Parses every .model / .terrain-model in the given archives and reports failures.
use anyhow::Result;
use unleashed_assets::{mirage::mesh::{parse_model, parse_terrain_model}, resources::ResourceDb};

fn main() -> Result<()> {
    let game = std::path::Path::new("../game");
    let mut db = ResourceDb::new();
    for stem in std::env::args().skip(1) {
        db.load_arl(game, &stem)?;
    }
    let (mut ok, mut bad) = (0, 0);
    for name in db.names() {
        let r = if name.ends_with(".terrain-model") {
            parse_terrain_model(db.get(name).unwrap()).map(|_| ())
        } else if name.ends_with(".model") {
            parse_model(db.get(name).unwrap()).map(|_| ())
        } else {
            continue;
        };
        match r {
            Ok(()) => ok += 1,
            Err(e) => { bad += 1; if bad <= 10 { println!("{name}: {e:#}"); } }
        }
    }
    println!("ok {ok} failed {bad}");
    Ok(())
}

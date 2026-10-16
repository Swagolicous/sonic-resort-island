//! Dumps files of a day stage's resource database: `stagefile <root> <stage> <name|*pattern*>...`
//! (common archives as windmill-isle loads them; a `*x*` pattern lists matching names).
use unleashed_assets::stage::day::load_day_stage;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let common = ["SystemCommonCore", "SystemCommon", "ActionCommon", "#ActionCommon", "SonicActionCommon",
        "#SonicActionCommon", "EvilActionCommonGeneral", "SonicActionCommonGeneral", "#SonicActionCommonGeneral", "Sonic", "#Sonic"];
    let stage = load_day_stage(a[0].as_ref(), &a[1], &common).unwrap();
    for name in &a[2..] {
        if let Some(p) = name.strip_prefix('*').and_then(|n| n.strip_suffix('*')) {
            let mut names: Vec<&str> = stage.db.names().filter(|n| n.contains(p)).collect();
            names.sort();
            for n in names { println!("{n}"); }
        } else if let Some((name, out)) = name.split_once('>') {
            if let Some(b) = stage.db.get(name) {
                std::fs::write(out, b).unwrap();
                println!("wrote {out} ({} bytes)", b.len());
            }
        } else if let Some(b) = stage.db.get(name) {
            println!("{}", String::from_utf8_lossy(b));
        }
    }
}

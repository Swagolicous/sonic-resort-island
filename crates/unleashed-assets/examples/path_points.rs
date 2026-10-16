//! Prints world-space knots of a named path in a stage path file.
//! usage: path_points <file in db> <name substring>
use anyhow::Result;
use unleashed_assets::{path::parse_path_xml, stage::day::load_day_stage};
fn main() -> Result<()> {
    let mut a = std::env::args().skip(1);
    let (file, want) = (a.next().unwrap(), a.next().unwrap_or_default());
    let stage = load_day_stage("..".as_ref(), "ActD_MykonosAct1", &[])?;
    let doc = parse_path_xml(stage.db.get(&file).unwrap())?;
    for node in &doc.nodes {
        if !node.name.contains(&want) { continue }
        let g = doc.geometry(node.geometry_id()).unwrap();
        println!("{} width={} splines={}", node.name, g.width, g.splines.len());
        for (si, s) in g.splines.iter().enumerate() {
            for k in s.knots.iter().step_by((s.knots.len() / 12).max(1)) {
                let p = [0, 1, 2].map(|i| k.point[i] + node.translate[i]);
                println!("  spline{si} ({:8.2},{:8.2},{:8.2})", p[0], p[1], p[2]);
            }
        }
    }
    Ok(())
}

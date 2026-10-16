//! Searches every ARL name table under a directory tree for resource names.
//! usage: find_resource <dir> <substring>...
use anyhow::Result;
use unleashed_assets::{ar, resources::read_maybe_compressed};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args.next().unwrap();
    let needles: Vec<String> = args.collect();
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let p = e?.path();
            if p.is_dir() { stack.push(p); continue; }
            if p.extension().is_some_and(|x| x == "arl") {
                let Ok(bytes) = read_maybe_compressed(&p) else { continue };
                let Ok(list) = ar::parse_archive_list(&bytes) else { continue };
                for n in &list.names {
                    if needles.iter().any(|k| n.contains(k.as_str())) {
                        println!("{} : {n}", p.display());
                    }
                }
            }
        }
    }
    Ok(())
}

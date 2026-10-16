//! How often does the game's animation move Sonic's skinning matrices? Holds the stick
//! forward and, per frame, prints '#' when the animated 111-matrix run changed and '.'
//! when it did not, plus the snapshot's frame time.
//!
//! usage: animrate <82346F08_pre.bin> [frames]
use unleashed_guest_host::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: animrate <82346F08_pre.bin> [frames]");
    let frames: u32 = std::env::args().nth(2).map_or(120, |s| s.parse().unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    println!("snapshot dt {} ({:.1} fps)", g.snapshot_dt(), 1.0 / g.snapshot_dt());
    let pad = Pad { left: [0.0, 1.0], ..Default::default() };
    for _ in 0..30 {
        g.step(&pad);
    }
    let runs = g.animated_matrix_runs(111);
    println!("animated 111-matrix runs: {runs:08X?}");
    for &run in &runs {
        let mut last = g.matrices(run, 111);
        let (mut line, mut changed) = (String::new(), 0);
        for _ in 0..frames {
            g.step(&pad);
            let now = g.matrices(run, 111);
            let moved = now != last;
            changed += moved as u32;
            line.push(if moved { '#' } else { '.' });
            last = now;
        }
        println!("{run:08X}: {line}\n   changed on {changed} of {frames} frames");
    }
}

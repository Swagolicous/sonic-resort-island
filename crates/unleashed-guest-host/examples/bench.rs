//! Time per game frame of the lifted update (player + camera).
use unleashed_guest_host::*;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let pad = Pad { left: [0.0, 1.0], ..Default::default() };
    for _ in 0..30 { g.step(&pad); }
    let mut times: Vec<f64> = (0..300).map(|_| { let t = std::time::Instant::now(); g.step(&pad); t.elapsed().as_secs_f64() * 1000.0 }).collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("frame ms: median {:.2} p90 {:.2} max {:.2}", times[150], times[270], times[299]);
    xenon_rt::trace_indirect(true);
    g.step(&pad);
    let calls = xenon_rt::trace_indirect(false).unwrap();
    println!("indirect calls per frame: {} (distinct {})", calls.values().sum::<u32>(), calls.len());
}

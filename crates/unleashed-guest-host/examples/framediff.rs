//! Steps a few frames, then lists the guest memory ranges one more frame changes
//! (merged across 64-byte gaps), largest first: where per-frame state lives.
use unleashed_guest_host::*;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let pad = Pad { left: [0.0, 1.0], ..Default::default() };
    for _ in 0..20 { g.step(&pad); }
    let b = g.base();
    let lo = 0x0002_0000usize;
    let hi = 0x1000_0000usize;
    let before: Vec<u8> = unsafe { std::slice::from_raw_parts(b.add(lo), hi - lo) }.to_vec();
    g.step(&pad);
    let after = unsafe { std::slice::from_raw_parts(b.add(lo), hi - lo) };
    let mut ranges: Vec<(usize, usize, usize)> = Vec::new();
    let mut i = 0;
    while i < after.len() {
        if after[i] != before[i] {
            let a = i & !3;
            match ranges.last_mut() {
                Some(r) if a <= r.1 + 64 => { r.1 = a + 4; r.2 += 1; }
                _ => ranges.push((a, a + 4, 1)),
            }
        }
        i += 1;
    }
    ranges.sort_by_key(|r| std::cmp::Reverse(r.1 - r.0));
    for (a, e, n) in ranges.iter().take(25) {
        println!("{:08X}-{:08X} {:6} bytes ({n} changed)", a + lo, e + lo, e - a);
    }
}

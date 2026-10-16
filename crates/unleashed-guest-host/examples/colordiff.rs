//! Runs forward 120 frames; lists changed 16-byte-aligned runs of 6+ vec4 whose components
//! all lie in [0, 8] (colour-like): candidates for the light-field sample (6 colours).
use unleashed_guest_host::*;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let pad = Pad { left: [0.0, 1.0], ..Default::default() };
    let b = g.base();
    let lo = 0x0002_0000usize;
    let hi = 0x1000_0000usize;
    let before: Vec<u8> = unsafe { std::slice::from_raw_parts(b.add(lo), hi - lo) }.to_vec();
    for _ in 0..120 { g.step(&pad); }
    let after = unsafe { std::slice::from_raw_parts(b.add(lo), hi - lo) };
    let f = |s: &[u8], o: usize| f32::from_be_bytes([s[o], s[o + 1], s[o + 2], s[o + 3]]);
    let colourish = |o: usize| (0..4).all(|k| { let v = f(after, o + 4 * k); v.is_finite() && (0.0..=8.0).contains(&v) }) && (0..3).any(|k| f(after, o + 4 * k) > 0.01);
    let mut o = 0;
    while o + 16 <= after.len() {
        if after[o..o + 16] != before[o..o + 16] && colourish(o) {
            let mut n = 1;
            while o + 16 * (n + 1) <= after.len() && colourish(o + 16 * n) { n += 1; }
            if (6..=12).contains(&n) {
                let vals: Vec<String> = (0..n.min(6)).map(|i| format!("({:.3},{:.3},{:.3},{:.3})", f(after, o + 16*i), f(after, o + 16*i + 4), f(after, o + 16*i + 8), f(after, o + 16*i + 12))).collect();
                println!("{:08X} x{n}: {}", o + lo, vals.join(" "));
            }
            o += 16 * n;
        } else {
            o += 16;
        }
    }
}

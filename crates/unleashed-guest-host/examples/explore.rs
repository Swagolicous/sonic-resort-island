//! Scripted input over many frames (run, turn, jump, boost, stomp, brake) to reach the code
//! a player would; prints the functions that are not lifted yet as `missing ...`.
use unleashed_guest_host::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: explore <82346F08_pre.bin> [frames]");
    let frames: u32 = std::env::args().nth(2).map_or(1200, |s| s.parse().unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let mut reported = 0;
    for f in 0..frames {
        let t = f % 240;
        let mut pad = Pad::default();
        let angle = f as f32 * 0.01;
        pad.left = match t {
            0..=59 => [0.0, 1.0],
            60..=119 => [angle.sin(), angle.cos()],
            120..=149 => [0.0, -1.0],
            150..=199 => [1.0, 0.3],
            _ => [-0.7, 0.7],
        };
        if (20..26).contains(&t) || (130..134).contains(&t) || (200..203).contains(&t) {
            pad.down |= button::A;
        }
        if (40..48).contains(&t) {
            pad.down |= button::A; // double jump / homing
        }
        if (70..110).contains(&t) || (210..235).contains(&t) {
            pad.down |= button::X;
        }
        if (205..208).contains(&t) {
            pad.down |= button::B;
        }
        if (160..163).contains(&t) {
            pad.down |= button::LEFT_BUMPER;
        }
        if (175..178).contains(&t) {
            pad.down |= button::RIGHT_BUMPER;
        }
        pad.right = [(f as f32 * 0.03).sin(), 0.0];
        let s = g.step(&pad);
        if f % 120 == 0 {
            println!("f{f:4} pos {:7.1?}", s.position);
        }
        // Report as soon as something new is missing (a later crash keeps the list).
        if g.missing().len() > reported {
            reported = g.missing().len();
            println!("missing {:08X?}", g.missing());
        }
    }
    println!("missing {:08X?}", g.missing());
}

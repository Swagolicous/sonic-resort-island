//! Scans [start, start+len) of a snapshot for vec3s within `radius` of a point.
//! usage: nearvec <pre.bin> <start hex> <len hex> <x> <y> <z> <radius>
use xenon_rt::*;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut snap = Snapshot::open(a[0].as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    let (start, len) = (u32::from_str_radix(&a[1], 16).unwrap(), u32::from_str_radix(&a[2], 16).unwrap());
    let p: Vec<f32> = a[3..6].iter().map(|s| s.parse().unwrap()).collect();
    let r: f32 = a[6].parse().unwrap();
    let f = |x: u32| f32::from_bits(unsafe { PPC_LOAD_U32(mem.base, x) });
    for o in (0..len).step_by(4) {
        let v = [f(start + o), f(start + o + 4), f(start + o + 8)];
        let d = ((v[0] - p[0]).powi(2) + (v[1] - p[1]).powi(2) + (v[2] - p[2]).powi(2)).sqrt();
        if d < r {
            println!("{:08X} (+{o:#X}) {v:8.3?} dist {d:.2}", start + o);
        }
    }
}

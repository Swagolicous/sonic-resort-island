//! What a live particle effect is made of. Runs to a chosen frame of the script below
//! (jump at 60, boost from 150), then for every live effect, each emitter and each of its
//! particle objects prints the words of the object: strings they point to, floats,
//! pointers to other heap objects (with that object's vtable).
//!
//! usage: fxdump <82346F08_pre.bin> <frame> [words per object]
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: fxdump <82346F08_pre.bin> <frame> [words]");
    let frame: u32 = args.next().expect("frame").parse().unwrap();
    let words: u32 = args.next().map_or(120, |s| s.parse().unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    g.sparkle = true;
    g.message_pass = true;
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [8000.0, 0.5, 8000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    for f in 0..=frame {
        let mut down = 0;
        if (60..75).contains(&f) {
            down |= button::A;
        }
        if (150..270).contains(&f) {
            down |= button::X;
        }
        g.step(&Pad { down, left: [0.0, 1.0], ..Default::default() });
    }
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let text = |a: u32| -> Option<String> {
        if !(0x1_0000..0xF000_0000).contains(&a) {
            return None;
        }
        let s: String = (0..48).map(|i| unsafe { PPC_LOAD_U8(b, a + i) }).take_while(|&c| c != 0).map(|c| c as char).collect();
        (s.len() >= 4 && s.chars().all(|c| c.is_ascii_alphanumeric() || "_-.[]@ ".contains(c))).then_some(s)
    };
    let dump = |label: String, a: u32| {
        println!("{label} {a:08X}");
        let mut line = String::new();
        for i in 0..words {
            let w = rd(a + 4 * i);
            let f = f32::from_bits(w);
            let cell = if w == 0 {
                continue;
            } else if let Some(s) = text(w) {
                format!("\"{s}\"")
            } else if let Some(s) = (0x2_0000..0x7FEA_0000).contains(&w).then(|| text(rd(w))).flatten() {
                format!("->\"{s}\"")
            } else if (0x8200_0000..0x8220_0000).contains(&w) {
                format!("vt:{w:08X}")
            } else if f.is_finite() && f.abs() > 1e-4 && f.abs() < 1e5 {
                format!("{f:.3}")
            } else if (0x2_0000..0x7FEA_0000).contains(&w) && w & 3 == 0 {
                let v = rd(w);
                if (0x8200_0000..0x8220_0000).contains(&v) { format!("*{w:08X}(vt {v:08X})") } else { format!("*{w:08X}") }
            } else {
                format!("{w:#X}")
            };
            line += &format!(" +{:X}={cell}", 4 * i);
        }
        println!("  {line}");
    };
    let s = g.sonic();
    println!("frame {frame}: Sonic feet {:.2?}", s.body_position);
    for effect in g.sparkle_effects() {
        dump("effect".into(), effect);
        let node = rd(effect + 0xF0);
        if node >= 0x1000 {
            dump(" effect +F0 (matrix node)".into(), node);
        }
        for (k, (emitter, particles)) in g.sparkle_emitters(effect).into_iter().enumerate() {
            dump(format!(" emitter {k}"), emitter);
            for (j, p) in particles.into_iter().enumerate() {
                dump(format!("  particle {k}.{j}"), p);
                // What its three pointers at +0x74 / +0x78 / +0x7C delimit.
                let (lo, hi) = (rd(p + 0x74), rd(p + 0x78));
                if lo >= 0x1000 && hi >= lo && hi - lo <= 0x400 {
                    let raw: Vec<String> = (0..(hi - lo) / 4 + 2).map(|i| format!("{:08X}", rd(lo + 4 * i))).collect();
                    println!("   +74..+78 ({} bytes): {}", hi - lo, raw.join(" "));
                    for i in 0..(hi - lo) / 4 {
                        let q = rd(lo + 4 * i);
                        if (0x2_0000..0x7FEA_0000).contains(&q) && q & 3 == 0 && q != p {
                            dump(format!("    [{i}] ->"), q);
                        }
                    }
                }
            }
        }
    }
}

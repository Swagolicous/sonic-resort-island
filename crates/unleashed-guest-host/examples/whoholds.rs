//! Where does a message Sonic sends end up? Boosts for half a second, finds the message
//! objects of the given class (by vtable) on the heap, and for each lists who points at
//! it: the address, and the vtable-like words before that address (its owner).
//!
//! usage: whoholds <82346F08_pre.bin> <message vtable hex>
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: whoholds <82346F08_pre.bin> <message vtable hex>");
    let vtable = u32::from_str_radix(&args.next().expect("message vtable"), 16).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let world = g.havok_world().expect("no hkpWorld");
    for (body, motion, _, shape) in g.rigid_bodies(world) {
        if motion == 7 && shape != 0 {
            g.remove_body(world, body);
        }
    }
    let feet = g.sonic().body_position;
    g.add_fixed_box(world, [feet[0], feet[1] - 0.5, feet[2]], [8000.0, 0.5, 8000.0], [0.0, 0.0, 0.0, 1.0], 0.0, 0x7);
    for f in 0..90u32 {
        g.step(&Pad { down: if f >= 60 { button::X } else { 0 }, left: [0.0, 1.0], ..Default::default() });
    }
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let heap = || (0x0002_0000u32..0x7FEA_0000).step_by(4);
    let messages: Vec<u32> = heap().filter(|&p| rd(p) == vtable).collect();
    println!("{} message objects: {messages:08X?}", messages.len());
    for &m in &messages {
        println!("message {m:08X}: {:08X?}", (0..12).map(|i| rd(m + 4 * i)).collect::<Vec<_>>());
        for p in heap().filter(|&p| rd(p) == m) {
            let mut owners = Vec::new();
            let mut q = p;
            while q > p.saturating_sub(0x300) && owners.len() < 3 {
                let w = rd(q);
                if (0x8200_0000..0x8220_0000).contains(&w) {
                    owners.push(format!("{w:08X}@-{:#X}", p - q));
                }
                q -= 4;
            }
            println!("   held at {p:08X}; before it: {}", owners.join(" "));
        }
    }
}

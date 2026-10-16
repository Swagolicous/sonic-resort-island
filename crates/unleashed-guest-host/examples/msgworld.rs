//! Finds the object the engine's per-frame message pass (sub_824EC080) runs on: an
//! object X whose member block (*(X + 4)) holds the message manager at +12. Prints the
//! candidates (X, its vtable, the member block).
//!
//! usage: msgworld <82346F08_pre.bin>
use std::collections::HashSet;
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: msgworld <82346F08_pre.bin>");
    let g = GuestSonic::load(path.as_ref()).unwrap();
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let manager = rd(g.player + 40 + 112);
    println!("message manager {manager:08X} (vtable {:08X})", rd(manager));
    let heap = || (0x0002_0000u32..0x7FEA_0000).step_by(4);
    let members: HashSet<u32> = heap().filter(|&p| rd(p + 12) == manager).collect();
    println!("{} blocks hold the manager at +12", members.len());
    for x in heap() {
        let m = rd(x + 4);
        if members.contains(&m) && (0x8200_0000..0x8220_0000).contains(&rd(x)) {
            println!("   object {x:08X} vtable {:08X} member {m:08X}", rd(x));
        }
    }
}

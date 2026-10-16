//! Matrix nodes around Sonic: the parent chain of the node his effects hang from (the
//! SWA::CMatrixNodeNoScale given as the argument, default 08767330), with each node's
//! vtable, world translation (+0xD0) and whether it moves over 60 frames of running; and
//! what happens when each node's UpdateMatrix (vtable slot 1) is called down the chain.
//!
//! usage: nodes <82346F08_pre.bin> [node hex]
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: nodes <82346F08_pre.bin> [node hex]");
    let leaf = std::env::args().nth(2).map_or(0x0876_7330, |s| u32::from_str_radix(&s, 16).unwrap());
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let rf = |a: u32| f32::from_bits(rd(a));
    let body = rd(g.context + 0x10);
    let mut chain = vec![leaf];
    while let Some(&n) = chain.last() {
        let parent = rd(n + 4);
        if parent < 0x2_0000 || parent >= 0x7FEA_0000 || chain.len() > 12 {
            break;
        }
        chain.push(parent);
    }
    // Where a node keeps a translation that looks like Sonic's place (searching its floats).
    let place = |n: u32| -> Vec<(u32, [f32; 3])> {
        (0x40..0x140u32).step_by(4).filter(|&o| (rf(n + o) - 263.6).abs() < 60.0 && (rf(n + o + 4) - 14.0).abs() < 30.0 && rf(n + o).abs() > 100.0)
            .map(|o| (o, [rf(n + o), rf(n + o + 4), rf(n + o + 8)])).collect()
    };
    println!("body node {body:08X}; chain from the leaf:");
    for &n in &chain {
        println!("   {n:08X} vtable {:08X} parent {:08X} {}  places {:.2?}", rd(n), rd(n + 4), if n == body { "(the body node)" } else { "" }, place(n));
    }
    for _ in 0..60 {
        g.step(&Pad { left: [0.0, 1.0], ..Default::default() });
    }
    println!("after 60 frames (Sonic body {:.2?}):", g.sonic().body_position);
    for &n in &chain {
        println!("   {n:08X} places {:.2?}", place(n));
    }
    // UpdateMatrix(node, parent) from the top of the chain down.
    for w in chain.windows(2).rev() {
        let (n, parent) = (w[0], w[1]);
        let update = rd(rd(n) + 4);
        g.guest_call(update, &[n, parent]);
    }
    println!("after UpdateMatrix down the chain:");
    for &n in &chain {
        println!("   {n:08X} places {:.2?}", place(n));
    }
    println!("missing {:08X?}", g.missing());
}

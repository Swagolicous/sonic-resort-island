//! Message actors: prints the first words of Sonic's and the render director's message
//! actor subobjects, then every message actor on the heap whose words include a given
//! actor ID (which objects a message addressed to that ID is for).
//!
//! usage: actors <82346F08_pre.bin> <actor id hex>...
use unleashed_guest_host::*;
use xenon_rt::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: actors <82346F08_pre.bin> <actor id hex>...");
    let ids: Vec<u32> = args.map(|a| u32::from_str_radix(&a, 16).unwrap()).collect();
    let g = GuestSonic::load(path.as_ref()).unwrap();
    let b = g.base();
    let rd = |a: u32| unsafe { PPC_LOAD_U32(b, a) };
    let words = |a: u32, n: u32| (0..n).map(|i| format!("{:08X}", rd(a + 4 * i))).collect::<Vec<_>>().join(" ");
    let director = g.find_object(VT_RENDER_DIRECTOR_FX).unwrap();
    println!("Sonic's actor    {:08X}: {}", g.player + 40, words(g.player + 40, 26));
    println!("director's actor {:08X}: {}", director + 44, words(director + 44, 26));
    // Message actors: objects whose vtable's second method is the shared pump sub_82E601C8.
    let mut p = 0x0002_0000u32;
    let mut count = 0;
    while p < 0x7FEA_0000 {
        let vt = rd(p);
        if (0x8200_0000..0x8220_0000).contains(&vt) && rd(vt + 4) == 0x82E6_01C8 {
            count += 1;
            for &id in &ids {
                if let Some(o) = (1..24).find(|&i| rd(p + 4 * i) == id) {
                    println!("actor {p:08X} (vtable {vt:08X}) has {id:#X} at +{:#X}; pending flag +92 = {}", 4 * o, unsafe { PPC_LOAD_U8(b, p + 92) });
                }
            }
        }
        p += 4;
    }
    println!("{count} message actors");
}

//! Finds hkaSkeleton bone arrays ({char* name, bool lock} x N) whose first name is <name>
//! and prints the bone names in order. usage: bones <pre.bin> <first bone name>
use xenon_rt::*;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut snap = Snapshot::open(a[0].as_ref()).unwrap();
    let mut mem = GuestMemory::new();
    snap.load_into(&mut mem).unwrap();
    let b = mem.base;
    let rd = |x: u32| unsafe { PPC_LOAD_U32(b, x) };
    let cstr = |x: u32| -> String {
        (0..64).map(|i| unsafe { PPC_LOAD_U8(b, x + i) }).take_while(|&c| c != 0).map(|c| c as char).collect()
    };
    let want = format!("{}\0", a[1]);
    let mut strings = Vec::new();
    let mut p = 0x1000u32;
    while p < 0xFFFF_0000 {
        if unsafe { PPC_LOAD_U8(b, p) } == want.as_bytes()[0] && cstr(p) == a[1] {
            strings.push(p);
        }
        p += 1;
        if p & 0xFFFFFF == 0 && strings.len() > 50 { break; }
    }
    println!("{} strings", strings.len());
    let mut q = 0x1000u32;
    while q < 0xFFFF_0000 {
        let w = rd(q);
        if strings.contains(&w) {
            // Bone arrays: stride 8, names are valid strings.
            let names: Vec<String> = (0..130).map(|i| rd(q + 8 * i)).take_while(|&n| n > 0x1000 && n < 0xF000_0000).map(cstr).take_while(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_graphic())).collect();
            if names.len() > 20 {
                println!("array at {q:08X}: {} bones: {}", names.len(), names.join(" "));
            }
        }
        q += 4;
    }
}

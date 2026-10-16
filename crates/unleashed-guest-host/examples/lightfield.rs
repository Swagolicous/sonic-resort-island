use unleashed_guest_host::*;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut g = GuestSonic::load(path.as_ref()).unwrap();
    let b = g.base();
    let rd = |a: u32| unsafe { xenon_rt::PPC_LOAD_U32(b, a) };
    let lf = g.light_field_handle();
    println!("light field handle {lf:08X?}");
    if let Some(lf) = lf {
        let o = rd(lf);
        println!("*lf = {o:08X}, flag@148 = {}", unsafe { xenon_rt::PPC_LOAD_U8(b, lf + 148) });
        let words: Vec<String> = (0..24).map(|i| format!("{:08X}", rd(o + 4 * i))).collect();
        println!("obj: {}", words.join(" "));
        let w2: Vec<String> = (0..16).map(|i| format!("{:08X}", rd(lf + 4 * i))).collect();
        println!("lf: {}", w2.join(" "));
    }
    for f in 0..31 {
        let s = g.step(&Pad { left: [0.0, 1.0], ..Default::default() });
        if f % 30 == 0 {
            println!("pos {:6.1?} light field {:?}", s.position, g.light_field_sample(s.position).map(|(c, sh)| (c.map(|v| v.map(|x| (x * 1000.0).round() / 1000.0)), sh)));
        }
    }
}

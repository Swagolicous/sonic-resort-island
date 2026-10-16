//! Compare accelerated resource lookup with the previous full-heap word scan.
use std::{path::Path, time::Instant};
use unleashed_guest_host::GuestSonic;
use xenon_rt::{PPC_LOAD_U8, PPC_LOAD_U32, PPC_STORE_U32};

fn legacy(base: *mut u8, pointer: u32) -> Vec<String> {
    let text = |a: u32| -> Option<String> {
        if !(0x1_0000..0xF000_0000).contains(&a) { return None; }
        let s: String = (0..96).map(|i| unsafe { PPC_LOAD_U8(base,a+i) })
            .take_while(|&c| c!=0).map(|c| c as char).collect();
        (s.len()>=4 && s.len()<96 && s.chars().all(|c| c.is_ascii_alphanumeric() || "_-.[]@ #".contains(c))).then_some(s)
    };
    let mut names=Vec::new();
    let mut p=0x2_0000u32;
    while p<0x7FEA_0000 && pointer!=0 {
        if unsafe { PPC_LOAD_U32(base,p) }==pointer {
            for o in (0..0x60u32).step_by(4) {
                for q in [p.wrapping_sub(o),p+o] {
                    if let Some(s)=text(unsafe { PPC_LOAD_U32(base,q) }) {
                        if !names.contains(&s) { names.push(s); }
                    }
                }
            }
        }
        p+=4;
    }
    names
}

fn main() {
    let path=std::env::args().nth(1).expect("resource_lookup <snapshot>");
    let mut g=GuestSonic::load(Path::new(&path)).unwrap();
    let base=g.base();
    let slot=g.guest_alloc(1024);
    for (i,name) in ["fx_sonic_jump","fx_sonic_boost"].iter().enumerate() {
        unsafe { std::ptr::copy_nonoverlapping(name.as_ptr(),base.add(slot as usize+i*128),name.len()); }
    }
    // Aligned holders, multiple candidates, a duplicate and an unaligned decoy.
    for (offset,word) in [(256,slot),(264,0x90F3_A2B1),(384,slot+128),(392,0x90F3_A2B1),
        (512,slot),(520,0x91E4_B3C2),(648,0x91E4_B3C2),(640,slot)] {
        unsafe { PPC_STORE_U32(base,slot+offset,word); }
    }
    unsafe { std::ptr::copy_nonoverlapping(0x90F3_A2B1u32.to_be_bytes().as_ptr(),base.add(slot as usize+801),4); }
    // An unaligned repeated-byte match immediately precedes an aligned holder.
    unsafe {
        std::ptr::write_bytes(base.add(slot as usize+863),0x90,5);
        PPC_STORE_U32(base,slot+880,slot);
    }
    for pointer in [0x90F3_A2B1,0x91E4_B3C2,0x9090_9090,0x92D5_C4E3,0] {
        let t=Instant::now(); let fast=g.resource_names(pointer); let fast_time=t.elapsed();
        let t=Instant::now(); let old=legacy(base,pointer); let old_time=t.elapsed();
        assert_eq!(fast,old,"lookup changed names or ordering");
        assert_eq!(g.resource_names(pointer),old,"cache changed results");
        if pointer==0x90F3_A2B1 {
            assert!(fast.contains(&"fx_sonic_jump".into()) && fast.contains(&"fx_sonic_boost".into()));
        }
        if pointer==0x9090_9090 { assert!(fast.contains(&"fx_sonic_jump".into())); }
        println!("PASS {pointer:08X}: same candidates and order; SIMD {:.2} ms, scalar {:.2} ms; names {fast:?}",
            fast_time.as_secs_f64()*1000.0,old_time.as_secs_f64()*1000.0);
    }
}

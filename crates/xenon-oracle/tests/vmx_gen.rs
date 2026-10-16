//! Every mechanically translated VMX function (sheets/vmx_functions.csv) against the guest.

use unleashed_game::sonic::vmx_gen::ENTRIES;
use xenon_oracle::Oracle;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// Ordinary values, unit-scale values (quaternions), tiny, huge, zero.
    fn f32(&mut self) -> f32 {
        let sign = if self.next() & 1 == 0 { 1.0 } else { -1.0 };
        match self.next() % 10 {
            0 => 0.0,
            1 => f32::from_bits((self.next() as u32 & 0x007F_FFFF) | 0x3400_0000) * sign,
            2 => (self.next() % 2_000_000) as f32 * 1e3 - 1e9,
            3..=6 => (self.next() % 2_000_001) as f32 / 2_000_000.0 * sign,
            _ => (self.next() % 2_000_001) as f32 / 1000.0 - 1000.0,
        }
    }
}

#[test]
fn translated_vmx_functions_match_the_guest() {
    let Some(mut o) = Oracle::get() else {
        eprintln!("xenon oracle not built; skipping");
        return;
    };
    // One 256-byte block per argument register r3..r10, so inputs and outputs never alias.
    let blocks: Vec<u32> = (0..8).map(|_| o.alloc(256)).collect();
    for e in ENTRIES {
        let mut rng = Rng(0x5EED_0000_0000_0000 ^ e.address as u64);
        for i in 0..20_000 {
            let args: Vec<[f32; 4]> = e.inputs.iter().map(|_| [rng.f32(), rng.f32(), rng.f32(), rng.f32()]).collect();
            for (&(r, off), v) in e.inputs.iter().zip(&args) {
                o.write_vec4(blocks[r as usize - 3] + off, *v);
            }
            for (k, b) in blocks.iter().enumerate() {
                o.set_r(3 + k as i32, *b as u64);
            }
            o.call(e.address);
            let port = (e.call)(&args);
            for (&(r, off), p) in e.outputs.iter().zip(&port) {
                let guest = o.read_vec4(blocks[r as usize - 3] + off);
                assert_eq!(guest.map(f32::to_bits), p.map(f32::to_bits),
                    "sub_{:08X} sample {i} r{r}+{off}: args={args:?} guest={guest:?} port={p:?}", e.address);
            }
        }
    }
    assert_eq!(o.import_hits(), 0);
}

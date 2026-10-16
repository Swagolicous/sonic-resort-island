//! Lane-level SSE operations as XenonRecomp's translation of VMX128 uses them, for
//! the mechanically translated vector functions in `vmx_gen.rs` (sheets/vmx_functions.csv,
//! re/xenon/vmx2rust.py). A vector is four u32 lanes in host order: guest element `i`
//! is lane `3 - i`, as the recompiled code keeps it.
//!
//! Not modelled: the flush-to-zero / denormals-are-zero mode the recompiled code turns
//! on around vector code (denormal lanes would be treated as zero there).

pub type V = [u32; 4];

#[inline]
fn f(x: u32) -> f32 {
    f32::from_bits(x)
}
#[inline]
fn b(x: f32) -> u32 {
    x.to_bits()
}

/// Guest vector (x, y, z, w) to host lanes.
#[inline]
pub fn from_guest(v: [f32; 4]) -> V {
    [b(v[3]), b(v[2]), b(v[1]), b(v[0])]
}

/// Host lanes back to the guest vector.
#[inline]
pub fn to_guest(v: V) -> [f32; 4] {
    [f(v[3]), f(v[2]), f(v[1]), f(v[0])]
}

#[inline]
pub fn set1(x: u32) -> V {
    [x; 4]
}

#[inline]
pub fn set1_ps(x: f32) -> V {
    [b(x); 4]
}

#[inline]
pub fn shuffle_epi32(a: V, imm: u8) -> V {
    std::array::from_fn(|i| a[(imm >> (2 * i)) as usize & 3])
}

/// `permute_ps` (same selection as shuffle_epi32 on float lanes).
#[inline]
pub fn permute_ps(a: V, imm: u8) -> V {
    shuffle_epi32(a, imm)
}

/// `blend_ps`: lane i from `b` where bit i of imm is set.
#[inline]
pub fn blend_ps(a: V, bb: V, imm: u8) -> V {
    std::array::from_fn(|i| if imm >> i & 1 != 0 { bb[i] } else { a[i] })
}

#[inline]
pub fn unpackhi_epi32(a: V, bb: V) -> V {
    [a[2], bb[2], a[3], bb[3]]
}

#[inline]
pub fn unpacklo_epi32(a: V, bb: V) -> V {
    [a[0], bb[0], a[1], bb[1]]
}

/// `alignr_epi8(a, b, n)`: bytes n.. of (a:b) with b the low half.
pub fn alignr_epi8(a: V, bb: V, n: u32) -> V {
    let mut bytes = [0u8; 32];
    for i in 0..4 {
        bytes[4 * i..4 * i + 4].copy_from_slice(&bb[i].to_le_bytes());
        bytes[16 + 4 * i..16 + 4 * i + 4].copy_from_slice(&a[i].to_le_bytes());
    }
    let n = n as usize;
    std::array::from_fn(|i| {
        let k = n + 4 * i;
        let byte = |j: usize| if j < 32 { bytes[j] } else { 0 };
        u32::from_le_bytes([byte(k), byte(k + 1), byte(k + 2), byte(k + 3)])
    })
}

macro_rules! lanes_f {
    ($name:ident, $op:tt) => {
        #[inline]
        pub fn $name(a: V, bb: V) -> V {
            std::array::from_fn(|i| b(f(a[i]) $op f(bb[i])))
        }
    };
}
lanes_f!(add_ps, +);
lanes_f!(sub_ps, -);
lanes_f!(mul_ps, *);
lanes_f!(div_ps, /);

#[inline]
pub fn sqrt_ps(a: V) -> V {
    a.map(|x| b(f(x).sqrt()))
}

/// SSE `minps`: the second operand unless the first is strictly smaller (NaN -> second).
#[inline]
pub fn min_ps(a: V, bb: V) -> V {
    std::array::from_fn(|i| if f(a[i]) < f(bb[i]) { a[i] } else { bb[i] })
}

/// SSE `maxps`: the second operand unless the first is strictly greater.
#[inline]
pub fn max_ps(a: V, bb: V) -> V {
    std::array::from_fn(|i| if f(a[i]) > f(bb[i]) { a[i] } else { bb[i] })
}

macro_rules! lanes_cmp {
    ($name:ident, $op:tt) => {
        #[inline]
        pub fn $name(a: V, bb: V) -> V {
            std::array::from_fn(|i| if f(a[i]) $op f(bb[i]) { u32::MAX } else { 0 })
        }
    };
}
lanes_cmp!(cmpeq_ps, ==);
lanes_cmp!(cmpge_ps, >=);
lanes_cmp!(cmpgt_ps, >);

#[inline]
pub fn xor(a: V, bb: V) -> V {
    std::array::from_fn(|i| a[i] ^ bb[i])
}
#[inline]
pub fn and(a: V, bb: V) -> V {
    std::array::from_fn(|i| a[i] & bb[i])
}
#[inline]
pub fn or(a: V, bb: V) -> V {
    std::array::from_fn(|i| a[i] | bb[i])
}
/// `andnot_si128(a, b)` = !a & b.
#[inline]
pub fn andnot(a: V, bb: V) -> V {
    std::array::from_fn(|i| !a[i] & bb[i])
}

/// SSE4.1 `dpps a, b, imm`: products of the lanes in imm's high nibble summed as
/// (l0 + l1) + (l2 + l3), broadcast to the lanes in its low nibble (others 0).
pub fn dp_ps(a: V, bb: V, imm: u8) -> V {
    let p: [f32; 4] = std::array::from_fn(|i| if imm >> (4 + i) & 1 != 0 { f(a[i]) * f(bb[i]) } else { 0.0 });
    let s = (p[0] + p[1]) + (p[2] + p[3]);
    std::array::from_fn(|i| if imm >> i & 1 != 0 { b(s) } else { 0 })
}

/// `vslw`: each lane shifted left by the low 5 bits of the shift lane's lowest byte.
#[inline]
pub fn slw(a: V, s: V) -> V {
    std::array::from_fn(|i| a[i] << (s[i] & 0x1F))
}

/// `vsrw`.
#[inline]
pub fn srw(a: V, s: V) -> V {
    std::array::from_fn(|i| a[i] >> (s[i] & 0x1F))
}

//! The 360's vector float operations, exactly as the recompiled game evaluates them.
//!
//! UnleashedRecomp runs XenonRecomp's translation of default.xex with
//! `-march=sandybridge` (no FMA) and `/fp:strict`, so every multiply and add rounds on
//! its own; Rust never fuses either, so writing the same operations in the same order
//! reproduces it bit for bit. The few instructions whose translation is not the plain
//! operation are spelled out here (checked against the recompiled code by the
//! xenon-oracle tests).
//!
//! Not modelled: the VMX flush-to-zero mode XenonRecomp turns on around vector code
//! (denormal inputs/results become 0 there).

/// `vmsum3fp128 a, b`: SSE4.1 `dpps` with mask 0xEF over XenonRecomp's reversed lanes
/// (x in lane 3, z in lane 1, lane 0 dropped), so the sum is (0 + z) + (y + x).
#[inline]
pub fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let low = 0.0 + a[2] * b[2];
    let high = a[1] * b[1] + a[0] * b[0];
    low + high
}

/// `vrsqrtefp128`: translated as an exact `1 / sqrt(x)` (two roundings).
#[inline]
pub fn rsqrte(x: f32) -> f32 {
    1.0 / x.sqrt()
}

/// `vmaddfp a, b, c` = a * b + c, unfused.
#[inline]
pub fn madd(a: f32, b: f32, c: f32) -> f32 {
    a * b + c
}

/// `vnmsubfp a, b, c` = -(a * b - c), unfused.
#[inline]
pub fn nmsub(a: f32, b: f32, c: f32) -> f32 {
    -(a * b - c)
}

/// Reciprocal square root refined by one Newton step, as the game's length and
/// normalize idioms compute it from a squared length `d`.
#[inline]
fn rsqrt_refined(d: f32) -> f32 {
    let r = rsqrte(d);
    let half = 0.5;
    let t = nmsub(d * half, r * r, half);
    madd(r, t, r)
}

/// The game's vector length idiom (e.g. sub_8231A468): squared length, reciprocal
/// sqrt estimate, one Newton step, times the squared length; 0 stays 0.
#[inline]
pub fn length3(v: [f32; 3]) -> f32 {
    let d = dot3(v, v);
    if d == 0.0 { d } else { d * rsqrt_refined(d) }
}

/// sub_82E86198: normalize (all four lanes scaled by the xyz length's refined
/// reciprocal); a zero vector stays zero.
#[inline]
pub fn normalize3(v: [f32; 4]) -> [f32; 4] {
    let d = dot3([v[0], v[1], v[2]], [v[0], v[1], v[2]]);
    if d == 0.0 {
        return [d; 4];
    }
    let r = rsqrt_refined(d);
    v.map(|c| c * r)
}

/// Row-major 4x4 matrix; vectors are rows (`v * M`), as the game stores them.
pub type Mat4 = [[f32; 4]; 4];

/// sub_82E84A90: rotate `v` by `m`'s upper rows, x*row0 + (y*row1 + z*row2) per lane.
#[inline]
pub fn transform3(m: &Mat4, v: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| madd(v[0], m[0][i], madd(v[1], m[1][i], v[2] * m[2][i])))
}

/// sub_8230A4E0's math: the forward axis of a frame, `(0, 0, 1, 0) * m` (the game
/// gets `m` from the object at ctx+0x10 through its vtable slot 2).
#[inline]
pub fn forward(m: &Mat4) -> [f32; 4] {
    transform3(m, [0.0, 0.0, 1.0, 0.0])
}

/// sub_8249A6A8: moves `angle` by one turn so it lies within pi of `reference`.
#[inline]
pub fn wrap_near(angle: f32, reference: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    if angle - reference > PI {
        angle - TAU
    } else if reference - angle > PI {
        angle + TAU
    } else {
        angle
    }
}

/// sub_82E84BE0: unit quaternion (x, y, z, w) to a rotation matrix, in the order the
/// game's vector code forms each term.
pub fn quat_to_matrix([x, y, z, w]: [f32; 4]) -> Mat4 {
    let (x2, y2, z2) = (x + x, y + y, z + z);
    let (xx, yy, zz) = (x * x2, y * y2, z * z2);
    let (xy, xz, yz) = (x * y2, x * z2, y * z2);
    let (wx, wy, wz) = (w * x2, w * y2, w * z2);
    [
        [(1.0 - yy) - zz, xy + wz, xz - wy, 0.0],
        [xy - wz, (1.0 - xx) - zz, yz + wx, 0.0],
        [xz + wy, yz - wx, (1.0 - xx) - yy, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// sub_82E85988: rotation of `angle` radians about `axis` as a quaternion
/// (x, y, z, w) = (n * sin(angle/2), cos(angle/2)), n = normalize3(axis).
pub fn axis_angle_quat(axis: [f32; 4], angle: f32) -> [f32; 4] {
    let n = normalize3(axis);
    let (s, c) = super::crt_math::sincos((angle * super::constants::QUAT_HALF_ANGLE) as f64);
    [n[0] * s, n[1] * s, n[2] * s, c]
}

/// sub_82E860B8: the zero vector (all four lanes +0.0).
pub const fn zero4() -> [f32; 4] {
    [0.0; 4]
}

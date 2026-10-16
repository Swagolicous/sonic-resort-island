//! The console CRT's double-precision math, as default.xex implements it. Rust's libm
//! gives different last bits, and the game rounds these results to f32 and feeds
//! them back into Sonic's state every frame, so the port runs the same algorithms.
//! Each routine follows its guest code operation for operation (multiply-adds are
//! unfused: XenonRecomp builds without FMA). Constants: sheets/math_tables.csv.

use super::crt_tables::*;

/// `fctid` as XenonRecomp translates it: `cvtsd2si` (round to nearest even), clamped
/// to i64::MAX above 2^63; NaN and other out-of-range values give i64::MIN.
fn fctid(v: f64) -> i64 {
    const LIMIT: f64 = i64::MAX as f64; // 2^63
    if v > LIMIT {
        return i64::MAX;
    }
    let r = v.round_ties_even();
    if v.is_nan() || r >= LIMIT || r < -LIMIT { i64::MIN } else { r as i64 }
}

/// `fctidz`: as `fctid` but truncating (`cvttsd2si`).
fn fctidz(v: f64) -> i64 {
    const LIMIT: f64 = i64::MAX as f64;
    if v > LIMIT {
        return i64::MAX;
    }
    let r = v.trunc();
    if v.is_nan() || r >= LIMIT || r < -LIMIT { i64::MIN } else { r as i64 }
}

/// The degree-8 odd polynomial both sin and cos evaluate on the reduced argument.
fn sincos_poly(r: f64) -> f64 {
    let z = r * r;
    let p = SINCOS_D112 * z + SINCOS_D104;
    let p = p * z + SINCOS_D96;
    let p = p * z + SINCOS_D88;
    let p = p * z + SINCOS_D80;
    let p = p * z + SINCOS_D72;
    let p = p * z + SINCOS_D64;
    let p = p * z + SINCOS_D56;
    let p = p * z + SINCOS_C0_D0;
    p * r
}

/// sub_831B1788: sin(x). Reduces |x| by the nearest multiple of pi (pi split in two,
/// Cody-Waite), flips sign on odd multiples and for negative x; NaN past ~2.2e8.
pub fn sin(x: f64) -> f64 {
    let a = x.abs();
    let n = fctid(SINCOS_D8 * a) as f64;
    let sign = if x >= 0.0 { SINCOS_S28 as f64 } else { SINCOS_S32 as f64 };
    let r = -(SINCOS_D40 * n - a);
    let odd = fctidz(n) & 1 != 0;
    let r = -(SINCOS_D48 * n - r);
    let mut s = sincos_poly(r);
    if odd {
        s = -s;
    }
    let s = s * sign;
    if a.to_bits() == 0 {
        return x; // sin(+-0) = +-0
    }
    if a - SINCOS_D16 >= 0.0 { SINCOS_BIG_D0 } else { s }
}

/// sub_831B1868: cos(x) = sin(|x| + pi/2), reduced around n - 1/2.
pub fn cos(x: f64) -> f64 {
    let a = x.abs();
    let shifted = SINCOS_D0 + a;
    let n = fctid(SINCOS_D8 * shifted) as f64;
    let m = n - SINCOS_S36 as f64;
    let odd = fctidz(n) & 1 != 0;
    let r = -(SINCOS_D40 * m - a);
    let r = -(SINCOS_D48 * m - r);
    let mut c = sincos_poly(r);
    if odd {
        c = -c;
    }
    if a == SINCOS_S24 as f64 {
        return SINCOS_S28 as f64; // cos(0) = 1
    }
    if shifted - SINCOS_D16 >= 0.0 { SINCOS_BIG_D0 } else { c }
}

/// sub_82453900: sin and cos of `angle`, each rounded to f32.
pub fn sincos(angle: f64) -> (f32, f32) {
    (sin(angle) as f32, cos(angle) as f32)
}

/// sub_831B3A68: atan2(y, x).
pub fn atan2(y: f64, x: f64) -> f64 {
    let zero = ATAN2_S168 as f64;
    let angle = if x == zero {
        if y == zero {
            // atan2(+-0, +0) = y; atan2(+-0, -0) = +-pi.
            return if !x.is_sign_negative() {
                y
            } else if y.is_sign_negative() {
                -ATAN2_D16
            } else {
                ATAN2_D16
            };
        }
        ATAN2_D8
    } else {
        let (ax, ay) = (x.abs(), y.abs());
        // Reduce to t = small / large in [0, 1]; `octant` picks the offset below.
        let (mut octant, small, large) = if ay > ax { (2usize, ax, ay) } else { (0, ay, ax) };
        let mut t = small / large;
        if t > ATAN2_D24 {
            // Past tan(pi/12): t = (sqrt3 * t - 1) / (sqrt3 + t), offset by pi/6.
            octant += 1;
            let den = ATAN2_D40 + t;
            t = (ATAN2_D40 * t - ATAN2_S176 as f64) / den;
        }
        let z = t * t;
        let p = ((ATAN2_D80 * z + ATAN2_D72) * z + ATAN2_D64) * z + ATAN2_D56;
        let p = p * z * t;
        let q = (((ATAN2_D112 + z) * z + ATAN2_D104) * z + ATAN2_D96) * z + ATAN2_D88;
        let mut r = p / q + t;
        if octant > 1 {
            r = -r;
        }
        [ATAN2_D128, ATAN2_D136, ATAN2_D144, ATAN2_D152][octant] + r
    };
    // Left half-plane: pi - angle (fsel takes x >= 0, -0 included); then y's sign.
    let r = if x >= 0.0 { angle } else { ATAN2_D16 - angle };
    if y.is_sign_negative() { -r } else { r }
}

/// sub_831B3B98: the CRT's shared asin / acos core (`acos` selects acos).
fn asin_acos(x: f64, acos: bool) -> f64 {
    let a = x.abs();
    let (z, t, slot) = if a > ASIN_ACOS_S12 as f64 {
        // |x| > 1/2: t = -2 sqrt((1 - |x|) / 2).
        let z = (ASIN_ACOS_S4 as f64 - a) * ASIN_ACOS_S12 as f64;
        (z, -(z.sqrt() * ASIN_ACOS_S8 as f64), 1 - acos as usize)
    } else {
        if !acos && x == ASIN_ACOS_S0 as f64 {
            return x;
        }
        (a * a, a, acos as usize)
    };
    let p = ASIN_ACOS_D80 * z + ASIN_ACOS_D72;
    let q = ASIN_ACOS_D120 + z;
    let p = p * z + ASIN_ACOS_D64;
    let q = q * z + ASIN_ACOS_D112;
    let p = p * z + ASIN_ACOS_D56;
    let q = q * z + ASIN_ACOS_D104;
    let p = p * z + ASIN_ACOS_D48;
    let q = q * z + ASIN_ACOS_D96;
    let p = p * z;
    let q = q * z + ASIN_ACOS_D88;
    let r = p * t / q + t;
    if !acos {
        let r = [ASIN_ACOS_D16, ASIN_ACOS_D24][slot] + r;
        return if x >= 0.0 { r } else { -r };
    }
    let (hi, lo) = ([ASIN_ACOS_D16, ASIN_ACOS_D24][slot], [ASIN_ACOS_D32, ASIN_ACOS_D40][slot]);
    if x >= 0.0 { hi - r } else { lo + r }
}

/// sub_831B3D58: acos(x).
pub fn acos(x: f64) -> f64 {
    asin_acos(x, true)
}

/// The CRT asin (sub_831B3B98 with r4 = 0).
pub fn asin(x: f64) -> f64 {
    asin_acos(x, false)
}

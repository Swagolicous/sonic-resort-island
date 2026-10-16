//! Runtime for guest code lifted to Rust from XenonRecomp's C++ (re/xenon/ppc2rust.py).
//!
//! Mirrors XenonUtils/ppc_context.h with SWA.toml's configuration (re/xenon/ppc/ppc_config.h):
//! register unions, CR / XER, the FPSCR as the host MXCSR, big-endian guest memory at
//! `base`, and the lvx/stvx byte-swap tables. The lifted code uses the same x86 SSE
//! intrinsics the C++ compiles to (simde on x86-64 maps 1:1), under the same MXCSR, so it
//! computes the same bits; crates/xenon-oracle checks that against the C++.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, clippy::missing_safety_doc)]

use core::arch::x86_64::*;

pub mod kernel;

pub use core::arch::x86_64;

pub const PPC_IMAGE_BASE: u32 = 0x8200_0000;
pub const PPC_IMAGE_SIZE: u32 = 0x014F_0000;
pub const PPC_CODE_BASE: u32 = 0x822C_0000;
pub const PPC_CODE_SIZE: u32 = 0x00F9_189C;

#[derive(Clone, Copy)]
#[repr(C)]
pub union PPCRegister {
    pub s8: i8,
    pub u8: u8,
    pub s16: i16,
    pub u16: u16,
    pub s32: i32,
    pub u32: u32,
    pub s64: i64,
    pub u64: u64,
    pub f32: f32,
    pub f64: f64,
}

impl Default for PPCRegister {
    fn default() -> Self {
        PPCRegister { u64: 0 }
    }
}

#[derive(Clone, Copy)]
#[repr(C, align(16))]
pub union PPCVRegister {
    pub s8: [i8; 16],
    pub u8: [u8; 16],
    pub s16: [i16; 8],
    pub u16: [u16; 8],
    pub s32: [i32; 4],
    pub u32: [u32; 4],
    pub s64: [i64; 2],
    pub u64: [u64; 2],
    pub f32: [f32; 4],
    pub f64: [f64; 2],
}

impl Default for PPCVRegister {
    fn default() -> Self {
        PPCVRegister { u8: [0; 16] }
    }
}

#[derive(Clone, Copy, Default)]
pub struct PPCXERRegister {
    pub so: u8,
    pub ov: u8,
    pub ca: u8,
}

/// `so` doubles as `un` (unordered) after a floating-point compare, as in the C++ union.
#[derive(Clone, Copy, Default)]
pub struct PPCCRRegister {
    pub lt: u8,
    pub gt: u8,
    pub eq: u8,
    pub so: u8,
}

impl PPCCRRegister {
    #[inline(always)]
    pub fn compare<T: PartialOrd>(&mut self, left: T, right: T, xer: &PPCXERRegister) {
        self.lt = (left < right) as u8;
        self.gt = (left > right) as u8;
        self.eq = (left == right) as u8;
        self.so = xer.so;
    }

    #[inline(always)]
    pub fn compare_f(&mut self, left: f64, right: f64) {
        let un = left.is_nan() || right.is_nan();
        self.so = un as u8;
        self.lt = (!un && left < right) as u8;
        self.gt = (!un && left > right) as u8;
        self.eq = (!un && left == right) as u8;
    }

    #[inline(always)]
    pub unsafe fn set_from_mask_ps(&mut self, mask: __m128, imm: i32) {
        let m = unsafe { _mm_movemask_ps(mask) };
        self.lt = (m == imm) as u8;
        self.gt = 0;
        self.eq = (m == 0) as u8;
        self.so = 0;
    }

    #[inline(always)]
    pub unsafe fn set_from_mask_si(&mut self, mask: __m128i, imm: i32) {
        let m = unsafe { _mm_movemask_epi8(mask) };
        self.lt = (m == imm) as u8;
        self.gt = 0;
        self.eq = (m == 0) as u8;
        self.so = 0;
    }
}

const ROUND_SHIFT: u32 = 13;
const ROUND_MASK: u32 = 0x6000;
const FLUSH_MASK: u32 = 0x8000 | 0x0040; // FTZ | DAZ
const HOST_TO_GUEST: [u32; 4] = [0, 3, 2, 1];
const GUEST_TO_HOST: [u32; 4] = [0x0000, 0x6000, 0x4000, 0x2000];

#[inline(always)]
fn getcsr() -> u32 {
    #[allow(deprecated)]
    unsafe {
        _mm_getcsr()
    }
}

#[inline(always)]
fn setcsr(csr: u32) {
    #[allow(deprecated)]
    unsafe {
        _mm_setcsr(csr)
    }
}

/// The guest FPSCR kept as the host MXCSR (rounding mode and flush-to-zero).
#[derive(Clone, Copy, Default)]
pub struct PPCFPSCRRegister {
    pub csr: u32,
}

impl PPCFPSCRRegister {
    pub fn loadFromHost(&mut self) -> u32 {
        self.csr = getcsr();
        HOST_TO_GUEST[((self.csr & ROUND_MASK) >> ROUND_SHIFT) as usize]
    }
    pub fn storeFromGuest(&mut self, value: u32) {
        self.csr &= !ROUND_MASK;
        self.csr |= GUEST_TO_HOST[(value & 3) as usize];
        setcsr(self.csr);
    }
    #[inline(always)]
    pub fn enableFlushModeUnconditional(&mut self) {
        self.csr |= FLUSH_MASK;
        setcsr(self.csr);
    }
    #[inline(always)]
    pub fn disableFlushModeUnconditional(&mut self) {
        self.csr &= !FLUSH_MASK;
        setcsr(self.csr);
    }
    #[inline(always)]
    pub fn enableFlushMode(&mut self) {
        if self.csr & FLUSH_MASK != FLUSH_MASK {
            self.csr |= FLUSH_MASK;
            setcsr(self.csr);
        }
    }
    #[inline(always)]
    pub fn disableFlushMode(&mut self) {
        if self.csr & FLUSH_MASK != 0 {
            self.csr &= !FLUSH_MASK;
            setcsr(self.csr);
        }
    }
}

/// The registers SWA.toml keeps in the context (everything else is a local of the
/// lifted function, zero-initialised, as in the C++).
#[derive(Clone, Copy, Default)]
#[repr(C, align(64))]
pub struct PPCContext {
    pub r1: PPCRegister,
    pub r3: PPCRegister,
    pub r4: PPCRegister,
    pub r5: PPCRegister,
    pub r6: PPCRegister,
    pub r7: PPCRegister,
    pub r8: PPCRegister,
    pub r9: PPCRegister,
    pub r10: PPCRegister,
    pub r13: PPCRegister,
    pub fpscr: PPCFPSCRRegister,
    pub f1: PPCRegister,
    pub f2: PPCRegister,
    pub f3: PPCRegister,
    pub f4: PPCRegister,
    pub f5: PPCRegister,
    pub f6: PPCRegister,
    pub f7: PPCRegister,
    pub f8: PPCRegister,
    pub f9: PPCRegister,
    pub f10: PPCRegister,
    pub f11: PPCRegister,
    pub f12: PPCRegister,
    pub f13: PPCRegister,
    pub v0: PPCVRegister,
    pub v1: PPCVRegister,
    pub v2: PPCVRegister,
    pub v3: PPCVRegister,
    pub v4: PPCVRegister,
    pub v5: PPCVRegister,
    pub v6: PPCVRegister,
    pub v7: PPCVRegister,
    pub v8: PPCVRegister,
    pub v9: PPCVRegister,
    pub v10: PPCVRegister,
    pub v11: PPCVRegister,
    pub v12: PPCVRegister,
    pub v13: PPCVRegister,
}

impl PPCContext {
    pub fn gpr(&mut self, i: u32) -> Option<&mut PPCRegister> {
        Some(match i {
            1 => &mut self.r1,
            3 => &mut self.r3,
            4 => &mut self.r4,
            5 => &mut self.r5,
            6 => &mut self.r6,
            7 => &mut self.r7,
            8 => &mut self.r8,
            9 => &mut self.r9,
            10 => &mut self.r10,
            13 => &mut self.r13,
            _ => return None,
        })
    }
    pub fn fpr(&mut self, i: u32) -> Option<&mut PPCRegister> {
        Some(match i {
            1 => &mut self.f1,
            2 => &mut self.f2,
            3 => &mut self.f3,
            4 => &mut self.f4,
            5 => &mut self.f5,
            6 => &mut self.f6,
            7 => &mut self.f7,
            8 => &mut self.f8,
            9 => &mut self.f9,
            10 => &mut self.f10,
            11 => &mut self.f11,
            12 => &mut self.f12,
            13 => &mut self.f13,
            _ => return None,
        })
    }
    pub fn vr(&mut self, i: u32) -> Option<&mut PPCVRegister> {
        Some(match i {
            0 => &mut self.v0,
            1 => &mut self.v1,
            2 => &mut self.v2,
            3 => &mut self.v3,
            4 => &mut self.v4,
            5 => &mut self.v5,
            6 => &mut self.v6,
            7 => &mut self.v7,
            8 => &mut self.v8,
            9 => &mut self.v9,
            10 => &mut self.v10,
            11 => &mut self.v11,
            12 => &mut self.v12,
            13 => &mut self.v13,
            _ => return None,
        })
    }
}

// ---------------------------------------------------------------- guest memory

/// Guest memory: 4 GiB at `base`, big-endian scalars (PPC_LOAD_* / PPC_STORE_*).
#[inline(always)]
pub unsafe fn PPC_LOAD_U8(base: *mut u8, a: u32) -> u8 {
    unsafe { core::ptr::read_volatile(base.add(a as usize)) }
}
#[inline(always)]
pub unsafe fn PPC_LOAD_U16(base: *mut u8, a: u32) -> u16 {
    unsafe { u16::from_be(core::ptr::read_volatile(base.add(a as usize) as *const u16)) }
}
#[inline(always)]
pub unsafe fn PPC_LOAD_U32(base: *mut u8, a: u32) -> u32 {
    unsafe { u32::from_be((base.add(a as usize) as *const u32).read_unaligned()) }
}
#[inline(always)]
pub unsafe fn PPC_LOAD_U64(base: *mut u8, a: u32) -> u64 {
    unsafe { u64::from_be((base.add(a as usize) as *const u64).read_unaligned()) }
}
#[inline(always)]
pub unsafe fn PPC_STORE_U8(base: *mut u8, a: u32, v: u8) {
    unsafe { core::ptr::write_volatile(base.add(a as usize), v) }
}
#[inline(always)]
pub unsafe fn PPC_STORE_U16(base: *mut u8, a: u32, v: u16) {
    unsafe { (base.add(a as usize) as *mut u16).write_unaligned(v.to_be()) }
}
#[inline(always)]
pub unsafe fn PPC_STORE_U32(base: *mut u8, a: u32, v: u32) {
    unsafe { (base.add(a as usize) as *mut u32).write_unaligned(v.to_be()) }
}
#[inline(always)]
pub unsafe fn PPC_STORE_U64(base: *mut u8, a: u32, v: u64) {
    unsafe { (base.add(a as usize) as *mut u64).write_unaligned(v.to_be()) }
}

/// Host pointer for a guest address (`base + a`).
#[inline(always)]
pub fn host(base: *mut u8, a: u32) -> *mut u8 {
    base.wrapping_add(a as usize)
}

// ---------------------------------------------------------------- vector register access

#[inline(always)]
pub unsafe fn vld(r: &PPCVRegister) -> __m128i {
    unsafe { _mm_load_si128(r as *const PPCVRegister as *const __m128i) }
}
#[inline(always)]
pub unsafe fn vld_ps(r: &PPCVRegister) -> __m128 {
    unsafe { _mm_load_ps(r as *const PPCVRegister as *const f32) }
}
#[inline(always)]
pub unsafe fn vst(r: &mut PPCVRegister, v: __m128i) {
    unsafe { _mm_store_si128(r as *mut PPCVRegister as *mut __m128i, v) }
}
#[inline(always)]
pub unsafe fn vst_ps(r: &mut PPCVRegister, v: __m128) {
    unsafe { _mm_store_ps(r as *mut PPCVRegister as *mut f32, v) }
}

#[repr(C, align(16))]
pub struct Table<const N: usize>(pub [u8; N]);

macro_rules! mask_rows {
    ($f:expr) => {{
        let mut t = [0u8; 256];
        let mut i = 0;
        while i < 256 {
            t[i] = $f(i / 16, i % 16);
            i += 1;
        }
        Table(t)
    }};
}

const fn mask_l(row: usize, col: usize) -> u8 {
    if col < row { 0xFF } else { (15 - (col - row)) as u8 }
}
const fn mask_r(row: usize, col: usize) -> u8 {
    if col < row { (row - 1 - col) as u8 } else { 0xFF }
}
const fn shift_l(row: usize, col: usize) -> u8 {
    (15 + row - col) as u8
}
const fn shift_r(row: usize, col: usize) -> u8 {
    (31 - row - col) as u8
}

/// ppc_context.h's tables, same contents (row = byte offset of the access).
pub static VectorMaskL: Table<256> = mask_rows!(mask_l);
pub static VectorMaskR: Table<256> = mask_rows!(mask_r);
pub static VectorShiftTableL: Table<256> = mask_rows!(shift_l);
pub static VectorShiftTableR: Table<256> = mask_rows!(shift_r);

// ---------------------------------------------------------------- builtins

#[inline(always)]
pub fn __builtin_rotateleft32(x: u32, n: u32) -> u32 {
    x.rotate_left(n)
}
#[inline(always)]
pub fn __builtin_rotateleft64(x: u64, n: u64) -> u64 {
    x.rotate_left(n as u32)
}
/// Undefined for 0 in C; XenonRecomp guards that case.
#[inline(always)]
pub fn __builtin_clz(x: u32) -> i32 {
    x.leading_zeros() as i32
}
#[inline(always)]
pub fn __builtin_clzll(x: u64) -> i32 {
    x.leading_zeros() as i32
}
#[inline(always)]
pub fn __builtin_bswap16(x: u16) -> u16 {
    x.swap_bytes()
}
#[inline(always)]
pub fn __builtin_bswap32(x: u32) -> u32 {
    x.swap_bytes()
}
#[inline(always)]
pub fn __builtin_bswap64(x: u64) -> u64 {
    x.swap_bytes()
}
#[inline(always)]
pub fn __rdtsc() -> u64 {
    unsafe { _rdtsc() }
}

/// `__sync_bool_compare_and_swap` on a host u32 (the guest word, native order compare).
#[inline(always)]
pub unsafe fn __sync_bool_compare_and_swap(p: *mut u8, old: i32, new: i32) -> bool {
    use core::sync::atomic::{AtomicI32, Ordering};
    let a = unsafe { &*(p as *const AtomicI32) };
    a.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst).is_ok()
}

/// x86 `cvttsd2si` / `cvtsd2si` (out of range and NaN give the "integer indefinite").
#[inline(always)]
pub fn cvttsd_si32(x: f64) -> i32 {
    unsafe { _mm_cvttsd_si32(_mm_set_sd(x)) }
}
#[inline(always)]
pub fn cvttsd_si64(x: f64) -> i64 {
    unsafe { _mm_cvttsd_si64(_mm_set_sd(x)) }
}
#[inline(always)]
pub fn cvtsd_si32(x: f64) -> i32 {
    unsafe { _mm_cvtsd_si32(_mm_set_sd(x)) }
}
#[inline(always)]
pub fn cvtsd_si64(x: f64) -> i64 {
    unsafe { _mm_cvtsd_si64(_mm_set_sd(x)) }
}

/// C integer division: x86 traps on 0 and on MIN / -1; the guest code avoids both, and
/// here they give 0 instead of a crash.
pub trait CDiv: Sized {
    fn cdiv(self, d: Self) -> Self;
    fn crem(self, d: Self) -> Self;
}
macro_rules! cdiv {
    ($($t:ty),*) => {$(
        impl CDiv for $t {
            #[inline(always)]
            fn cdiv(self, d: Self) -> Self { self.checked_div(d).unwrap_or(0) }
            #[inline(always)]
            fn crem(self, d: Self) -> Self { self.checked_rem(d).unwrap_or(0) }
        }
    )*};
}
cdiv!(i32, u32, i64, u64);

// ---------------------------------------------------------------- host interface

/// What the lifted code reaches outside itself: kernel imports, indirect calls to
/// functions that were not lifted, and the boundary functions (sheets/boundaries.csv)
/// a host may replace. One `Host` per guest thread of execution.
pub trait Host {
    /// A kernel / XAM import (`__imp__Name`). The oracle counts these and returns 0.
    fn import(&mut self, name: &'static str, ctx: &mut PPCContext, base: *mut u8) {
        let _ = (name, base);
        unsafe { ctx.r3.u64 = 0 };
    }
    /// A guest function the lifted set does not contain.
    fn missing(&mut self, address: u32, ctx: &mut PPCContext, base: *mut u8) {
        let _ = (ctx, base);
        panic!("guest function 0x{address:08X} is not lifted");
    }
    /// A guest function UnleashedRecomp replaces with host code (sheets/host_functions.csv).
    fn hook(&mut self, address: u32, ctx: &mut PPCContext, base: *mut u8) {
        let _ = (ctx, base);
        panic!("host function 0x{address:08X} (sheets/host_functions.csv) is not implemented by this host");
    }
    /// A boundary: return true when the host handled the call (else the lifted guest
    /// function runs).
    fn boundary(&mut self, address: u32, ctx: &mut PPCContext, base: *mut u8) -> bool {
        let _ = (address, ctx, base);
        false
    }
}

/// The host the lifted code calls; installed per thread by [`with_host`].
pub struct HostSlot(pub core::cell::Cell<Option<*mut dyn Host>>);

thread_local! {
    pub static HOST: HostSlot = const { HostSlot(core::cell::Cell::new(None)) };
}

/// Runs `f` with `host` installed for the lifted code on this thread.
pub fn with_host<R>(host: &mut dyn Host, f: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only used while `f` runs, during which `host` stays borrowed.
    let ptr: *mut (dyn Host + '_) = host;
    let ptr: *mut dyn Host = unsafe { core::mem::transmute(ptr) };
    let prev = HOST.with(|h| h.0.replace(Some(ptr)));
    let r = f();
    HOST.with(|h| h.0.set(prev));
    r
}

struct NoHost;
impl Host for NoHost {}

/// Calls into the installed host (a default one that only zeroes imports otherwise).
pub fn host_call<R>(f: impl FnOnce(&mut dyn Host) -> R) -> R {
    match HOST.with(|h| h.0.get()) {
        Some(p) => f(unsafe { &mut *p }),
        None => f(&mut NoHost),
    }
}

// ---------------------------------------------------------------- ppc_context.h helpers

/// `vpermilps` with an immediate: the same lane selection as `shufps a, a, imm`.
#[inline(always)]
pub unsafe fn simde_mm_permute_ps<const IMM: i32>(a: __m128) -> __m128 {
    unsafe { _mm_shuffle_ps::<IMM>(a, a) }
}

unsafe extern "C" {
    #[link_name = "sqrt"]
    fn ucrt_sqrt(x: f64) -> f64;
    #[link_name = "sqrtf"]
    fn ucrt_sqrtf(x: f32) -> f32;
}

/// C++ `sqrt` as the recompiled code calls it: clang-cl with math-errno uses `sqrtsd` and
/// falls back to the UCRT on a NaN result, which returns +qNaN where `sqrtsd` gives the
/// negative default NaN. Calling the UCRT gives the same bits in every case.
#[inline(always)]
pub fn c_sqrt(x: f64) -> f64 {
    let r = x.sqrt();
    if r.is_nan() { unsafe { ucrt_sqrt(x) } } else { r }
}

#[inline(always)]
pub fn c_sqrtf(x: f32) -> f32 {
    let r = x.sqrt();
    if r.is_nan() { unsafe { ucrt_sqrtf(x) } } else { r }
}

// ---------------------------------------------------------------- UnleashedRecomp PC fixes

thread_local! {
    static PC_FIXES: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// Whether the lifted code runs UnleashedRecomp's mid-asm PC fixes (off: the console game,
/// as the oracle runs it; on: what the UnleashedRecomp capture build executes).
pub fn pc_fixes() -> bool {
    PC_FIXES.with(|c| c.get())
}

pub fn set_pc_fixes(on: bool) {
    PC_FIXES.with(|c| c.set(on));
}

/// UnleashedRecomp patches/fps_patches.cpp, same arithmetic.
pub fn DownForceDeltaTimeFixMidAsmHook_r(f0: &mut PPCRegister) {
    f0.f64 = 30.0;
}

pub fn DownForceDeltaTimeFixMidAsmHook_vr(v127: &mut PPCVRegister, f24: &mut PPCRegister) {
    unsafe {
        let factor = 1.0 / (f24.f64 * 30.0);
        for i in 0..4 {
            v127.f32[i] = (v127.f32[i] as f64 * factor) as f32;
        }
    }
}

// ---------------------------------------------------------------- guest memory + snapshots

unsafe extern "system" {
    fn VirtualAlloc(addr: *mut u8, size: usize, kind: u32, protect: u32) -> *mut u8;
    fn VirtualFree(addr: *mut u8, size: usize, kind: u32) -> i32;
}

/// 4 GiB of guest memory (committed, zero; Windows maps pages on first touch), as
/// UnleashedRecomp's kernel/memory.cpp and the oracle allocate it.
pub struct GuestMemory {
    pub base: *mut u8,
}

impl GuestMemory {
    pub fn new() -> Self {
        const MEM_COMMIT_RESERVE: u32 = 0x3000;
        const PAGE_READWRITE: u32 = 4;
        let base = unsafe { VirtualAlloc(core::ptr::null_mut(), 1 << 32, MEM_COMMIT_RESERVE, PAGE_READWRITE) };
        assert!(!base.is_null(), "cannot allocate 4 GiB of guest memory");
        GuestMemory { base }
    }

    pub fn bytes(&self, a: u32, n: usize) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.base.add(a as usize), n) }
    }

    pub fn bytes_mut(&mut self, a: u32, n: usize) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.base.add(a as usize), n) }
    }
}

impl Default for GuestMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        unsafe { VirtualFree(self.base, 0, 0x8000) };
    }
}

pub const SNAPSHOT_PAGE: usize = 0x10000;

/// A guest snapshot written by the capture build (re/capture, patches/guest_snapshot.h),
/// read as a stream: the header and registers up front, then one page at a time (pages
/// come in ascending order), so a 4 GiB snapshot never has to sit in host memory.
pub struct Snapshot {
    pub function: u32,
    pub pc_fixes: bool,
    pub delta_time: f64,
    pub csr: u32,
    pub ctx: PPCContext,
    pub page_count: usize,
    read: usize,
    file: std::io::BufReader<std::fs::File>,
}

impl Snapshot {
    pub fn open(path: &std::path::Path) -> std::io::Result<Self> {
        use std::io::Read;
        let mut file = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path)?);
        let bad = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {m}", path.display()));
        let mut head = [0u8; 36 + 10 * 8 + 13 * 8 + 14 * 16];
        file.read_exact(&mut head)?;
        if &head[..8] != b"ULSNAP01" {
            return Err(bad("not a ULSNAP01 snapshot"));
        }
        let u32_at = |o: usize| u32::from_le_bytes(head[o..o + 4].try_into().unwrap());
        let u64_at = |o: usize| u64::from_le_bytes(head[o..o + 8].try_into().unwrap());
        if u32_at(8) != 1 {
            return Err(bad("unknown version"));
        }
        let (function, flags) = (u32_at(12), u32_at(16));
        let delta_time = f64::from_bits(u64_at(20));
        let csr = u32_at(28);
        let page_count = u32_at(32) as usize;
        let mut o = 36;
        let mut ctx = PPCContext::default();
        for i in [1u32, 3, 4, 5, 6, 7, 8, 9, 10, 13] {
            ctx.gpr(i).unwrap().u64 = u64_at(o);
            o += 8;
        }
        for i in 1..=13 {
            ctx.fpr(i).unwrap().u64 = u64_at(o);
            o += 8;
        }
        for i in 0..=13 {
            let mut v = [0u8; 16];
            v.copy_from_slice(&head[o..o + 16]);
            ctx.vr(i).unwrap().u8 = v;
            o += 16;
        }
        ctx.fpscr.csr = csr;
        Ok(Snapshot { function, pc_fixes: flags & 1 != 0, delta_time, csr, ctx, page_count, read: 0, file })
    }

    /// The next page into `buf` (SNAPSHOT_PAGE bytes): its 64 KiB page index, or None
    /// after the last.
    pub fn next_page(&mut self, buf: &mut [u8]) -> std::io::Result<Option<u32>> {
        use std::io::Read;
        if self.read == self.page_count {
            return Ok(None);
        }
        let mut idx = [0u8; 4];
        self.file.read_exact(&mut idx)?;
        self.file.read_exact(&mut buf[..SNAPSHOT_PAGE])?;
        self.read += 1;
        Ok(Some(u32::from_le_bytes(idx)))
    }

    /// Reads every remaining page into `mem`.
    pub fn load_into(&mut self, mem: &mut GuestMemory) -> std::io::Result<()> {
        while self.read < self.page_count {
            use std::io::Read;
            let mut idx = [0u8; 4];
            self.file.read_exact(&mut idx)?;
            let p = u32::from_le_bytes(idx);
            self.file.read_exact(mem.bytes_mut(p * SNAPSHOT_PAGE as u32, SNAPSHOT_PAGE))?;
            self.read += 1;
        }
        Ok(())
    }
}

/// ppc_context.h `simde_mm_vctsxs`: float -> int32 with saturation (positive overflow to
/// INT_MAX, NaN to 0), as `vctsxs`.
#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_vctsxs(src1: __m128) -> __m128i {
    unsafe {
        let xmm2 = _mm_cmpunord_ps(src1, src1);
        let xmm0 = _mm_cvttps_epi32(src1);
        let mut xmm1 = _mm_cmpeq_epi32(xmm0, _mm_set1_epi32(i32::MIN));
        xmm1 = _mm_andnot_si128(_mm_castps_si128(src1), xmm1);
        let dest = _mm_blendv_ps(_mm_castsi128_ps(xmm0), _mm_castsi128_ps(_mm_set1_epi32(i32::MAX)), _mm_castsi128_ps(xmm1));
        _mm_andnot_si128(_mm_castps_si128(xmm2), _mm_castps_si128(dest))
    }
}

// ---------------------------------------------------------------- more UnleashedRecomp PC fixes

thread_local! {
    static DELTA_TIME: core::cell::Cell<f64> = const { core::cell::Cell::new(1.0 / 30.0) };
}

/// App::s_deltaTime as the PC fixes read it (the host's frame time).
pub fn set_delta_time(dt: f64) {
    DELTA_TIME.with(|c| c.set(dt));
}

fn delta_time() -> f64 {
    DELTA_TIME.with(|c| c.get())
}

/// A PC-fix hook xenon-rt does not implement was reached with pc_fixes() on.
pub fn unimplemented_hook(name: &str) {
    panic!("UnleashedRecomp mid-asm hook {name} is not implemented in xenon-rt");
}

pub fn unimplemented_bool_hook(name: &str) -> bool {
    panic!("UnleashedRecomp mid-asm hook {name} (bool) is not implemented in xenon-rt");
}

// patches/fps_patches.cpp
fn compute_lerp_factor(t: f64, delta_time: f64) -> f64 {
    let fps = 1.0 / delta_time;
    let bias = t * 60.0;
    1.0 - (1.0 - t).powf((30.0 + bias) / (fps + bias))
}

pub fn CameraDeltaTimeFixMidAsmHook_rr(dest: &mut PPCRegister, src: &mut PPCRegister) {
    unsafe { dest.f64 = src.f64 / 30.0 };
}

pub fn CameraDeltaTimeFixMidAsmHook_r(dest: &mut PPCRegister) {
    unsafe { dest.f64 /= 30.0 };
}

pub fn CameraLerpFixMidAsmHook_r(t: &mut PPCRegister) {
    unsafe { t.f64 = compute_lerp_factor(t.f64, delta_time()) };
}

pub fn CameraTargetSideOffsetLerpFixMidAsmHook_vv(v13: &mut PPCVRegister, v62: &mut PPCVRegister) {
    unsafe {
        let factor = compute_lerp_factor((v13.f32[0] * v62.f32[0]) as f64, delta_time()) as f32;
        for i in 0..4 {
            v62.f32[i] = factor;
            v13.f32[i] = 1.0;
        }
    }
}

pub fn Camera2DLerpFixMidAsmHook_rr(t: &mut PPCRegister, delta_time: &mut PPCRegister) {
    unsafe { t.f64 = compute_lerp_factor(1.0f64.min(t.f64 * 2.0), delta_time.f64 / 60.0) };
}

pub fn Camera2DSlopeLerpFixMidAsmHook_rr(t: &mut PPCRegister, delta_time: &mut PPCRegister) {
    unsafe { t.f64 = compute_lerp_factor(t.f64, delta_time.f64 / 60.0) };
}

/// patches/player_patches.cpp: resets host-side score trackers only.
pub fn ResetScoreOnRestartMidAsmHook_() {}

/// patches/input_patches.cpp: maps the D-pad onto the stick values the posture reads.
/// A host that writes SPadState's sticks itself (and leaves the D-pad bits clear) gets
/// the same result from this no-op.
pub fn PostureDPadSupportInvertYMidAsmHook_rrr(_pad: &mut PPCRegister, _x: &mut PPCRegister, _y: &mut PPCRegister) {}
pub fn PostureDPadSupportXMidAsmHook_rr(_pad: &mut PPCRegister, _x: &mut PPCRegister) {}
pub fn PostureDPadSupportMidAsmHook_rrr(_pad: &mut PPCRegister, _x: &mut PPCRegister, _y: &mut PPCRegister) {}
pub fn PostureDPadSupportYMidAsmHook_rr(_pad: &mut PPCRegister, _y: &mut PPCRegister) {}

/// ppc_context.h `simde_mm_perm_epi8_`: `vperm` on host-order lanes.
#[target_feature(enable = "sse4.1,ssse3")]
#[inline]
pub unsafe fn simde_mm_perm_epi8_(a: __m128i, b: __m128i, c: __m128i) -> __m128i {
    unsafe {
        let d = _mm_set1_epi8(0xF);
        let e = _mm_sub_epi8(d, _mm_and_si128(c, d));
        _mm_blendv_epi8(_mm_shuffle_epi8(a, e), _mm_shuffle_epi8(b, e), _mm_slli_epi32(c, 3))
    }
}

/// ppc_context.h `simde_mm_cvtepu32_ps_`: unsigned int32 -> float (`vcfux`).
#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_cvtepu32_ps_(src1: __m128i) -> __m128 {
    unsafe {
        let xmm1 = _mm_add_epi32(src1, _mm_set1_epi32(127));
        let mut xmm0 = _mm_slli_epi32(src1, 31 - 8);
        xmm0 = _mm_srli_epi32(xmm0, 31);
        xmm0 = _mm_add_epi32(xmm0, xmm1);
        xmm0 = _mm_srai_epi32(xmm0, 8);
        xmm0 = _mm_add_epi32(xmm0, _mm_set1_epi32(0x4F80_0000));
        let xmm2 = _mm_cvtepi32_ps(src1);
        _mm_blendv_ps(xmm2, _mm_castsi128_ps(xmm0), _mm_castsi128_ps(src1))
    }
}

/// patches/input_patches.cpp: Config::HomingAttackOnJump (UnleashedRecomp's default:
/// off, which is also the console's behaviour).
pub fn IsHomingAttackOnJump_() -> bool {
    false
}

// ---------------------------------------------------------------- call tracing

thread_local! {
    static TRACE: core::cell::RefCell<Option<std::collections::BTreeMap<u32, u32>>> = const { core::cell::RefCell::new(None) };
}

/// Starts (or with false stops and returns) counting indirect-call targets on this thread.
pub fn trace_indirect(on: bool) -> Option<std::collections::BTreeMap<u32, u32>> {
    TRACE.with(|t| if on { t.replace(Some(Default::default())) } else { t.take() })
}

#[inline(always)]
pub fn trace_call(address: u32) {
    TRACE.with(|t| {
        if let Some(m) = t.borrow_mut().as_mut() {
            *m.entry(address).or_default() += 1;
        }
    });
}

// ---------------------------------------------------------------- camera PC fixes

thread_local! {
    static GUEST_BASE: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    static ASPECT: core::cell::Cell<f32> = const { core::cell::Cell::new(16.0 / 9.0) };
}

/// Guest memory for hooks that touch it (set by the host before running guest code).
pub fn set_guest_base(base: *mut u8) {
    GUEST_BASE.with(|c| c.set(base as usize));
}

/// The window's aspect ratio (UnleashedRecomp's g_aspectRatio).
pub fn set_aspect_ratio(aspect: f32) {
    ASPECT.with(|c| c.set(aspect));
}

fn guest_base() -> *mut u8 {
    GUEST_BASE.with(|c| c.get()) as *mut u8
}

/// SWA::CCamera::m_HorzAspectRatio (api/SWA/Camera/Camera.h; +0x1D0 in this build).
const CAMERA_HORZ_ASPECT: u32 = 0x1D0;
const WIDE_ASPECT_RATIO: f32 = 16.0 / 9.0;

/// patches/camera_patches.cpp: the projection always follows the window's aspect ratio.
pub fn CameraAspectRatioMidAsmHook_rr(r30: &mut PPCRegister, r31: &mut PPCRegister) {
    unsafe {
        r30.u32 = 0;
        PPC_STORE_U32(guest_base(), r31.u32 + CAMERA_HORZ_ASPECT, ASPECT.with(|c| c.get()).to_bits());
    }
}

/// patches/camera_patches.cpp AdjustFieldOfView (Config::AspectRatio = Auto): VERT+ below 16:9.
pub fn CameraFieldOfViewMidAsmHook_rr(r31: &mut PPCRegister, f31: &mut PPCRegister) {
    unsafe {
        let aspect = f32::from_bits(PPC_LOAD_U32(guest_base(), r31.u32 + CAMERA_HORZ_ASPECT));
        if aspect < WIDE_ASPECT_RATIO {
            f31.f64 = 2.0 * ((0.5 * f31.f64).tan() / aspect as f64 * WIDE_ASPECT_RATIO as f64).atan();
        }
    }
}

// ppc_context.h's remaining custom simde helpers.

#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_cmpgt_epu8(a: __m128i, b: __m128i) -> __m128i {
    unsafe {
        let c = _mm_set1_epi8(-128);
        _mm_cmpgt_epi8(_mm_xor_si128(a, c), _mm_xor_si128(b, c))
    }
}

#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_cmpgt_epu16(a: __m128i, b: __m128i) -> __m128i {
    unsafe {
        let c = _mm_set1_epi16(-32768);
        _mm_cmpgt_epi16(_mm_xor_si128(a, c), _mm_xor_si128(b, c))
    }
}

#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_adds_epu32(a: __m128i, b: __m128i) -> __m128i {
    unsafe { _mm_add_epi32(a, _mm_min_epu32(_mm_xor_si128(a, _mm_cmpeq_epi32(a, a)), b)) }
}

#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_avg_epi8(a: __m128i, b: __m128i) -> __m128i {
    unsafe {
        let c = _mm_set1_epi8(-128);
        _mm_xor_si128(c, _mm_avg_epu8(_mm_xor_si128(c, a), _mm_xor_si128(c, b)))
    }
}

#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_avg_epi16(a: __m128i, b: __m128i) -> __m128i {
    unsafe {
        let c = _mm_set1_epi16(-32768);
        _mm_xor_si128(c, _mm_avg_epu16(_mm_xor_si128(c, a), _mm_xor_si128(c, b)))
    }
}

#[target_feature(enable = "sse4.1")]
#[inline]
pub unsafe fn simde_mm_vsr(a: __m128i, b: __m128i) -> __m128i {
    unsafe {
        let b = _mm_srli_epi64(_mm_slli_epi64(b, 61), 61);
        _mm_castps_si128(_mm_insert_ps(_mm_castsi128_ps(_mm_srl_epi64(a, b)), _mm_castsi128_ps(_mm_srl_epi64(_mm_srli_si128(a, 4), b)), 0x10))
    }
}

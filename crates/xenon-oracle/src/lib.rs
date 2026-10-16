//! Runs original Sonic Unleashed guest functions for differential tests.
//!
//! `re/oracle` builds `xenon_oracle.dll` from the recompiled game (re/xenon/ppc, made
//! from the player's own default.xex): 4 GiB of guest memory with the game image
//! loaded, and a call entry point. A test lays out guest state, calls the original
//! function and compares its result with the Rust port, bit for bit.
//!
//! The DLL and the game are the player's, so [`Oracle::get`] returns `None` when they
//! are not there and tests skip instead of failing.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::Library;

type Init = unsafe extern "C" fn(*const std::ffi::c_char) -> i32;
type Base = unsafe extern "C" fn() -> *mut u8;
type Alloc = unsafe extern "C" fn(u32) -> u32;
type Unit = unsafe extern "C" fn();
type Hits = unsafe extern "C" fn() -> u32;
type CtxSize = unsafe extern "C" fn() -> usize;
type CtxInit = unsafe extern "C" fn(*mut u8);
type Gpr = unsafe extern "C" fn(*mut u8, i32) -> *mut u64;
type Fpr = unsafe extern "C" fn(*mut u8, i32) -> *mut f64;
type Vr = unsafe extern "C" fn(*mut u8, i32) -> *mut f32;
type Call = unsafe extern "C" fn(*mut u8, u32) -> i32;
type MockFn = unsafe extern "C" fn(u32, *mut u8, *mut u8);
type SetMock = unsafe extern "C" fn(u32, Option<MockFn>) -> i32;

/// Registers of the guest call a mock stands in for.
pub struct MockRegs {
    ctx: *mut u8,
    base: *mut u8,
    gpr: Gpr,
    fpr: Fpr,
}

impl MockRegs {
    pub fn r(&self, i: i32) -> u64 {
        unsafe { *(self.gpr)(self.ctx, i).as_ref().expect("register not in context") }
    }
    pub fn set_r(&mut self, i: i32, v: u64) {
        unsafe { *(self.gpr)(self.ctx, i).as_mut().expect("register not in context") = v }
    }
    pub fn f(&self, i: i32) -> f64 {
        unsafe { *(self.fpr)(self.ctx, i).as_ref().expect("register not in context") }
    }
    pub fn set_f(&mut self, i: i32, v: f64) {
        unsafe { *(self.fpr)(self.ctx, i).as_mut().expect("register not in context") = v }
    }
    /// Guest memory, big-endian.
    pub fn read_f32(&self, addr: u32) -> f32 {
        let b = unsafe { std::slice::from_raw_parts(self.base.add(addr as usize), 4) };
        f32::from_be_bytes(b.try_into().unwrap())
    }
    pub fn read_vec4(&self, addr: u32) -> [f32; 4] {
        std::array::from_fn(|i| self.read_f32(addr + 4 * i as u32))
    }
    pub fn write_vec4(&mut self, addr: u32, v: [f32; 4]) {
        for (i, c) in v.into_iter().enumerate() {
            let b = unsafe { std::slice::from_raw_parts_mut(self.base.add(addr as usize + 4 * i), 4) };
            b.copy_from_slice(&c.to_be_bytes());
        }
    }
}

type Mock = Box<dyn FnMut(&mut MockRegs) + Send>;
static MOCKS: Mutex<Vec<(u32, Mock)>> = Mutex::new(Vec::new());
static ACCESS: OnceLock<(Gpr, Fpr)> = OnceLock::new();

unsafe extern "C" fn trampoline(address: u32, ctx: *mut u8, base: *mut u8) {
    let (gpr, fpr) = *ACCESS.get().expect("oracle not loaded");
    let mut regs = MockRegs { ctx, base, gpr, fpr };
    let mut mocks = MOCKS.lock().unwrap_or_else(|e| e.into_inner());
    let (_, mock) = mocks.iter_mut().find(|(a, _)| *a == address).expect("mock dispatched without a closure");
    mock(&mut regs);
}

pub struct Oracle {
    _lib: Library,
    base: *mut u8,
    alloc: Alloc,
    reset_heap: Unit,
    hits: Hits,
    ctx_init: CtxInit,
    gpr: Gpr,
    fpr: Fpr,
    vr: Vr,
    call: Call,
    set_mock: SetMock,
    clear_mocks: Unit,
    ctx: Vec<u8>,
}

// The guest memory is only touched through the mutex in `get`.
unsafe impl Send for Oracle {}

fn rs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

impl Oracle {
    /// The process-wide oracle, or `None` when the DLL or the patched xex is missing
    /// (build re/xenon then re/oracle). Tests hold the guard for their whole run.
    /// With `UNLEASHED_REQUIRE_ORACLE=1` a missing oracle panics instead (CI, or any run
    /// whose point is the comparison).
    pub fn get() -> Option<MutexGuard<'static, Oracle>> {
        static ORACLE: OnceLock<Option<Mutex<Oracle>>> = OnceLock::new();
        let oracle = ORACLE.get_or_init(|| Self::load().map(Mutex::new)).as_ref();
        if oracle.is_none() && std::env::var("UNLEASHED_REQUIRE_ORACLE").is_ok_and(|v| v == "1") {
            panic!("xenon oracle required but missing: build re/xenon (build.bat + XenonRecomp) and re/oracle (build.bat)");
        }
        oracle.map(|m| {
            let mut o = m.lock().unwrap_or_else(|e| e.into_inner());
            o.reset();
            o
        })
    }

    fn load() -> Option<Self> {
        let root = rs_root();
        let dll = root.join("re/oracle/build/xenon_oracle.dll");
        let xex = root.join("re/xenon/ppc/default_patched.xex");
        if !dll.is_file() || !xex.is_file() {
            return None;
        }
        unsafe {
            let lib = Library::new(&dll).ok()?;
            let init: Init = *lib.get(b"oracle_init").ok()?;
            let path = std::ffi::CString::new(xex.to_str()?).ok()?;
            assert_eq!(init(path.as_ptr()), 0, "oracle_init failed");
            let base: Base = *lib.get(b"oracle_base").ok()?;
            let size: CtxSize = *lib.get(b"oracle_context_size").ok()?;
            let o = Self {
                base: base(),
                alloc: *lib.get(b"oracle_alloc").ok()?,
                reset_heap: *lib.get(b"oracle_reset_heap").ok()?,
                hits: *lib.get(b"oracle_import_hits").ok()?,
                ctx_init: *lib.get(b"oracle_context_init").ok()?,
                gpr: *lib.get(b"oracle_gpr").ok()?,
                fpr: *lib.get(b"oracle_fpr").ok()?,
                vr: *lib.get(b"oracle_vr").ok()?,
                call: *lib.get(b"oracle_call").ok()?,
                set_mock: *lib.get(b"oracle_set_mock").ok()?,
                clear_mocks: *lib.get(b"oracle_clear_mocks").ok()?,
                // PPCContext is alignas(0x40); over-allocate and align by hand.
                ctx: vec![0; size() + 0x40],
                _lib: lib,
            };
            let _ = ACCESS.set((o.gpr, o.fpr));
            Some(o)
        }
    }

    fn ctx_ptr(&mut self) -> *mut u8 {
        let p = self.ctx.as_mut_ptr();
        unsafe { p.add(p.align_offset(0x40)) }
    }

    /// Fresh registers (r1 = stack, r13 = thread block), an empty test heap, no mocks.
    pub fn reset(&mut self) {
        MOCKS.lock().unwrap_or_else(|e| e.into_inner()).clear();
        unsafe {
            (self.clear_mocks)();
            (self.reset_heap)();
            let c = self.ctx_ptr();
            (self.ctx_init)(c);
        }
    }

    /// Host address of guest address 0 (the lifted Rust runs on this same memory).
    pub fn base(&self) -> *mut u8 {
        self.base
    }

    /// Kernel/XAM imports reached so far; a test should see 0.
    pub fn import_hits(&self) -> u32 {
        unsafe { (self.hits)() }
    }

    /// Zeroed guest memory, 16-aligned.
    pub fn alloc(&mut self, size: u32) -> u32 {
        unsafe { (self.alloc)(size) }
    }

    fn at(&self, addr: u32, len: usize) -> *mut u8 {
        assert!(addr >= 0x1000 && (addr as u64 + len as u64) <= 1 << 32);
        unsafe { self.base.add(addr as usize) }
    }

    pub fn write(&mut self, addr: u32, bytes: &[u8]) {
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.at(addr, bytes.len()), bytes.len()) }
    }

    pub fn read(&self, addr: u32, len: usize) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.at(addr, len), len).to_vec() }
    }

    pub fn write_u8(&mut self, addr: u32, v: u8) {
        self.write(addr, &[v]);
    }
    pub fn write_u32(&mut self, addr: u32, v: u32) {
        self.write(addr, &v.to_be_bytes());
    }
    pub fn write_f32(&mut self, addr: u32, v: f32) {
        self.write(addr, &v.to_be_bytes());
    }
    pub fn write_vec4(&mut self, addr: u32, v: [f32; 4]) {
        for (i, x) in v.into_iter().enumerate() {
            self.write_f32(addr + 4 * i as u32, x);
        }
    }
    pub fn read_u32(&self, addr: u32) -> u32 {
        u32::from_be_bytes(self.read(addr, 4).try_into().unwrap())
    }
    pub fn read_f32(&self, addr: u32) -> f32 {
        f32::from_be_bytes(self.read(addr, 4).try_into().unwrap())
    }
    pub fn read_vec4(&self, addr: u32) -> [f32; 4] {
        std::array::from_fn(|i| self.read_f32(addr + 4 * i as u32))
    }

    pub fn set_r(&mut self, i: i32, v: u64) {
        let c = self.ctx_ptr();
        unsafe { *(self.gpr)(c, i).as_mut().expect("register not in context") = v }
    }
    pub fn r(&mut self, i: i32) -> u64 {
        let c = self.ctx_ptr();
        unsafe { *(self.gpr)(c, i).as_ref().expect("register not in context") }
    }
    pub fn set_f(&mut self, i: i32, v: f64) {
        let c = self.ctx_ptr();
        unsafe { *(self.fpr)(c, i).as_mut().expect("register not in context") = v }
    }
    pub fn f(&mut self, i: i32) -> f64 {
        let c = self.ctx_ptr();
        unsafe { *(self.fpr)(c, i).as_ref().expect("register not in context") }
    }
    /// Vector register as guest elements x, y, z, w (the DLL stores them reversed).
    pub fn v(&mut self, i: i32) -> [f32; 4] {
        let c = self.ctx_ptr();
        let lanes = unsafe { std::slice::from_raw_parts((self.vr)(c, i), 4) };
        [lanes[3], lanes[2], lanes[1], lanes[0]]
    }

    /// Replaces the boundary function at `address` (sheets/boundaries.csv) with `mock`
    /// for the rest of this test.
    pub fn mock(&mut self, address: u32, mock: impl FnMut(&mut MockRegs) + Send + 'static) {
        let mut mocks = MOCKS.lock().unwrap_or_else(|e| e.into_inner());
        mocks.retain(|(a, _)| *a != address);
        mocks.push((address, Box::new(mock)));
        drop(mocks);
        assert_eq!(unsafe { (self.set_mock)(address, Some(trampoline)) }, 0, "{address:#010X} is not in sheets/boundaries.csv");
    }

    /// Runs the guest function at `address` with the current registers.
    pub fn call(&mut self, address: u32) {
        let c = self.ctx_ptr();
        assert_eq!(unsafe { (self.call)(c, address) }, 0, "no guest function at {address:#010X}");
    }
}

//! The host side UnleashedRecomp puts under the game (kernel/heap.cpp, kernel/imports.cpp),
//! so lifted code running on a capture-build snapshot sees the same heap and locks.
//!
//! The heap is o1heap 2.x (thirdparty/o1heap) as UnleashedRecomp builds it on x86-64:
//! O1HEAP_ALIGNMENT 32, fragment headers {next, prev, size: u64, used: bool} holding
//! *host* pointers, the user heap's instance at guest 0x20000 and the physical heap's at
//! 0xA0000000. A snapshot's links point into the capture build's `g_memory.base`;
//! `Kernel::from_memory` recovers that base so allocation continues exactly as it would
//! have in the game (same addresses).

use crate::{PPCContext, PPC_LOAD_U32, PPC_STORE_U32};

const ALIGNMENT: u64 = 32;
const FRAGMENT_SIZE_MIN: u64 = ALIGNMENT * 2;
const NUM_BINS: u64 = 64;
// bins[64] + nonempty_bin_mask + diagnostics {capacity, allocated, peak_allocated,
// peak_request_size, oom_count}, padded to ALIGNMENT.
const INSTANCE_SIZE_PADDED: u64 = (NUM_BINS * 8 + 8 + 5 * 8 + ALIGNMENT - 1) & !(ALIGNMENT - 1);
const OFF_MASK: u64 = NUM_BINS * 8;
const OFF_CAPACITY: u64 = OFF_MASK + 8;
const OFF_ALLOCATED: u64 = OFF_CAPACITY + 8;
const OFF_PEAK_ALLOCATED: u64 = OFF_ALLOCATED + 8;
const OFF_PEAK_REQUEST: u64 = OFF_PEAK_ALLOCATED + 8;
const OFF_OOM: u64 = OFF_PEAK_REQUEST + 8;
// Fragment: header.next 0, header.prev 8, header.size 16, header.used 24; next_free 32,
// prev_free 40.
const F_NEXT: u64 = 0;
const F_PREV: u64 = 8;
const F_SIZE: u64 = 16;
const F_USED: u64 = 24;
const F_NEXT_FREE: u64 = 32;
const F_PREV_FREE: u64 = 40;

pub const USER_HEAP: u32 = 0x20000;
pub const PHYSICAL_HEAP: u32 = 0xA000_0000;

/// One o1heap instance in guest memory. Pointers are host addresses of the capture build:
/// guest address = pointer - `host_base`. 0 is NULL.
struct O1Heap {
    base: *mut u8,
    host_base: u64,
    instance: u64, // host pointer
}

impl O1Heap {
    fn at(&self, p: u64) -> *mut u8 {
        self.base.wrapping_add((p - self.host_base) as usize)
    }
    fn rd(&self, p: u64) -> u64 {
        unsafe { (self.at(p) as *const u64).read_unaligned() }
    }
    fn wr(&self, p: u64, v: u64) {
        unsafe { (self.at(p) as *mut u64).write_unaligned(v) }
    }
    fn used(&self, f: u64) -> bool {
        unsafe { *self.at(f + F_USED) != 0 }
    }
    fn set_used(&self, f: u64, u: bool) {
        unsafe { *self.at(f + F_USED) = u as u8 }
    }

    fn log2_floor(x: u64) -> u32 {
        63 - x.leading_zeros()
    }
    fn log2_ceil(x: u64) -> u32 {
        if x <= 1 { 0 } else { 64 - (x - 1).leading_zeros() }
    }
    fn round_up_pow2(x: u64) -> u64 {
        1u64 << (64 - (x - 1).leading_zeros())
    }

    fn interlink(&self, left: u64, right: u64) {
        if left != 0 {
            self.wr(left + F_NEXT, right);
        }
        if right != 0 {
            self.wr(right + F_PREV, left);
        }
    }

    fn rebin(&self, f: u64) {
        let h = self.instance;
        let idx = Self::log2_floor(self.rd(f + F_SIZE) / FRAGMENT_SIZE_MIN) as u64;
        let head = self.rd(h + idx * 8);
        self.wr(f + F_NEXT_FREE, head);
        self.wr(f + F_PREV_FREE, 0);
        if head != 0 {
            self.wr(head + F_PREV_FREE, f);
        }
        self.wr(h + idx * 8, f);
        self.wr(h + OFF_MASK, self.rd(h + OFF_MASK) | 1 << idx);
    }

    fn unbin(&self, f: u64) {
        let h = self.instance;
        let idx = Self::log2_floor(self.rd(f + F_SIZE) / FRAGMENT_SIZE_MIN) as u64;
        let (nf, pf) = (self.rd(f + F_NEXT_FREE), self.rd(f + F_PREV_FREE));
        if nf != 0 {
            self.wr(nf + F_PREV_FREE, pf);
        }
        if pf != 0 {
            self.wr(pf + F_NEXT_FREE, nf);
        }
        if self.rd(h + idx * 8) == f {
            self.wr(h + idx * 8, nf);
            if nf == 0 {
                self.wr(h + OFF_MASK, self.rd(h + OFF_MASK) & !(1 << idx));
            }
        }
    }

    /// o1heapAllocate: a host pointer, or 0.
    fn allocate(&self, amount: u64) -> u64 {
        let h = self.instance;
        let capacity = self.rd(h + OFF_CAPACITY);
        let mut out = 0;
        if amount > 0 && amount <= capacity - ALIGNMENT {
            let fragment_size = Self::round_up_pow2(amount + ALIGNMENT);
            let optimal = Self::log2_ceil(fragment_size / FRAGMENT_SIZE_MIN);
            let candidate = !((1u64 << optimal) - 1);
            let suitable = self.rd(h + OFF_MASK) & candidate;
            let smallest = suitable & !(suitable.wrapping_sub(1));
            if smallest != 0 {
                let bin = Self::log2_floor(smallest) as u64;
                let frag = self.rd(h + bin * 8);
                self.unbin(frag);
                let leftover = self.rd(frag + F_SIZE) - fragment_size;
                self.wr(frag + F_SIZE, fragment_size);
                if leftover >= FRAGMENT_SIZE_MIN {
                    let nf = frag + fragment_size;
                    self.wr(nf + F_SIZE, leftover);
                    self.set_used(nf, false);
                    self.interlink(nf, self.rd(frag + F_NEXT));
                    self.interlink(frag, nf);
                    self.rebin(nf);
                }
                let allocated = self.rd(h + OFF_ALLOCATED) + fragment_size;
                self.wr(h + OFF_ALLOCATED, allocated);
                if self.rd(h + OFF_PEAK_ALLOCATED) < allocated {
                    self.wr(h + OFF_PEAK_ALLOCATED, allocated);
                }
                self.set_used(frag, true);
                out = frag + ALIGNMENT;
            }
        }
        if self.rd(h + OFF_PEAK_REQUEST) < amount {
            self.wr(h + OFF_PEAK_REQUEST, amount);
        }
        if out == 0 && amount > 0 {
            self.wr(h + OFF_OOM, self.rd(h + OFF_OOM) + 1);
        }
        out
    }

    /// o1heapFree.
    fn free(&self, pointer: u64) {
        if pointer == 0 {
            return;
        }
        let h = self.instance;
        let frag = pointer - ALIGNMENT;
        self.set_used(frag, false);
        let size = self.rd(frag + F_SIZE);
        self.wr(h + OFF_ALLOCATED, self.rd(h + OFF_ALLOCATED) - size);
        let (prev, next) = (self.rd(frag + F_PREV), self.rd(frag + F_NEXT));
        let join_left = prev != 0 && !self.used(prev);
        let join_right = next != 0 && !self.used(next);
        if join_left && join_right {
            self.unbin(prev);
            self.unbin(next);
            self.wr(prev + F_SIZE, self.rd(prev + F_SIZE) + size + self.rd(next + F_SIZE));
            self.wr(frag + F_SIZE, 0);
            self.wr(next + F_SIZE, 0);
            self.interlink(prev, self.rd(next + F_NEXT));
            self.rebin(prev);
        } else if join_left {
            self.unbin(prev);
            self.wr(prev + F_SIZE, self.rd(prev + F_SIZE) + size);
            self.wr(frag + F_SIZE, 0);
            self.interlink(prev, next);
            self.rebin(prev);
        } else if join_right {
            self.unbin(next);
            self.wr(frag + F_SIZE, size + self.rd(next + F_SIZE));
            self.wr(next + F_SIZE, 0);
            self.interlink(frag, self.rd(next + F_NEXT));
            self.rebin(frag);
        } else {
            self.rebin(frag);
        }
    }
}

/// UnleashedRecomp's kernel services on one guest memory.
pub struct Kernel {
    base: *mut u8,
    host_base: u64,
    /// KeTlsGetValue / KeTlsSetValue slots (UnleashedRecomp keeps them host-side, so a
    /// snapshot does not carry them: they start empty).
    tls: std::cell::RefCell<std::collections::HashMap<u32, u32>>,
}

impl Kernel {
    /// For memory restored from a capture-build snapshot: recovers the build's
    /// `g_memory.base` from the user heap's first fragment (its `next` link is the host
    /// address of the fragment right after it).
    pub fn from_memory(base: *mut u8) -> Option<Self> {
        let first = USER_HEAP as u64 + INSTANCE_SIZE_PADDED;
        let rd = |g: u64| unsafe { (base.add(g as usize) as *const u64).read_unaligned() };
        let (next, size) = (rd(first + F_NEXT), rd(first + F_SIZE));
        if next == 0 || size == 0 {
            return None;
        }
        let host_base = next.checked_sub(first + size)?;
        Some(Kernel { base, host_base, tls: Default::default() })
    }

    /// Fresh memory: initialises both heaps as Heap::Init does (host base = `base`).
    pub fn init(base: *mut u8) -> Self {
        let k = Kernel { base, host_base: base as u64, tls: Default::default() };
        k.o1init(USER_HEAP, 0x7FEA_0000 - USER_HEAP as u64);
        k.o1init(PHYSICAL_HEAP, 0x1_0000_0000 - PHYSICAL_HEAP as u64);
        k
    }

    fn o1init(&self, guest: u32, size: u64) {
        let h = self.heap(guest);
        for i in 0..NUM_BINS {
            h.wr(h.instance + i * 8, 0);
        }
        h.wr(h.instance + OFF_MASK, 0);
        let mut capacity = (size - INSTANCE_SIZE_PADDED).min((u64::MAX >> 1) + 1);
        capacity -= capacity % FRAGMENT_SIZE_MIN;
        let f = h.instance + INSTANCE_SIZE_PADDED;
        for o in [F_NEXT, F_PREV, F_NEXT_FREE, F_PREV_FREE] {
            h.wr(f + o, 0);
        }
        h.wr(f + F_SIZE, capacity);
        h.set_used(f, false);
        h.rebin(f);
        h.wr(h.instance + OFF_CAPACITY, capacity);
        for o in [OFF_ALLOCATED, OFF_PEAK_ALLOCATED, OFF_PEAK_REQUEST, OFF_OOM] {
            h.wr(h.instance + o, 0);
        }
    }

    fn heap(&self, guest: u32) -> O1Heap {
        O1Heap { base: self.base, host_base: self.host_base, instance: self.host_base + guest as u64 }
    }

    fn guest(&self, host: u64) -> u32 {
        (host - self.host_base) as u32
    }

    fn host(&self, guest: u32) -> u64 {
        self.host_base + guest as u64
    }

    /// Heap::Alloc.
    pub fn alloc(&self, size: u32) -> u32 {
        let p = self.heap(USER_HEAP).allocate((size as u64).max(1));
        assert!(p != 0, "guest heap out of memory");
        self.guest(p)
    }

    /// Heap::AllocPhysical.
    pub fn alloc_physical(&self, size: u32, alignment: u32) -> u32 {
        let size = (size as u64).max(1);
        let alignment = if alignment == 0 { 0x1000 } else { (alignment as u64).max(16) };
        let ptr = self.heap(PHYSICAL_HEAP).allocate(size + alignment);
        assert!(ptr != 0, "guest physical heap out of memory");
        let aligned = (ptr + alignment) & !(alignment - 1);
        let h = self.heap(PHYSICAL_HEAP);
        h.wr(aligned - 8, ptr);
        h.wr(aligned - 16, size + ALIGNMENT);
        self.guest(aligned)
    }

    /// Heap::Free.
    pub fn free(&self, guest: u32) {
        if guest >= PHYSICAL_HEAP {
            let h = self.heap(PHYSICAL_HEAP);
            let p = h.rd(self.host(guest) - 8);
            h.free(p);
        } else {
            self.heap(USER_HEAP).free(self.host(guest));
        }
    }

    /// Heap::Size.
    pub fn size(&self, guest: u32) -> u32 {
        if guest == 0 {
            return 0;
        }
        (self.heap(USER_HEAP).rd(self.host(guest) - 16) - ALIGNMENT) as u32
    }

    fn zero(&self, guest: u32, n: u32) {
        unsafe { core::ptr::write_bytes(self.base.add(guest as usize), 0, n as usize) }
    }

    /// A function UnleashedRecomp replaces with host code (sheets/host_functions.csv).
    /// Arguments in r3.., result in r3, as HostToGuestFunction marshals them. False if
    /// this kernel does not implement `address`.
    pub fn hook(&self, address: u32, ctx: &mut PPCContext, base: *mut u8) -> bool {
        let (r3, r4, r5, r6) = unsafe { (ctx.r3.u32, ctx.r4.u32, ctx.r5.u32, ctx.r6.u32) };
        let ret: u32 = match address {
            // RtlAllocateHeap(heap, flags, size)
            0x82BD7D30 => {
                let p = self.alloc(r5);
                if r4 & 8 != 0 {
                    self.zero(p, r5);
                }
                p
            }
            // RtlFreeHeap(heap, flags, pointer)
            0x82BD8600 => {
                if r5 != 0 {
                    self.free(r5);
                }
                1
            }
            // RtlReAllocateHeap(heap, flags, pointer, size)
            0x82BD88F0 => {
                let p = self.alloc(r6);
                if r4 & 8 != 0 {
                    self.zero(p, r6);
                }
                if r5 != 0 {
                    let n = r6.min(self.size(r5));
                    unsafe { core::ptr::copy(base.add(r5 as usize), base.add(p as usize), n as usize) };
                    self.free(r5);
                }
                p
            }
            // RtlSizeHeap(heap, flags, pointer)
            0x82BD6FD0 => self.size(r5),
            // XAllocMem(size, flags)
            0x831CC9C8 => {
                let p = if r4 & 0x8000_0000 != 0 { self.alloc_physical(r3, 1 << ((r4 >> 24) & 0xF)) } else { self.alloc(r3) };
                if r4 & 0x4000_0000 != 0 {
                    self.zero(p, r3);
                }
                p
            }
            // XFreeMem(address, flags)
            0x831CCA60 => {
                if r3 != 0 {
                    self.free(r3);
                }
                return true;
            }
            // OutputDebugStringA
            0x82BD4CA8 => return true,
            // memcpy (every CRT copy), memmove(dst, src, n): returns dst
            0x831B0ED0 | 0x831CCB98 | 0x831CEAE0 | 0x831CEE04 | 0x831CF2D0 | 0x831CF660 | 0x831B1358 | 0x831B5E00 => {
                unsafe { core::ptr::copy(base.add(r4 as usize), base.add(r3 as usize), r5 as usize) };
                r3
            }
            // memset(dst, c, n): returns dst
            0x831B0BA0 | 0x831CCAA0 => {
                unsafe { core::ptr::write_bytes(base.add(r3 as usize), r4 as u8, r5 as usize) };
                r3
            }
            _ => return false,
        };
        ctx.r3.u64 = ret as u64;
        true
    }

    /// Kernel imports UnleashedRecomp implements that the lifted code reaches. False if
    /// not implemented here.
    pub fn import(&self, name: &str, ctx: &mut PPCContext, base: *mut u8) -> bool {
        // XRTL_CRITICAL_SECTION: 16-byte header, then LockCount, RecursionCount,
        // OwningThread in host (little-endian) order.
        let cs = unsafe { ctx.r3.u32 };
        let thread = unsafe { ctx.r13.u32 };
        let rec = |d: i32| unsafe {
            let p = base.add(cs as usize + 20) as *mut i32;
            p.write_unaligned(p.read_unaligned() + d);
        };
        let owner = unsafe { base.add(cs as usize + 24) as *mut u32 };
        match name {
            "RtlEnterCriticalSection" | "RtlTryEnterCriticalSection" => {
                let prev = unsafe { owner.read_unaligned() };
                // Single guest thread here: a lock held by another thread was taken in
                // the snapshot by a thread that is not running; report it like a failed try.
                let ok = prev == 0 || prev == thread;
                if ok {
                    unsafe { owner.write_unaligned(thread) };
                    rec(1);
                }
                if name == "RtlTryEnterCriticalSection" {
                    unsafe { ctx.r3.u64 = ok as u64 };
                } else {
                    assert!(ok, "RtlEnterCriticalSection({cs:#X}) held by thread {prev:#X}");
                }
                true
            }
            "KeTlsGetValue" => {
                let v = self.tls.borrow().get(&cs).copied().unwrap_or(0);
                unsafe { ctx.r3.u64 = v as u64 };
                true
            }
            "KeTlsSetValue" => {
                self.tls.borrow_mut().insert(cs, unsafe { ctx.r4.u32 });
                unsafe { ctx.r3.u64 = 1 };
                true
            }
            "RtlLeaveCriticalSection" => {
                rec(-1);
                if unsafe { (base.add(cs as usize + 20) as *const i32).read_unaligned() } == 0 {
                    unsafe { owner.write_unaligned(0) };
                }
                true
            }
            _ => false,
        }
    }
}

/// Reads a big-endian guest word (convenience for hosts).
pub unsafe fn load32(base: *mut u8, a: u32) -> u32 {
    unsafe { PPC_LOAD_U32(base, a) }
}

pub unsafe fn store32(base: *mut u8, a: u32, v: u32) {
    unsafe { PPC_STORE_U32(base, a, v) }
}

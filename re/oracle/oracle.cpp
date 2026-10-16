// Xenon oracle: the recompiled Sonic Unleashed code (re/xenon/ppc, from the player's own
// default.xex) as a DLL, so Rust tests can run an original guest function on a state
// they set up and compare the port's result with it, bit for bit.
//
// Guest memory is 4 GiB like UnleashedRecomp's (kernel/memory.cpp): the patched image
// at 0x82000000, the indirect-call table after it, and two regions the tests own:
//   0x40000000..  bump heap for test objects   (oracle_alloc)
//   0x70000000..  call stack (r1 starts at the top)
//   0x7E000000..  r13 thread block (zeroed)

#include "ppc_recomp_shared.h"
#include <image.h>

#include <atomic>
#include <cstdio>
#include <cstring>
#include <memory>
#include <vector>
#include <windows.h>

extern "C" std::atomic<uint32_t> g_oracleImportHits{0};

static uint8_t* g_base;
static uint32_t g_heapNext = 0x40000000;
static constexpr uint32_t StackBase = 0x70000000, StackSize = 0x01000000, ThreadBlock = 0x7E000000;

#define EXPORT extern "C" __declspec(dllexport)

// 0 on success.
EXPORT int oracle_init(const char* patchedXexPath)
{
    if (g_base)
        return 0;
    g_base = (uint8_t*)VirtualAlloc(nullptr, PPC_MEMORY_SIZE, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);
    if (!g_base)
        return 1;

    FILE* f = fopen(patchedXexPath, "rb");
    if (!f)
        return 2;
    fseek(f, 0, SEEK_END);
    std::vector<uint8_t> data(ftell(f));
    fseek(f, 0, SEEK_SET);
    fread(data.data(), 1, data.size(), f);
    fclose(f);

    Image image = Image::ParseImage(data.data(), data.size());
    if (!image.data || image.base != PPC_IMAGE_BASE)
        return 3;
    memcpy(g_base + image.base, image.data.get(), image.size);

    for (size_t i = 0; PPCFuncMappings[i].guest != 0; i++)
        PPC_LOOKUP_FUNC(g_base, PPCFuncMappings[i].guest) = PPCFuncMappings[i].host;
    return 0;
}

EXPORT uint8_t* oracle_base() { return g_base; }

// Bump-allocates `size` zeroed guest bytes (16-aligned); returns the guest address.
EXPORT uint32_t oracle_alloc(uint32_t size)
{
    uint32_t a = (g_heapNext + 15) & ~15u;
    g_heapNext = a + size;
    memset(g_base + a, 0, size);
    return a;
}

EXPORT void oracle_reset_heap() { g_heapNext = 0x40000000; }

EXPORT uint32_t oracle_import_hits() { return g_oracleImportHits.load(); }

EXPORT size_t oracle_context_size() { return sizeof(PPCContext); }

// A fresh context: zeroed, r1 at the top of the oracle stack, r13 at a zeroed block.
EXPORT void oracle_context_init(PPCContext* ctx)
{
    memset(ctx, 0, sizeof(PPCContext));
    memset(g_base + StackBase, 0, StackSize);
    memset(g_base + ThreadBlock, 0, 0x10000);
    ctx->r1.u64 = StackBase + StackSize - 0x100;
    ctx->r13.u64 = ThreadBlock;
    ctx->fpscr.loadFromHost(); // as cpu/guest_thread.cpp does for a new guest thread
}

// Register access. With SWA.toml's *_as_local options only argument/volatile registers
// live in the context (r1, r3-r10, r13, f1-f13, v0-v13); anything else returns null.
EXPORT uint64_t* oracle_gpr(PPCContext* ctx, int i)
{
    switch (i)
    {
    case 1: return &ctx->r1.u64;
    case 3: return &ctx->r3.u64;
    case 4: return &ctx->r4.u64;
    case 5: return &ctx->r5.u64;
    case 6: return &ctx->r6.u64;
    case 7: return &ctx->r7.u64;
    case 8: return &ctx->r8.u64;
    case 9: return &ctx->r9.u64;
    case 10: return &ctx->r10.u64;
    case 13: return &ctx->r13.u64;
    default: return nullptr;
    }
}

EXPORT double* oracle_fpr(PPCContext* ctx, int i)
{
    PPCRegister* f[] = { nullptr, &ctx->f1, &ctx->f2, &ctx->f3, &ctx->f4, &ctx->f5, &ctx->f6, &ctx->f7,
        &ctx->f8, &ctx->f9, &ctx->f10, &ctx->f11, &ctx->f12, &ctx->f13 };
    return i >= 1 && i <= 13 ? &f[i]->f64 : nullptr;
}

// Vector lanes in host order: XenonRecomp keeps guest element 0 in lane 3.
EXPORT float* oracle_vr(PPCContext* ctx, int i)
{
    PPCVRegister* v[] = { &ctx->v0, &ctx->v1, &ctx->v2, &ctx->v3, &ctx->v4, &ctx->v5, &ctx->v6,
        &ctx->v7, &ctx->v8, &ctx->v9, &ctx->v10, &ctx->v11, &ctx->v12, &ctx->v13 };
    return i >= 0 && i <= 13 ? v[i]->f32 : nullptr;
}

using OracleMock = void (*)(uint32_t address, PPCContext* ctx, uint8_t* base);
extern "C" OracleMock g_oracleMocks[];
extern "C" const uint32_t g_oracleMockAddresses[];
extern "C" const uint32_t g_oracleMockCount;
OracleMock g_oracleMocks[64];

// Installs (or with null removes) the mock of a boundary function from
// sheets/boundaries.csv. 0 on success, 1 if `address` is not a boundary.
EXPORT int oracle_set_mock(uint32_t address, OracleMock mock)
{
    for (uint32_t i = 0; i < g_oracleMockCount; i++)
        if (g_oracleMockAddresses[i] == address)
        {
            g_oracleMocks[i] = mock;
            return 0;
        }
    return 1;
}

EXPORT void oracle_clear_mocks() { memset(g_oracleMocks, 0, sizeof(g_oracleMocks)); }

// Runs the guest function at `address`. 0 on success, 1 if there is no such function.
EXPORT int oracle_call(PPCContext* ctx, uint32_t address)
{
    if (address < PPC_CODE_BASE || address >= PPC_CODE_BASE + PPC_CODE_SIZE)
        return 1;
    PPCFunc* fn = PPC_LOOKUP_FUNC(g_base, address);
    if (!fn)
        return 1;
    // The guest code switches the host MXCSR (flush-to-zero around vector code); give
    // the caller its own mode back so the Rust side of a comparison is unaffected.
    // The context caches MXCSR (enable/disableFlushMode skip the write when the cache
    // says it is set): resync it with the host, as a running guest thread always is.
    const uint32_t csr = _mm_getcsr();
    ctx->fpscr.csr = csr;
    fn(*ctx, g_base);
    _mm_setcsr(csr);
    return 0;
}

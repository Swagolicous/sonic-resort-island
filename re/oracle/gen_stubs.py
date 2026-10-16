"""Generate stubs.cpp: host-side definitions the recompiled code links against.

re/xenon/ppc comes from the unmodified SWA.toml, so its mid-asm hooks are
UnleashedRecomp's PC fixes (frame-rate, input, widescreen). The oracle reproduces the
Xbox 360 game, so every hook is a no-op (bool hooks return false: keep the original
control flow). Kernel/XAM imports count their hits and return 0; a test that reaches
one is outside what the oracle models and should say so.

usage: python re/oracle/gen_stubs.py
"""
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
PPC = HERE.parents[1] / "re" / "xenon" / "ppc"


def main():
    hooks, imports = {}, set()
    for p in sorted(PPC.glob("ppc_recomp.*.cpp")):
        text = p.read_text(encoding="utf-8", errors="replace")
        for ret, name, params in re.findall(r"^extern (void|bool) (\w+)\(([^)]*)\);$", text, re.M):
            # Hooks are overloaded on their register lists: key by the parameter types.
            types = tuple(t.rsplit(" ", 1)[0].strip() for t in params.split(",") if t.strip())
            hooks.setdefault((name, types), (ret, params))
    shared = (PPC / "ppc_recomp_shared.h").read_text(encoding="utf-8", errors="replace")
    imports = sorted(set(re.findall(r"PPC_EXTERN_FUNC\((__imp__(?!sub_)\w+)\);", shared)))
    out = ['#include "ppc_recomp_shared.h"', "#include <atomic>", "",
           'extern "C" std::atomic<uint32_t> g_oracleImportHits;', "",
           "// Mid-asm hooks: UnleashedRecomp's PC fixes, absent on the original console."]
    for (name, _), (ret, params) in sorted(hooks.items()):
        out.append(f"{ret} {name}({params}) {{{' return false;' if ret == 'bool' else ''} }}")
    out += ["", "// Kernel / XAM imports: counted, return 0."]
    for name in imports:
        out.append(f"PPC_FUNC({name}) {{ ++g_oracleImportHits; ctx.r3.u64 = 0; }}")
    # Boundaries (sheets/boundaries.csv): strong overrides of the weak guest functions
    # that hand the call to a test's mock when one is installed.
    import csv
    with open(HERE.parents[1] / "sheets" / "boundaries.csv", encoding="utf-8-sig", newline="") as fh:
        bounds = [int(r["address"], 16) for r in csv.DictReader(fh)]
    out += ["", "// Boundaries: a mock installed with oracle_set_mock replaces the guest function.",
            "using OracleMock = void (*)(uint32_t address, PPCContext* ctx, uint8_t* base);",
            "extern \"C\" OracleMock g_oracleMocks[];",
            f"extern \"C\" const uint32_t g_oracleMockAddresses[{max(len(bounds), 1)}] = {{ {', '.join(f'0x{a:08X}' for a in bounds) or '0'} }};",
            f"extern \"C\" const uint32_t g_oracleMockCount = {len(bounds)};"]
    for i, a in enumerate(bounds):
        out += [f"PPC_FUNC_IMPL(__imp__sub_{a:08X});",
                f"PPC_FUNC(sub_{a:08X}) {{ if (auto m = g_oracleMocks[{i}]) m(0x{a:08X}, &ctx, base); else __imp__sub_{a:08X}(ctx, base); }}"]
    (HERE / "stubs.cpp").write_text("\n".join(out) + "\n", encoding="utf-8")
    print(f"stubs.cpp: {len(hooks)} hooks, {len(imports)} imports")


if __name__ == "__main__":
    main()

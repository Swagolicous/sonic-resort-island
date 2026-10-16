"""Lift additional roots with the existing exact PPC converter, without relifting all code.

First record roots with add_roots.py. A later sheets/sheetc.py lift produces the
same functions from that source of truth. This only updates affected modules,
direct calls to newly available functions, and the guest dispatch table.
"""
import csv
import re
import sys
from pathlib import Path

from ppc2rust import Lifter, RS, write, partition_dispatch


def main():
    folder = RS / "crates/unleashed-guest/src"
    lib = (folder / "lib.rs").read_text(encoding="utf-8")
    old = set(re.findall(r"0x([0-9A-F]{8}),", lib.split("pub const LIFTED:", 1)[1]))
    roots = [a.upper().removeprefix("0X") for a in sys.argv[1:]]
    lifter = Lifter()
    new = lifter.closure(roots) - old
    lifter.lifted = old | new
    # Complete conversion before modifying any generated source. Unsupported
    # instructions stop here; they are never approximated.
    bodies = {a: lifter.function(a) for a in sorted(new)}
    for path in folder.glob("f*.rs"):
        source = path.read_text(encoding="utf-8")
        updated = re.sub(r"host_call\(\|h\| h\.missing\(0x([0-9A-F]{8}), ctx, base\)\);",
            lambda m: f"crate::sub_{m[1]}(ctx, base);" if m[1] in new else m[0], source)
        write(path, updated)
    modules = sorted({f"f{a[:4]}" for a in new})
    for module in modules:
        path = folder / f"{module}.rs"
        source = path.read_text(encoding="utf-8") if path.exists() else (
            "#![allow(unused, clippy::all)]\nuse xenon_rt::*;\nuse xenon_rt::x86_64::*;\n")
        source += "\n" + "\n\n".join(bodies[a] for a in sorted(new) if f"f{a[:4]}" == module) + "\n"
        write(path, source)
        if f"mod {module};" not in lib:
            lib = lib.replace("/// PPC_CALL_INDIRECT_FUNC:", f"mod {module};\npub use {module}::*;\n\n/// PPC_CALL_INDIRECT_FUNC:")
    lib = partition_dispatch(lib, {a: f"sub_{a}(ctx, base)" for a in sorted(new)})
    all_addresses = sorted(old | new)
    lib = lib[:lib.index("pub const LIFTED:")] + f"pub const LIFTED: [u32; {len(all_addresses)}] = [\n"
    lib += "".join(f"    0x{a},\n" for a in all_addresses) + "];\n"
    with open(RS / "sheets/lift_roots.csv", encoding="utf-8-sig", newline="") as file:
        root_count = sum(1 for _ in csv.DictReader(file))
    lib = re.sub(r"//! \d+ roots \(sheets/lift_roots.csv\); \d+ functions;",
        f"//! {root_count} roots (sheets/lift_roots.csv); {len(all_addresses)} functions;", lib, count=1)
    write(folder / "lib.rs", lib)
    print(f"Lifted {len(new)} original functions: {', '.join(sorted(new))}", flush=True)


if __name__ == "__main__":
    main()

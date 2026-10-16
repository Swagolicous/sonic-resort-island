"""Recompiled functions that build a given guest address (a register set to the high
half, then + low half within a few lines), e.g. the functions that store a vtable into
an object (constructors / destructors).

usage: python re/xenon/addrref.py <hex address>...
"""
import glob
import re
import sys
from pathlib import Path

PPC = Path(__file__).resolve().parent / "ppc"


def refs(targets):
    out = {t: set() for t in targets}
    for p in sorted(glob.glob(str(PPC / "ppc_recomp.*.cpp"))):
        txt = open(p, encoding="utf-8", errors="replace").read()
        for m in re.finditer(r"(\w+(?:\.\w+)?)\.s64 = (-?\d+);\n(?:.*\n){0,16}?.*?= \1\.s64 \+ (-?\d+);", txt):
            a = (int(m[2]) + int(m[3])) & 0xFFFFFFFF
            if a in out:
                fn = txt.rfind("PPC_FUNC_IMPL(__imp__sub_", 0, m.start())
                out[a].add(txt[fn + 25:fn + 33])
    return out


if __name__ == "__main__":
    targets = [int(a, 16) for a in sys.argv[1:]]
    for t, fns in refs(targets).items():
        print(f"{t:08X}: {' '.join(sorted(fns))}")

"""Look up recompiled guest functions in re/xenon/ppc (XenonRecomp output).

  python fn.py show <addr>              exact C++ of one function
  python fn.py tree <addr> [depth]      call tree with line counts (each function once)

Addresses are guest hex (8234EE00 or sub_8234EE00). The index of function ->
file/line is cached in ppc/index.json; delete it after re-running XenonRecomp.
"""
import json
import re
import sys
from pathlib import Path

PPC = Path(__file__).resolve().parent / "ppc"
INDEX = PPC / "index.json"
DECOMP_MANIFEST = Path(__file__).resolve().parents[3] / "Decomp" / "manifest.jsonl"


def index():
    if INDEX.is_file():
        return json.loads(INDEX.read_text())
    out = {}
    for p in sorted(PPC.glob("ppc_recomp.*.cpp")):
        lines = p.read_text(encoding="utf-8", errors="replace").splitlines()
        start = None
        for i, line in enumerate(lines):
            m = re.match(r"PPC_FUNC_IMPL\(__imp__sub_([0-9A-F]{8})\)", line)
            if m:
                start = (m[1], i)
            elif start and line == "}":
                out[start[0]] = [p.name, start[1], i + 1]
                start = None
    INDEX.write_text(json.dumps(out))
    return out


def names():
    out = {}
    if DECOMP_MANIFEST.is_file():
        for line in open(DECOMP_MANIFEST, encoding="utf-8"):
            r = json.loads(line)
            out[r["address"].upper()] = r["full_name"]
    return out


def body(idx, addr):
    f, a, b = idx[addr]
    return (PPC / f).read_text(encoding="utf-8", errors="replace").splitlines()[a:b]


def norm(a):
    return a.upper().removeprefix("SUB_").removeprefix("0X")


def main():
    cmd, addr = sys.argv[1], norm(sys.argv[2])
    idx = index()
    if addr not in idx:
        sys.exit(f"no recompiled function at {addr}")
    if cmd == "show":
        print("\n".join(body(idx, addr)))
    elif cmd == "tree":
        depth, nm, seen, total = int(sys.argv[3]) if len(sys.argv) > 3 else 3, names(), set(), 0

        def walk(a, d):
            nonlocal total
            n = len(body(idx, a))
            total += n
            print(f"{'  ' * d}{a} {n:6} {nm.get(a, '')}")
            if d >= depth:
                return
            for c in dict.fromkeys(re.findall(r"\bsub_([0-9A-F]{8})\(ctx, base\)", "\n".join(body(idx, a)))):
                if c in idx and c not in seen:
                    seen.add(c)
                    walk(c, d + 1)

        seen.add(addr)
        walk(addr, 0)
        print(f"total {total} lines in {len(seen)} functions")


if __name__ == "__main__":
    main()

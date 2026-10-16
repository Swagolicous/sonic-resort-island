"""Which decompiled functions read which named Sonic parameter.

Scans Decomp/src for parameter-store reads with a constant id, e.g.
    Function_8245DB60(*(undefined4 *)(param_1 + 0x1fc),0x10)
and maps ids to names via Decomp/params.json (from param_table.py).
usage: python param_uses.py fn <addr>        params read by one function
       python param_uses.py param <name|id>  functions that read a parameter
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
DECOMP = ROOT / "Decomp"
GETTERS = r"(?:Function|FUN)_8245(?:DB60|db60|DB18|db18|DC20|dc20)"
CALL = re.compile(GETTERS + r"\((?:[^(),]|\([^()]*\))*,\s*(0x[0-9a-fA-F]+|\d+)\)")
MARK = re.compile(r"^// @function (\S+) \d+ (\S+)")


def names():
    data = json.loads((DECOMP / "params.json").read_text(encoding="utf-8"))
    out = {}
    for group in data.values():
        for e in group:
            if e.get("name"):
                out[e["id"]] = e["name"]
    return out


def scan():
    uses = {}
    for path in (DECOMP / "src").rglob("*.c"):
        cur = None
        for line in path.read_text(encoding="utf-8", errors="replace").split("\n"):
            m = MARK.match(line)
            if m:
                cur = (m.group(1), m.group(2))
                continue
            if cur:
                for g in CALL.finditer(line):
                    uses.setdefault(cur, set()).add(int(g.group(1), 0))
    return uses


def main():
    n = names()
    cache = DECOMP / "param_uses.json"
    if cache.exists():
        uses = {tuple(k.split(" ")): set(v) for k, v in json.loads(cache.read_text()).items()}
    else:
        uses = scan()
        cache.write_text(json.dumps({" ".join(k): sorted(v) for k, v in uses.items()}))
    if sys.argv[1] == "fn":
        for (a, fname), ids in uses.items():
            if a == sys.argv[2].lower():
                for i in sorted(ids):
                    print(f"0x{i:03x} {n.get(i, '?')}")
    elif sys.argv[1] == "param":
        q = sys.argv[2]
        want = {int(q, 0)} if q.startswith("0x") else {i for i, s in n.items() if s == q}
        for (a, fname), ids in sorted(uses.items()):
            if ids & want:
                print(a, fname, " ".join(n.get(i, hex(i)) for i in sorted(ids)))


if __name__ == "__main__":
    main()

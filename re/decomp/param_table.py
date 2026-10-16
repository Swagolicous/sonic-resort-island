"""Extract Sonic's tuning parameters (id, name, default, editor range) straight from the
parameter-registration functions in default.xex.

Those functions (sub_82324410 Sonic, sub_8237a828 per-mode) are too large for Ghidra's
decompiler, but they are straight-line code with a fixed pattern per parameter:
    set_default(store, id, &value)                  8245eac0 (float) / 8245eb58.. (other types)
    string(name_jp); string(name_en)                82dfb728
    ptr = get(store, id)                            8245d6e0 (float) / 8245d7a0.. (other types)
    item = editor_item(&tmp, name_jp, ptr, a, b, c) 825b2d50 (float) / 825b2e28.. (other types)
    category_add(cat, name_en, item)                825a69e8
so a linear constant-propagation pass recovers the whole table.

usage: python param_table.py [--json out.json]
"""
import json
import struct
import sys

from ppc import Image
from ppc_calls import calls

FUNCS = {"sonic": (0x82324410, 43380), "mode": (0x8237A828, 47292), "common": (0x8230DBA8, 3560)}
STRING = 0x82DFB728
SETTERS = {0x8245EAC0: "float", 0x8245EB58: "t58", 0x8245EB98: "t98", 0x8245EC18: "t18",
           0x8245EBD8: "td8", 0x8245EB00: "t00"}
GETTERS = {0x8245D6E0, 0x8245D7A0, 0x8245D860, 0x8245D740, 0x8245D800}
ITEMS = {0x825B2D50, 0x825B2E28, 0x825B2CE0, 0x825A83F0}
CATEGORY = 0x825CFC40
ADDS = {0x825A69E8, 0x825A7520, 0x825A6ED8, 0x825A7150}


def extract(img, start, size):
    params = {}
    cats = {}

    def slot(v):
        return v[1] if isinstance(v, tuple) and v[0] == "sp" else None

    def hook(addr, tgt, g, f, stack):
        if tgt == STRING:
            s, p = slot(g.get(3)), g.get(4)
            if s is not None and isinstance(p, int):
                stack[s] = ("str", img.cstr(p & 0xFFFFFFFF, "shift_jis"))
            return None
        if tgt in SETTERS:
            pid, s = g.get(4), slot(g.get(5))
            if isinstance(pid, int):
                v = stack.get(s) if s is not None else None
                e = params.setdefault(pid, {"id": pid})
                st = g.get(3)
                store = f"{st[1]:08x}" if isinstance(st, tuple) and st[0] == "mem" else "?"
                e.setdefault("defaults", {})[store] = v
                e["type"] = SETTERS[tgt]
                e.setdefault("set_at", f"{addr:08x}")
            return None
        if tgt in GETTERS:
            pid = g.get(4)
            return ("param", pid) if isinstance(pid, int) else None
        if tgt in ITEMS:
            name, ptr = stack.get(slot(g.get(4))), g.get(5)
            if isinstance(ptr, tuple) and ptr[0] == "param":
                e = params.setdefault(ptr[1], {"id": ptr[1]})
                if isinstance(name, tuple):
                    e["name_jp"] = name[1]
                e["editor"] = [f.get(1), f.get(2), f.get(3)]
                return ("item", ptr[1])
            return None
        if tgt == CATEGORY:
            name = stack.get(slot(g.get(4)))
            return ("cat", name[1] if isinstance(name, tuple) else None)
        if tgt in ADDS:
            cat, name, item = g.get(3), stack.get(slot(g.get(4))), g.get(5)
            if isinstance(item, tuple) and item[0] == "item":
                e = params[item[1]]
                if isinstance(name, tuple):
                    e["name"] = name[1]
                if isinstance(cat, tuple) and cat[0] == "cat":
                    e["category"] = cat[1]
            return None
        return None

    calls(img, start, size, hook)
    return params


def fmt(v):
    if isinstance(v, float):
        return f"{v:.6g}"
    return repr(v)


def main():
    img = Image()
    out = {}
    for key, (start, size) in FUNCS.items():
        ps = extract(img, start, size)
        out[key] = sorted(ps.values(), key=lambda e: e["id"])
        print(f"== {key} ({len(ps)} params)")
        for e in out[key]:
            print(f"  0x{e['id']:03x} {e.get('name','?'):34} {e.get('type','?'):6} "
                  f"{' '.join(k[-3:] + '=' + fmt(v) for k, v in e.get('defaults', {}).items())}  "
                  f"[{e.get('category','')}] {[fmt(x) for x in e.get('editor', [])]}")
    if "--json" in sys.argv:
        with open(sys.argv[sys.argv.index("--json") + 1], "w", encoding="utf-8") as fh:
            json.dump(out, fh, ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main()

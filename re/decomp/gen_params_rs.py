"""Generate crates/unleashed-game/src/sonic/param_table.rs from Decomp/params.json.

params.json comes from param_table.py (straight from default.xex), so the Rust table
is a mechanical copy of the executable's defaults: one entry per parameter id with
its value in every store that sets it. Store inheritance comes from the
create-store calls (8245ec58(parent) -> 8230cdb8(global)).
usage: python gen_params_rs.py
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "unleashed-rs" / "crates" / "unleashed-game" / "src" / "sonic" / "param_table.rs"

# global that holds the store -> global of its parent store (from ppc_calls.py on the
# two registration functions).
PARENTS = {
    0x83362F9C: 0x83362F50, 0x83362FA4: 0x83362F9C, 0x83362FAC: 0x83362F9C,
    0x83362FB4: 0x83362FA4, 0x83362FBC: 0x83362FA4, 0x83362FC4: 0x83362FAC,
    0x83362FCC: 0x83362FA4, 0x83362FD4: 0x83362FAC,
    0x83364728: 0x83362FA4, 0x83364750: 0x83362FA4, 0x83364758: 0x83364750,
    0x83364760: 0x83364758, 0x83364768: 0x83364758, 0x83364770: 0x83364758,
    0x83364778: 0x83364758,
}


def main():
    data = json.loads((ROOT / "Decomp" / "params.json").read_text(encoding="utf-8"))
    merged = {}
    for group in data.values():
        for e in group:
            m = merged.setdefault(e["id"], {"id": e["id"], "values": {}})
            for k in ("name", "category", "name_jp", "type"):
                if e.get(k) and not m.get(k):
                    m[k] = e[k]
            for store, v in e.get("defaults", {}).items():
                if store != "?" and v is not None and not isinstance(v, (list, dict, str)):
                    m["values"][int(store, 16)] = float(v)
    lines = [
        "//! Sonic's parameter defaults, extracted from default.xex by",
        "//! re/decomp/param_table.py + gen_params_rs.py (registration functions",
        "//! sub_82324410 and sub_8237a828). Generated: do not edit.",
        "",
        "/// One parameter: its id in the store, editor name/category, and the value",
        "/// each store sets (store = address of the global that holds it).",
        "pub struct ParamDef {",
        "    pub id: u16,",
        "    pub name: &'static str,",
        "    pub category: &'static str,",
        "    pub values: &'static [(u32, f32)],",
        "}",
        "",
        "/// (store, parent store) pairs: lookups fall back to the parent chain.",
        "pub const STORE_PARENTS: &[(u32, u32)] = &[",
    ]
    lines += [f"    (0x{k:08X}, 0x{v:08X})," for k, v in PARENTS.items()]
    lines += ["];", "", "pub static PARAMS: &[ParamDef] = &["]
    for pid in sorted(merged):
        m = merged[pid]
        name = m.get("name") or f"Param{pid:03X}"
        vals = ", ".join(f"(0x{s:08X}, {v!r}_f32)".replace("inf", "f32::INFINITY") for s, v in sorted(m["values"].items()))
        lines.append(f'    ParamDef {{ id: 0x{pid:03X}, name: "{name}", category: "{m.get("category", "")}", '
                     f"values: &[{vals}] }},")
    lines += ["];", ""]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {OUT} ({len(merged)} params)")


if __name__ == "__main__":
    main()

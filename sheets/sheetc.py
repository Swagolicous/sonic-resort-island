"""Sheet compiler: the CSVs in this folder are the source of truth for generated code.

  json       sheets -> build/sheets.json (typed, like rows grouped, columns listed once)
  check      every row x column intersection must be checked (non-empty; "-" = checked, no value)
  preflight  check + types + keys + refs + cross-sheet rules + drift; lists failures and
             unimplemented items, writes build/preflight.json, exits 1 on any failure
  gen        preflight, then write one strut per row from build/sheets.json
  build      gen, then cargo build (and test) the crates that receive generated code

Schema lives in _sheets.csv (key / group / outputs), _columns.csv (type, nullable) and
_refs.csv (foreign keys). Generators read build/sheets.json only, never the CSVs.

usage: python sheets/sheetc.py <json|check|preflight|gen|build> [--test]
"""
import csv
import json
import math
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RS = HERE.parent
CRATES = RS / "crates"
DECOMP = RS.parent / "Decomp"
BUILD = HERE / "build"
NULL = "-"
SYMBOLS = {"inf": "f32::INFINITY", "TAU": "std::f32::consts::TAU", "PI": "std::f32::consts::PI"}


# ---------------------------------------------------------------- reading

def read_csv(name):
    with open(HERE / f"{name}.csv", encoding="utf-8-sig", newline="") as fh:
        rows = list(csv.reader(fh))
    return rows[0], rows[1:]


def schema():
    _, sheets = read_csv("_sheets")
    _, cols = read_csv("_columns")
    _, refs = read_csv("_refs")
    out = {}
    for sheet, key, group, generates, doc in sheets:
        out[sheet] = {"key": key.split("+"), "group": None if group == NULL else group,
                      "generates": [] if generates == NULL else generates.split(";"), "doc": doc, "columns": {}, "refs": []}
    for sheet, col, typ, nullable, doc in cols:
        out[sheet]["columns"][col] = {"type": typ, "nullable": nullable == "yes"}
    for sheet, cols_, target, tcols in refs:
        out[sheet]["refs"].append((cols_.split("+"), target, tcols.split("+")))
    return out


def parse(typ, v):
    """Raw cell -> typed value; raises ValueError when the cell does not fit its type."""
    if typ == "hex":
        if not re.fullmatch(r"0x[0-9A-F]+", v):
            raise ValueError("expected upper-case 0x hex")
        return int(v, 16)
    if typ == "int":
        return int(v)
    if typ == "f32":
        if v in SYMBOLS:
            return v
        f = float(v)
        if not math.isfinite(f):
            raise ValueError("use inf as a symbol")
        return f
    if typ.startswith("enum:"):
        if v not in typ[5:].split("|"):
            raise ValueError(f"not one of {typ[5:]}")
        return v
    pattern = {"ident": r"[A-Za-z_][A-Za-z0-9_]*", "snake": r"[a-z_][a-z0-9_]*", "screaming": r"[A-Z_][A-Z0-9_]*",
               "addrlist": r"(?:sub_|DAT_)[0-9A-Fa-f]{8}(?:;(?:sub_|DAT_)[0-9A-Fa-f]{8})*",
               "module": r"unleashed-[a-z]+/[a-z0-9_/]+", "crate": r"unleashed-[a-z]+",
               "sheetlist": r"[a-z_]+(?:;[a-z_]+)*"}.get(typ)
    if pattern and not re.fullmatch(pattern, v):
        raise ValueError(f"not a valid {typ}")
    if typ == "regex":
        re.compile(v)
    return v


def load():
    """All sheets, typed. Returns (schema, {sheet: [row dict]}, issues)."""
    sch = schema()
    data, issues = {}, {"unchecked": [], "failures": []}
    for sheet, meta in sch.items():
        header, rows = read_csv(sheet)
        cols = meta["columns"]
        if header != list(cols):
            issues["failures"].append(f"{sheet}: header {header} != _columns.csv {list(cols)}")
            data[sheet] = []
            continue
        typed = []
        for i, raw in enumerate(rows, start=2):
            if len(raw) != len(header):
                issues["failures"].append(f"{sheet}:{i}: {len(raw)} cells, expected {len(header)}")
                continue
            row, missing = {}, []
            for col, v in zip(header, raw):
                v = v.strip()
                if v == "":
                    missing.append(col)
                    row[col] = None
                elif v == NULL:
                    if not cols[col]["nullable"]:
                        issues["failures"].append(f"{sheet}:{i}.{col}: '-' but column is not nullable")
                    row[col] = None
                else:
                    try:
                        row[col] = parse(cols[col]["type"], v)
                    except (ValueError, re.error) as e:
                        issues["failures"].append(f"{sheet}:{i}.{col}={v!r}: {e}")
                        row[col] = None
            if missing:
                issues["unchecked"].append(f"{sheet}:{i} unfinished, unchecked: {', '.join(missing)}")
            row["_line"] = i
            typed.append(row)
        data[sheet] = typed
    return sch, data, issues


def to_json(sch, data):
    """Columns listed once per sheet; rows as arrays, grouped by the sheet's group column."""
    out = {}
    for sheet, rows in data.items():
        cols = list(sch[sheet]["columns"])
        groups = {}
        for r in rows:
            g = r.get(sch[sheet]["group"]) if sch[sheet]["group"] else "all"
            groups.setdefault(fmt_key(g), []).append([r[c] for c in cols])
        out[sheet] = {"doc": sch[sheet]["doc"], "key": sch[sheet]["key"], "columns": cols, "groups": groups}
    return out


def fmt_key(v):
    if v is None:
        return NULL
    return f"0x{v:08X}" if isinstance(v, int) else str(v)


def rows_of(js, sheet):
    s = js[sheet]
    return [dict(zip(s["columns"], r)) for g in s["groups"].values() for r in g]


# ---------------------------------------------------------------- preflight

def manifest_functions():
    addrs = set()
    with open(DECOMP / "manifest.jsonl", encoding="utf-8") as fh:
        for line in fh:
            m = re.search(r'"address": "([0-9a-f]{8})"', line)
            if m:
                addrs.add(m[1])
    return addrs


def merged_params(js):
    """Gen's merge: by id over groups in enum order, first non-null name/category wins."""
    order = schema()["params"]["columns"]["group"]["type"][5:].split("|")
    regs = sorted(rows_of(js, "params"), key=lambda r: (order.index(r["group"]), r["id"]))
    merged = {}
    for r in regs:
        m = merged.setdefault(r["id"], {"id": r["id"], "values": {}, "regs": []})
        m["regs"].append(r)
        for k in ("name", "category"):
            if r[k] is not None and m.get(k) is None:
                m[k] = r[k]
    for v in sorted(rows_of(js, "param_values"), key=lambda r: order.index(r["group"])):
        merged[v["param_id"]]["values"].setdefault(v["store"], []).append(v)
    for m in merged.values():
        m["ident"] = m.get("name") or f"Param{m['id']:03X}"
    return merged


def rules(sch, data, js):
    """Cross-sheet rules: everything that would fail to generate, compile or run."""
    fail, todo = [], []
    # keys
    for sheet, rows in data.items():
        seen = {}
        for r in rows:
            k = tuple(r.get(c) for c in sch[sheet]["key"])
            if k in seen:
                fail.append(f"{sheet}:{r['_line']}: duplicate key {k} (first at line {seen[k]})")
            seen.setdefault(k, r["_line"])
    # refs
    for sheet, meta in sch.items():
        for cols, target, tcols in meta["refs"]:
            have = {tuple(r.get(c) for c in tcols) for r in data[target]}
            for r in data[sheet]:
                k = tuple(r.get(c) for c in cols)
                if None not in k and k not in have:
                    fail.append(f"{sheet}:{r['_line']}: {'+'.join(cols)}={[fmt_key(x) for x in k]} not in {target}")
    stores = {r["store"]: r for r in rows_of(js, "stores")}
    # store tree terminates at a root
    for s in stores:
        chain, cur = [], s
        while cur is not None and cur in stores:
            if cur in chain:
                fail.append(f"stores: cycle through 0x{s:08X}")
                break
            chain.append(cur)
            cur = stores[cur]["parent"]
    names = [r["name"] for r in stores.values() if r["name"]]
    for n in {n for n in names if names.count(n) > 1}:
        fail.append(f"stores: name {n} used twice (STORE_ constant collides)")
    # params merge cleanly into one table entry per id
    merged = merged_params(js)
    for m in merged.values():
        for k in ("name", "category"):
            vals = {r[k] for r in m["regs"] if r[k] is not None}
            if len(vals) > 1:
                fail.append(f"params 0x{m['id']:03X}: groups disagree on {k} {sorted(vals)}; merged table is ambiguous")
        for store, vs in m["values"].items():
            if len({v["value"] for v in vs}) > 1:
                fail.append(f"param_values 0x{m['id']:03X} @0x{store:08X}: groups set different values")
    idents = {}
    for m in merged.values():
        idents.setdefault(m["ident"], []).append(m["id"])
    for n, ids in idents.items():
        if len(ids) > 1:
            fail.append(f"params: name {n} on ids {[hex(i) for i in ids]}; lookups by name are ambiguous")
    # every bound field resolves in every view (else SonicParams::from_set panics)
    def resolve(pid, store):
        cur = store
        while cur is not None:
            if cur in merged[pid]["values"]:
                return True
            cur = stores.get(cur, {}).get("parent")
        return False
    views = [s for s, r in stores.items() if r["view"] == "yes"]
    if not views:
        fail.append("stores: no row has view=yes")
    for f in rows_of(js, "sonic_fields"):
        if f["param_id"] not in merged:
            continue
        for v in views:
            if not resolve(f["param_id"], v):
                fail.append(f"sonic_fields.{f['field']}: {merged[f['param_id']]['ident']} has no value in "
                            f"{stores[v]['name']} or its parents; from_set would panic")
    # constants cite functions that exist in the decomp
    funcs = manifest_functions()
    for c in rows_of(js, "constants"):
        for ref in (c["source"] or "").split(";"):
            if ref.startswith("sub_") and ref[4:].lower() not in funcs:
                fail.append(f"constants.{c['name']}: {ref} is not a function in Decomp/manifest.jsonl")
        if not (CRATES / c["module"].split("/")[0] / "src" / c["module"].split("/", 1)[1]).is_dir():
            fail.append(f"constants.{c['name']}: module dir {c['module']} does not exist")
        if c["source"] is None:
            todo.append(f"constants.{c['name']}: no decomp source cited")
    # systems: regexes still bucket the RTTI classes to the recorded counts
    systems = sorted(data["systems"], key=lambda r: r["_line"])
    counts, seen = {r["system"]: 0 for r in systems}, set()
    with open(DECOMP / "rtti_classes.jsonl", encoding="utf-8") as fh:
        for line in fh:
            cls = json.loads(line)["class"]
            if cls not in seen:
                seen.add(cls)
                hit = next((r["system"] for r in systems if r["match"] and re.search(r["match"], cls)), None)
                if hit is None:
                    fail.append(f"systems: class {cls} matches no row")
                else:
                    counts[hit] += 1
    for r in systems:
        if r["classes"] is not None and counts[r["system"]] != r["classes"]:
            fail.append(f"systems.{r['system']}: classes={r['classes']} but rtti_classes.jsonl gives {counts[r['system']]}")
        if r["crate"] and not (CRATES / r["crate"]).is_dir():
            fail.append(f"systems.{r['system']}: crate {r['crate']} does not exist")
        for s in (r["sheets"] or "").split(";"):
            if s and s not in sch:
                fail.append(f"systems.{r['system']}: sheet {s} does not exist")
        if counts[r["system"]] and r["sheets"] is None:
            todo.append(f"systems.{r['system']}: {counts[r['system']]} classes, no sheet"
                        + ("" if r["crate"] else ", no crate"))
    f2, t2 = rules_camera_look_port(js)
    fail += f2 + rules_context(js)
    todo += t2
    # outputs declared in _sheets.csv are the ones the generators write
    declared = {o for m in sch.values() for o in m["generates"]}
    produced = set(GENERATED_FILES)
    for o in declared ^ produced:
        fail.append(f"_sheets.generates: {o} is {'declared but not generated' if o in declared else 'generated but not declared'}")
    # unnamed registrations
    unnamed = [m["ident"] for m in merged.values() if not m.get("name")]
    if unnamed:
        todo.append(f"params: {len(unnamed)} ids registered without a name (emitted as ParamXXX): {', '.join(unnamed)}")
    unnamed_stores = [fmt_key(s) for s, r in stores.items() if not r["name"]]
    if unnamed_stores:
        todo.append(f"stores: {len(unnamed_stores)} stores not identified: {', '.join(unnamed_stores)}")
    # hand-written literal constants in crates that receive generated code
    gen_paths = {(CRATES / o).resolve() for o in produced}
    for crate in sorted({o.split("/")[0] for o in produced}):
        for p in sorted((CRATES / crate / "src").rglob("*.rs")):
            if p.resolve() in gen_paths:
                continue
            for n, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
                if m := re.match(r"\s*(?:pub )?const (\w+): f32 = [-0-9.]", line):
                    todo.append(f"{p.relative_to(RS).as_posix()}:{n}: const {m[1]} is a literal, not in a sheet")
    return fail, todo


XENOS = RS / "re" / "xenos"
XENON_INDEX = RS / "re" / "xenon" / "ppc" / "index.json"
VARIANT_BITS = ((4, "_NoLight"), (2, "_NoGI"), (1, "_ConstTexCoord"))


def variant_names(base, mask):
    return [base + "".join(sfx for bit, sfx in VARIANT_BITS if i & bit) for i in range(8) if mask >> i & 1]


def rules_camera_look_port(js):
    fail, todo = [], []
    # camera: each path sits in its view's section and is read by one field
    section = {"common": "Common/", "forward": "Sonic/Forward/", "side": "Sonic/Side/"}
    paths = {}
    for r in rows_of(js, "camera_fields"):
        if not r["xml_path"].startswith(section[r["view"]]):
            fail.append(f"camera_fields.{r['field']}: {r['xml_path']} is not under {section[r['view']]}")
        if r["xml_path"] in paths:
            fail.append(f"camera_fields: {r['xml_path']} read by {paths[r['xml_path']]} and {r['field']}")
        paths[r["xml_path"]] = r["field"]
    # look: every variant a shader list can select is a program with its microcode on disk
    programs = {r["program"]: r for r in rows_of(js, "shader_programs")}
    for r in rows_of(js, "shader_lists"):
        for stage, base, mask in (("pixel", r["pixel_shader"], r["pixel_variants"]),
                                  ("vertex", r["vertex_shader"], r["vertex_variants"])):
            for prog in variant_names(base, mask):
                if prog not in programs:
                    fail.append(f"shader_lists {r['shader_list']}/{r['vertex_perm']}: {stage} variant {prog} not in shader_programs")
                elif programs[prog]["stage"] != stage:
                    fail.append(f"shader_programs.{prog}: stage {programs[prog]['stage']} but {r['shader_list']} uses it as {stage}")
    untranslated, unported = [], []
    for p in programs.values():
        if not (XENOS / "shader" / p["source"]).is_file():
            fail.append(f"shader_programs.{p['program']}: {p['source']} missing from re/xenos/shader")
        if not (XENOS / "hlsl" / f"{p['program']}.hlsl").is_file():
            untranslated.append(p["program"])
        if p["wgsl"] is None:
            unported.append(p["program"])
        elif not (RS / p["wgsl"]).is_file():
            fail.append(f"shader_programs.{p['program']}: wgsl {p['wgsl']} does not exist")
    if untranslated:
        todo.append(f"shader_programs: {len(untranslated)} not translated to HLSL (re/xenos/translate.py): {', '.join(untranslated)}")
    if unported:
        todo.append(f"shader_programs: {len(unported)}/{len(programs)} have no WGSL port")
    # port: ported rows name a Rust fn that exists and cites the guest address
    known = set(json.loads(XENON_INDEX.read_text())) if XENON_INDEX.is_file() else None
    progress = {}
    host_methods = {b["host"] for b in rows_of(js, "boundaries")}
    context_fields = {(f["struct"], f["field"]) for f in rows_of(js, "context_fields")}
    for r in rows_of(js, "port_functions"):
        addr = f"{r['address']:08X}"
        if known is not None and addr not in known:
            fail.append(f"port_functions 0x{addr}: not a recompiled function")
        done = progress.setdefault(r["system"], [0, 0, 0, 0])
        done[1] += 1
        done[3] += r["lines"]
        if r["rust"] is None:
            continue
        if r["rust"].startswith("field:"):
            struct, _, field = r["rust"][6:].partition(".")
            if (struct, field) not in context_fields:
                fail.append(f"port_functions 0x{addr}: {r['rust']} is not in context_fields.csv")
            done[0] += 1
            done[2] += r["lines"]
            continue
        if r["rust"].startswith("host:"):
            if r["rust"][5:] not in host_methods:
                fail.append(f"port_functions 0x{addr}: {r['rust']} is not a host method in boundaries.csv")
            done[0] += 1
            done[2] += r["lines"]
            continue
        m = re.fullmatch(r"(unleashed-[a-z]+/[\w/]+\.rs)::(\w+)", r["rust"])
        if not m:
            fail.append(f"port_functions 0x{addr}: rust {r['rust']!r} is not crate/path.rs::fn")
            continue
        src = CRATES / m[1]
        text = src.read_text(encoding="utf-8") if src.is_file() else ""
        if not re.search(rf"\bfn {m[2]}\b", text):
            fail.append(f"port_functions 0x{addr}: {r['rust']} not found")
        elif f"sub_{addr}" not in text:
            fail.append(f"port_functions 0x{addr}: {m[1]} does not cite sub_{addr}")
        done[0] += 1
        done[2] += r["lines"]
    for system, (n, total, lines, all_lines) in sorted(progress.items()):
        if n < total:
            todo.append(f"port_functions.{system}: {n}/{total} functions ported ({lines}/{all_lines} recompiled lines)")
    if known is None:
        todo.append("port_functions: re/xenon/ppc not generated, addresses unchecked (re/xenon/build.bat + XenonRecomp)")
    return fail, todo


def preflight(quiet=False):
    sch, data, issues = load()
    js = to_json(sch, data)
    BUILD.mkdir(exist_ok=True)
    (BUILD / "sheets.json").write_text(json.dumps(js, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    fail, todo = list(issues["failures"]), []
    if not fail and not issues["unchecked"]:
        f, todo = rules(sch, data, js)
        fail += f
    stale = [] if fail or issues["unchecked"] else [o for o, text in generate(js).items()
                                                    if not (CRATES / o).is_file() or (CRATES / o).read_text(encoding="utf-8") != text]
    total = sum(len(r) * len(sch[s]["columns"]) for s, r in data.items())
    report = {"intersections": total, "unchecked": issues["unchecked"], "failures": fail,
              "unimplemented": todo, "stale_outputs": stale,
              "sheets": {s: len(r) for s, r in data.items()}}
    (BUILD / "preflight.json").write_text(json.dumps(report, ensure_ascii=False, indent=1), encoding="utf-8")
    ok = not fail and not issues["unchecked"]
    if not quiet or not ok:
        rows = sum(len(r) for r in data.values())
        print(f"preflight: {len(data)} sheets, {rows} rows, {total} intersections")
        for title, items in (("UNCHECKED", issues["unchecked"]), ("FAIL", fail), ("UNIMPLEMENTED", todo),
                             ("STALE (gen rewrites)", stale)):
            if items:
                print(f"  {title} ({len(items)})")
                for it in items[:40]:
                    print(f"    {it}")
                if len(items) > 40:
                    print(f"    ... {len(items) - 40} more in sheets/build/preflight.json")
        print("preflight: " + ("CLEAN" if ok else "BLOCKED - fix the sheets and re-run"))
    return ok, js


# ---------------------------------------------------------------- generation

def f32(v):
    return SYMBOLS[v] if isinstance(v, str) else f"{v!r}"


def rs_str(s):
    return json.dumps(s, ensure_ascii=False)


def gen_param_table(js):
    merged = merged_params(js)
    stores = sorted(rows_of(js, "stores"), key=lambda r: r["store"])
    out = [
        "//! Sonic's parameter defaults, as registered by default.xex (sub_82324410 and",
        "//! sub_8237a828). Generated by sheets/sheetc.py from sheets/params.csv,",
        "//! param_values.csv and stores.csv: edit the sheets, not this file.",
        "",
    ]
    for s in stores:
        if s["name"]:
            if s["doc"]:
                out.append(f"/// {s['doc']}")
            out.append(f"pub const STORE_{re.sub(r'(?<!^)(?=[A-Z])', '_', s['name']).upper()}: u32 = 0x{s['store']:08X};")
    out += [
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
    out += [f"    (0x{s['store']:08X}, 0x{s['parent']:08X})," for s in stores if s["parent"] is not None]
    out += ["];", "", "pub static PARAMS: &[ParamDef] = &["]
    for pid in sorted(merged):
        m = merged[pid]
        vals = ", ".join(f"(0x{s:08X}, {f32(vs[0]['value'])}_f32)".replace("f32::INFINITY_f32", "f32::INFINITY")
                         for s, vs in sorted(m["values"].items()))
        out.append(f'    ParamDef {{ id: 0x{pid:03X}, name: "{m["ident"]}", category: "{m.get("category") or ""}", '
                   f"values: &[{vals}] }},")
    return "\n".join(out + ["];", ""])


def gen_param_fields(js):
    merged = merged_params(js)
    fields = rows_of(js, "sonic_fields")
    out = [
        "//! SonicParams: the parameters the ported controller reads. Generated by",
        "//! sheets/sheetc.py from sheets/sonic_fields.csv: edit the sheet, not this file.",
        "",
        "use super::params::ParamSet;",
        "",
        "/// The parameters the ported controller reads, by their in-game names.",
        "#[derive(Clone, Debug)]",
        "pub struct SonicParams {",
    ]
    for f in fields:
        out.append(f"    /// 0x{f['param_id']:03X} {merged[f['param_id']]['ident']}")
        out.append(f"    pub {f['field']}: f32,")
    out += ["}", "", "impl SonicParams {", "    pub fn from_set(set: &ParamSet) -> Self {", "        Self {"]
    for f in fields:
        n = merged[f["param_id"]]["ident"]
        out.append(f'            {f["field"]}: set.get("{n}").expect("parameter {n} missing"),')
    return "\n".join(out + ["        }", "    }", "}", ""])


def gen_constants(js, module):
    rows = [r for r in rows_of(js, "constants") if r["module"] == module]
    out = [
        "//! Constants read out of the executable. Generated by sheets/sheetc.py from",
        "//! sheets/constants.csv: edit the sheet, not this file.",
        "",
    ]
    for r in rows:
        out.append(f"/// {r['doc']}")
        out.append(f"pub const {r['name']}: f32 = {f32(r['value'])};")
    return "\n".join(out + [""])


CAMERA_STRUCTS = {"common": "CommonCameraParams", "forward": "ForwardCameraParams", "side": "SideCameraParams"}


def gen_camera_params(js):
    fields = rows_of(js, "camera_fields")
    out = [
        "//! Sonic's camera parameters, read from the player's own `Camera.prm.xml`",
        "//! (#SystemCommon). Generated by sheets/sheetc.py from sheets/camera_fields.csv:",
        "//! edit the sheet, not this file.",
        "",
    ]
    for view, name in CAMERA_STRUCTS.items():
        rows = [f for f in fields if f["view"] == view]
        section = "/".join(rows[0]["xml_path"].split("/")[:-2])
        out += [f"/// `Camera.prm.xml` `{section}` section.", "#[derive(Clone, Debug, Default, PartialEq)]",
                f"pub struct {name} {{"]
        for f in rows:
            out += [f"    /// `{f['xml_path']}`", f"    pub {f['field']}: {f['type']},"]
        out += ["}", "", f"impl {name} {{", "    /// Every path this struct reads, in field order.",
                "    pub const PATHS: &'static [&'static str] = &["]
        out += [f"        {rs_str(f['xml_path'])}," for f in rows]
        out += ["    ];", "",
                "    /// Reads every field through `get(path)` (flattened `Section/Category/Name`);",
                "    /// `Err` lists the paths that were missing or did not parse.",
                "    pub fn read<'a>(get: impl Fn(&str) -> Option<&'a str>) -> Result<Self, Vec<&'static str>> {",
                "        let mut bad = Vec::new();",
                "        let p = Self {"]
        out += [f"            {f['field']}: value(&get, {rs_str(f['xml_path'])}, &mut bad)," for f in rows]
        out += ["        };", "        if bad.is_empty() { Ok(p) } else { Err(bad) }", "    }", "}", ""]
    out += [
        "fn value<'a, T: std::str::FromStr + Default>(",
        "    get: &impl Fn(&str) -> Option<&'a str>,",
        "    path: &'static str,",
        "    bad: &mut Vec<&'static str>,",
        ") -> T {",
        "    match get(path).map(|v| v.trim().parse()) {",
        "        Some(Ok(v)) => v,",
        "        _ => {",
        "            bad.push(path);",
        "            T::default()",
        "        }",
        "    }",
        "}",
        "",
    ]
    return "\n".join(out)


def gen_look(js):
    out = [
        "//! What Sonic's look is made of: SonicRoot.model's materials, the shader lists they",
        "//! draw with, and the Xenos programs those lists select. Generated by sheets/sheetc.py",
        "//! from sheets/sonic_materials.csv, shader_lists.csv and shader_programs.csv: edit the",
        "//! sheets, not this file.",
        "",
        "/// Variant bits: bit `i` of `*_variants` set = the program with these suffixes exists.",
        "pub const CONST_TEX_COORD: u8 = 1;",
        "pub const NO_GI: u8 = 2;",
        "pub const NO_LIGHT: u8 = 4;",
        "",
        "pub struct Material {",
        "    pub name: &'static str,",
        "    pub shader_list: &'static str,",
        "}",
        "",
        "/// One (pixel, vertex) permutation of a shader list. `vertex_perm` is \"none\" for",
        "/// regular meshes and \"p\" for morph meshes.",
        "pub struct ShaderPermutation {",
        "    pub shader_list: &'static str,",
        "    pub pixel_perm: &'static str,",
        "    pub pixel_shader: &'static str,",
        "    pub pixel_variants: u8,",
        "    pub vertex_perm: &'static str,",
        "    pub vertex_shader: &'static str,",
        "    pub vertex_variants: u8,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]",
        "pub enum Stage {",
        "    Vertex,",
        "    Pixel,",
        "}",
        "",
        "pub struct Program {",
        "    pub name: &'static str,",
        "    pub stage: Stage,",
        "    /// Crate-relative WGSL port, once it exists.",
        "    pub wgsl: Option<&'static str>,",
        "}",
        "",
        "pub static MATERIALS: &[Material] = &[",
    ]
    out += [f"    Material {{ name: {rs_str(m['material'])}, shader_list: {rs_str(m['shader_list'])} }},"
            for m in rows_of(js, "sonic_materials")]
    out += ["];", "", "pub static SHADER_LISTS: &[ShaderPermutation] = &["]
    for r in rows_of(js, "shader_lists"):
        out.append(f"    ShaderPermutation {{ shader_list: {rs_str(r['shader_list'])}, pixel_perm: {rs_str(r['pixel_perm'])}, "
                   f"pixel_shader: {rs_str(r['pixel_shader'])}, pixel_variants: 0x{r['pixel_variants']:02X}, "
                   f"vertex_perm: {rs_str(r['vertex_perm'])}, vertex_shader: {rs_str(r['vertex_shader'])}, "
                   f"vertex_variants: 0x{r['vertex_variants']:02X} }},")
    out += ["];", "", "pub static PROGRAMS: &[Program] = &["]
    for p in rows_of(js, "shader_programs"):
        wgsl = f"Some({rs_str(p['wgsl'])})" if p["wgsl"] else "None"
        out.append(f"    Program {{ name: {rs_str(p['program'])}, stage: Stage::{p['stage'].title()}, wgsl: {wgsl} }},")
    out += ["];", "",
            "/// Name of variant `variant` (a combination of the bits above) of `base`.",
            "pub fn variant_name(base: &str, variant: u8) -> String {",
            "    let mut name = base.to_owned();",
            "    for (bit, suffix) in [(NO_LIGHT, \"_NoLight\"), (NO_GI, \"_NoGI\"), (CONST_TEX_COORD, \"_ConstTexCoord\")] {",
            "        if variant & bit != 0 {",
            "            name.push_str(suffix);",
            "        }",
            "    }",
            "    name",
            "}",
            ""]
    return "\n".join(out)


def gen_crt_tables(js):
    out = [
        "//! Constant tables of the console CRT math the port reproduces, as default.xex",
        "//! stores them. Generated by sheets/sheetc.py from sheets/math_tables.csv: edit the",
        "//! sheet, not this file. Names are <TABLE>_<kind><byte offset in the table>.",
        "",
    ]
    for r in sorted(rows_of(js, "math_tables"), key=lambda r: (r["table"], r["offset"])):
        name = f"{r['table'].upper()}_{'D' if r['kind'] == 'f64' else 'S'}{r['offset']}"
        width = 16 if r["kind"] == "f64" else 8
        out.append(f"/// 0x{r['base'] + r['offset']:08X}")
        out.append(f"pub const {name}: {r['kind']} = {r['kind']}::from_bits(0x{r['bits']:0{width}X});")
    return "\n".join(out + [""])


FIELD_SIZE = {"vec4": 16, "quat": 16, "mat4": 64, "f32": 4, "i32": 4, "ptr": 4, "u8": 1}


def gen_context(js):
    fields = sorted(rows_of(js, "context_fields"), key=lambda r: (r["struct"], r["offset"]))
    out = [
        "//! Guest layout of Sonic's speed context and posture: byte offsets (used to marshal",
        "//! state for the xenon-oracle tests and to decode traces) and flag indices.",
        "//! Generated by sheets/sheetc.py from sheets/context_fields.csv and",
        "//! context_flags.csv: edit the sheets, not this file.",
        "",
        "pub mod offset {",
    ]
    for struct in dict.fromkeys(f["struct"] for f in fields):
        out.append(f"    pub mod {struct} {{")
        for f in (f for f in fields if f["struct"] == struct):
            out.append(f"        /// {f['type']}: {f['doc']}")
            out.append(f"        pub const {f['field'].upper()}: u32 = 0x{f['offset']:03X};")
        out.append("    }")
    out += ["}", "", "/// Index of each flag in the flag set at speed_context+0x4AC.", "pub mod flag {"]
    for f in sorted(rows_of(js, "context_flags"), key=lambda r: r["index"]):
        out.append(f"    /// {f['doc']}")
        out.append(f"    pub const {f['name'].upper()}: u32 = {f['index']};")
    return "\n".join(out + ["}", ""])


def rules_context(js):
    fail = []
    funcs = manifest_functions()
    for sheet in ("context_fields", "context_flags"):
        for r in rows_of(js, sheet):
            for ref in r["evidence"].split(";"):
                if ref.startswith("sub_") and ref[4:].lower() not in funcs:
                    fail.append(f"{sheet}.{r.get('field') or r.get('name')}: {ref} is not a decomp function")
    spans = {}
    for r in rows_of(js, "context_fields"):
        spans.setdefault(r["struct"], []).append((r["offset"], r["offset"] + FIELD_SIZE[r["type"]], r["field"]))
    for struct, s in spans.items():
        s.sort()
        for (a0, a1, a), (b0, b1, b) in zip(s, s[1:]):
            if b0 < a1:
                fail.append(f"context_fields.{struct}: {a} (0x{a0:X}-0x{a1:X}) overlaps {b} at 0x{b0:X}")
    return fail


def gen_vmx(js):
    sys.path.insert(0, str(RS / "re" / "xenon"))
    import vmx2rust

    out = [
        "//! Straight-line VMX guest functions, translated operation for operation from",
        "//! XenonRecomp's C++ by re/xenon/vmx2rust.py. Generated by sheets/sheetc.py from",
        "//! sheets/vmx_functions.csv: edit the sheet, not this file.",
        "",
        "use super::vmx::*;",
        "",
    ]
    entries = []
    for r in sorted(rows_of(js, "vmx_functions"), key=lambda r: r["address"]):
        t = vmx2rust.Translator(f"{r['address']:08X}")
        code = t.run()
        sig = (";".join(f"{a}+{o}" for a, o in t.inputs), ";".join(f"{a}+{o}" for a, o in t.outputs))
        if sig != (r["inputs"], r["outputs"]):
            raise SystemExit(f"vmx_functions 0x{r['address']:08X}: translation reads {sig[0]} / writes {sig[1]}, "
                             f"sheet says {r['inputs']} / {r['outputs']}")
        out += [code, "", f"/// {r['doc']}", f"pub use self::sub_{r['address']:08x} as {r['name']};", ""]
        args = ", ".join(f"a[{k}]" for k in range(len(t.inputs)))
        ret = "vec![r]" if len(t.outputs) == 1 else "r.to_vec()"
        loc = lambda keys: ", ".join(f"({a[1:]}, {o})" for a, o in keys)
        entries.append(f"    VmxEntry {{ address: 0x{r['address']:08X}, inputs: &[{loc(t.inputs)}], outputs: &[{loc(t.outputs)}], "
                       f"call: |a| {{ let r = sub_{r['address']:08x}({args}); {ret} }} }},")
    out += [
        "/// Every translated function with where it reads and writes (argument register, byte",
        "/// offset), so the xenon-oracle test checks each one against the guest.",
        "pub struct VmxEntry {",
        "    pub address: u32,",
        "    pub inputs: &'static [(u8, u32)],",
        "    pub outputs: &'static [(u8, u32)],",
        "    pub call: fn(&[[f32; 4]]) -> Vec<[f32; 4]>,",
        "}",
        "",
        "pub const ENTRIES: &[VmxEntry] = &[",
        *entries,
        "];",
        "",
    ]
    return "\n".join(out)


GENERATED_FILES = {
    "unleashed-game/src/sonic/vmx_gen.rs": gen_vmx,
    "unleashed-game/src/sonic/param_table.rs": gen_param_table,
    "unleashed-game/src/sonic/param_fields.rs": gen_param_fields,
    "unleashed-game/src/sonic/constants.rs": lambda js: gen_constants(js, "unleashed-game/sonic"),
    "unleashed-game/src/camera/params.rs": gen_camera_params,
    "unleashed-game/src/sonic/look.rs": gen_look,
    "unleashed-game/src/sonic/crt_tables.rs": gen_crt_tables,
    "unleashed-game/src/sonic/context.rs": gen_context,
}


def generate(js):
    return {path: fn(js) for path, fn in GENERATED_FILES.items()}


def gen():
    ok, _ = preflight(quiet=True)
    if not ok:
        sys.exit(1)
    # Generate from the verified JSON on disk, not from the CSVs.
    js = json.loads((BUILD / "sheets.json").read_text(encoding="utf-8"))
    for path, text in generate(js).items():
        p = CRATES / path
        if not p.is_file() or p.read_text(encoding="utf-8") != text:
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text, encoding="utf-8", newline="\n")
            print(f"gen: wrote {path}")
    print("gen: up to date")
    return sorted({p.split("/")[0] for p in GENERATED_FILES})


def build(test):
    crates = gen()
    args = [a for c in crates for a in ("-p", c)]
    for cmd in (["build"], ["test"]) if test else (["build"],):
        print(f"cargo {' '.join(cmd + args)}")
        if subprocess.run(["cargo", *cmd, *args], cwd=RS).returncode:
            sys.exit(1)


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "preflight"
    if cmd == "json":
        sch, data, issues = load()
        BUILD.mkdir(exist_ok=True)
        (BUILD / "sheets.json").write_text(json.dumps(to_json(sch, data), ensure_ascii=False, separators=(",", ":")),
                                           encoding="utf-8")
        print(f"json: wrote sheets/build/sheets.json ({len(issues['failures'])} parse failures)")
    elif cmd == "check":
        sch, data, issues = load()
        for sheet, rows in data.items():
            bad = [u for u in issues["unchecked"] if u.startswith(f"{sheet}:")]
            print(f"  {sheet:14} {len(rows) - len(bad):5}/{len(rows):<5} rows checked  x {len(sch[sheet]['columns'])} columns")
        for u in issues["unchecked"]:
            print(f"    {u}")
        sys.exit(1 if issues["unchecked"] else 0)
    elif cmd == "preflight":
        sys.exit(0 if preflight()[0] else 1)
    elif cmd == "gen":
        gen()
    elif cmd == "build":
        build("--test" in sys.argv)
    elif cmd == "lift":
        # Lift the call closure of every lift_roots row to crates/unleashed-guest.
        ok, js = preflight(quiet=True)
        if not ok:
            sys.exit(1)
        sys.path.insert(0, str(RS / "re" / "xenon"))
        import ppc2rust
        roots = [f"{r['address']:08X}" for r in rows_of(js, "lift_roots")]
        names, failed = ppc2rust.write_crate(ppc2rust.Lifter(), roots, CRATES / "unleashed-guest")
        print(f"lift: {len(names)} functions from {len(roots)} roots, {len(failed)} not liftable"
              f"{' (see crates/unleashed-guest/lift_failures.json)' if failed else ''}")
        sys.exit(1 if failed else 0)
    elif cmd == "dbsync":
        # The database method: mirror what the sheets know into the decomp database
        # (Decomp/src/.gamedb/index.sqlite) so it can be queried next to the code.
        import sqlite3
        db = sqlite3.connect(RS.parent / "Decomp" / "src" / ".gamedb" / "index.sqlite")
        db.executescript("""
            DROP TABLE IF EXISTS rewrite_port;
            CREATE TABLE rewrite_port(address TEXT, system TEXT, rust TEXT, lines INTEGER, depth INTEGER,
                                      PRIMARY KEY(address, system));
            DROP TABLE IF EXISTS rewrite_context_field;
            CREATE TABLE rewrite_context_field(struct TEXT, offset INTEGER, field TEXT, type TEXT, evidence TEXT, doc TEXT);
            DROP TABLE IF EXISTS rewrite_boundary;
            CREATE TABLE rewrite_boundary(address TEXT PRIMARY KEY, host TEXT, args TEXT, doc TEXT);
        """)
        _, rows = read_csv("port_functions")
        db.executemany("INSERT OR REPLACE INTO rewrite_port VALUES (?,?,?,?,?)",
                       [(r[1][2:].lower(), r[0], None if r[6] == NULL else r[6], int(r[3]), int(r[5])) for r in rows])
        _, rows = read_csv("context_fields")
        db.executemany("INSERT INTO rewrite_context_field VALUES (?,?,?,?,?,?)",
                       [(r[0], int(r[1], 16), r[2], r[3], r[4], r[5]) for r in rows])
        _, rows = read_csv("boundaries")
        db.executemany("INSERT INTO rewrite_boundary VALUES (?,?,?,?)", [(r[0][2:].lower(), r[1], r[2], r[3]) for r in rows])
        db.commit()
        done = db.execute("select count(distinct address) from rewrite_port where rust is not null").fetchone()[0]
        total = db.execute("select count(distinct address) from rewrite_port").fetchone()[0]
        print(f"dbsync: {done}/{total} tracked functions handled; context fields and boundaries mirrored")
    elif cmd == "port":
        # port <address> <crate/path.rs::fn>: record a ported function in every system
        # that uses it (preflight then checks the fn exists and cites the address).
        addr, rust = f"0x{int(sys.argv[2], 16):08X}", sys.argv[3]
        header, rows = read_csv("port_functions")
        col = {c: i for i, c in enumerate(header)}
        hits = [r for r in rows if r[col["address"]] == addr]
        if not hits:
            sys.exit(f"{addr} is not in port_functions.csv")
        for r in hits:
            r[col["rust"]] = rust
        with open(HERE / "port_functions.csv", "w", encoding="utf-8-sig", newline="") as fh:
            csv.writer(fh).writerows([header, *rows])
        print(f"port: {addr} -> {rust} ({', '.join(r[col['system']] for r in hits)})")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()

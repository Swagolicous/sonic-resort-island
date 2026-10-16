"""One-time seed of the sheets from the existing decomp outputs.

After seeding, the CSVs in this folder are the source of truth: edit them, not
this script. Re-seeding overwrites them, so it refuses unless --force.

Sources (nothing here is typed in by hand):
  params / param_values  Decomp/params.json (re/decomp/param_table.py, from default.xex)
  stores                 re/decomp/gen_params_rs.py PARENTS + names/docs from sonic/params.rs
  sonic_fields           the sonic_params! binding in sonic/params.rs
  constants              the `pub const ..: f32` block in sonic/controller.rs
  systems                Decomp/rtti_classes.jsonl, bucketed by SYSTEM_RULES below

It parses params.rs / controller.rs as they were before the sheets took over, so it
cannot re-run once those modules use the generated code.

usage: python sheets/seed.py <player|camera|look|port|samplers|math> [--force]
"""
import csv
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RS = HERE.parent
DECOMP = RS.parent / "Decomp"
SONIC = RS / "crates" / "unleashed-game" / "src" / "sonic"

sys.path.insert(0, str(RS / "re" / "decomp"))
from gen_params_rs import PARENTS  # noqa: E402

# (system, class-name regex, crate implementing it or "-", sheets describing it or "-").
# First match wins.
SYSTEM_RULES = [
    ("player", r"^SWA::(Player::|CPlayer|CExPlayer)", "unleashed-game", "params;param_values;stores;sonic_fields;constants;port_functions;sonic_materials;shader_lists;shader_programs;camera_fields"),
    ("enemy", r"^SWA::(EvilEnemy::|CEnemy)", "-", "-"),
    ("boss", r"^SWA::(Boss::|CBoss|CExStageBoss)", "-", "-"),
    ("message", r"^SWA::Message::", "-", "-"),
    ("set_object", r"^SWA::(CObj|TrickSwitch|CTriggerZone|CHitEventTrigger|CVolumeEventTrigger|CSetEditor|CMetaInstancer|CInstanc|PathAnimation)", "unleashed-bevy", "-"),
    ("camera", r"^SWA::.*Camera", "unleashed-runtime", "-"),
    ("event", r"^SWA::(Inspire::|CEvent)", "-", "-"),
    ("game_flow", r"^SWA::(Sequence::|CGameMode|CMainMenuManager|CLogin|Achievement|CAchievement)", "-", "-"),
    ("hud", r"^SWA::(CHud|CGameObjectCSD|CMainMenuCSD|CScene)|^Chao", "unleashed-bevy", "-"),
    ("effect", r"^SWA::(CFx|CSpark|CSparkle|CFountain|ParticleTest)|^Hedgehog::Sparkle", "unleashed-render", "-"),
    ("audio", r"^SWA::(CSound|CAnimationEventTriggerAudio)", "unleashed-audio", "-"),
    ("npc", r"^SWA::NPC", "-", "-"),
    ("debug", r"^SWA::(CParameterEditor|CDebug|CReplay|CGameObjectCollisionRender)|^Hedgehog::MirageDebug", "-", "-"),
    ("render", r"^Hedgehog::Mirage|^SWA::(COccluder|CVisibility|CAutoDraw|CXMLType)", "unleashed-render", "-"),
    ("engine", r"^(Hedgehog|Sonicteam)::", "unleashed-runtime", "-"),
    ("havok_physics", r"^(hkp|hkMopp|hkCpuWorld|hkHeightField)", "unleashed-physics", "-"),
    ("havok_animation", r"^hka", "unleashed-animation", "-"),
    ("havok_base", r"^(hk|ValidatedClassNameRegistry)", "-", "-"),
    ("library", r"^(Templates|std|stdext|boost)\b", "-", "-"),
    ("swa_other", r"^SWA::", "-", "-"),
    ("other", r".", "-", "-"),
]

NULL = "-"


def write(name, header, rows):
    path = HERE / f"{name}.csv"
    with open(path, "w", encoding="utf-8-sig", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(header)
        w.writerows(rows)
    print(f"  {name}.csv  {len(rows)} rows")


def cell(v):
    return NULL if v in (None, "") else str(v)


def seed_params():
    # One row per registration: a few ids are registered by both groups with
    # different Japanese names / editor ranges, so (group, id) is the key.
    data = json.loads((DECOMP / "params.json").read_text(encoding="utf-8"))
    rows, values, names = [], [], {}
    for group, entries in data.items():
        for e in sorted(entries, key=lambda e: e["id"]):
            ed = e.get("editor") or [None] * 3
            rows.append([group, f"0x{e['id']:03X}", cell(e.get("name")), cell(e.get("name_jp")),
                         cell(e.get("category")), e["type"], *(cell(x) for x in ed), f"0x{e['set_at'].upper()}"])
            if e.get("name"):
                names.setdefault(e["id"], e["name"])
            for store, v in sorted(e.get("defaults", {}).items()):
                if store == "?" or v is None or isinstance(v, (list, dict, str)):
                    continue
                values.append([group, f"0x{e['id']:03X}", f"0x{int(store, 16):08X}", repr(float(v))])
            names.setdefault(e["id"], None)
    write("params", ["group", "id", "name", "name_jp", "category", "type", "editor_min", "editor_max",
                     "editor_step", "set_at"], rows)
    write("param_values", ["group", "param_id", "store", "value"], values)
    return {pid: n or f"Param{pid:03X}" for pid, n in names.items()}


def seed_stores():
    src = (SONIC / "params.rs").read_text(encoding="utf-8")
    named = {}  # store -> (name, doc)
    for doc, name, addr in re.findall(r"/// (.+)\npub const STORE_(\w+): u32 = 0x([0-9A-F]+);", src):
        camel = "".join(p.title() for p in name.split("_"))
        named[int(addr, 16)] = (camel, doc)
    # Stores the View enum maps to: the runtime resolves every bound field there.
    views = {"".join(p.title() for p in n.split("_")) for n in re.findall(r"View::\w+ => STORE_(\w+),", src)}
    stores = sorted(set(PARENTS) | set(PARENTS.values()))
    rows = []
    for s in stores:
        name, doc = named.get(s, (NULL, NULL))
        rows.append([f"0x{s:08X}", f"0x{PARENTS[s]:08X}" if s in PARENTS else NULL, name,
                     "yes" if name in views else "no", doc])
    write("stores", ["store", "parent", "name", "view", "doc"], rows)


def seed_sonic_fields(names):
    src = (SONIC / "params.rs").read_text(encoding="utf-8")
    block = src[src.index("sonic_params! {"):]
    by_name = {n: pid for pid, n in names.items()}
    rows = []
    for field, name in re.findall(r"^\s+(\w+) = \"(\w+)\",", block[:block.index("\n}")], re.M):
        rows.append([field, f"0x{by_name[name]:03X}"])
    write("sonic_fields", ["field", "param_id"], rows)  # param_id resolves over all groups


def seed_constants():
    src = (SONIC / "controller.rs").read_text(encoding="utf-8")
    rows, doc = [], []
    for line in src.splitlines():
        if line.startswith("/// "):
            if not doc or prev_const:
                doc = []
            doc.append(line[4:])
            prev_const = False
        elif m := re.match(r"pub const (\w+): f32 = (.+);", line):
            text = " ".join(doc)
            value = m[2].replace("std::f32::consts::", "")
            refs = sorted(set(re.findall(r"(?:sub_|DAT_)[0-9A-Fa-f]{8}", text)), key=str.upper)
            rows.append([m[1], "unleashed-game/sonic", value, ";".join(refs) or NULL, text])
            prev_const = True
        elif line.startswith("#[derive"):
            break
        else:
            doc, prev_const = [], False
    write("constants", ["name", "module", "value", "source", "doc"], rows)


def seed_systems():
    seen, counts = set(), {r[0]: 0 for r in SYSTEM_RULES}
    for line in open(DECOMP / "rtti_classes.jsonl", encoding="utf-8"):
        cls = json.loads(line)["class"]
        if cls in seen:
            continue
        seen.add(cls)
        counts[next(r[0] for r in SYSTEM_RULES if re.search(r[1], cls))] += 1
    write("systems", ["system", "match", "classes", "crate", "sheets"],
          [[s, rx, counts[s], crate, sheets] for s, rx, crate, sheets in SYSTEM_RULES])


# ---------------------------------------------------------------- camera / look / port

GAME = RS.parent / "game"
XENOS = RS / "re" / "xenos"


def cargo_example(*args):
    import subprocess
    r = subprocess.run(["cargo", "run", "-q", "--release", "-p", "unleashed-assets", "--example", *args],
                       cwd=RS, capture_output=True, text=True, encoding="utf-8")
    if r.returncode:
        sys.exit(f"example {args[0]} failed:\n{r.stderr[-2000:]}")
    return r.stdout


def extracted(stem, name):
    out = HERE / "build" / "extract"
    if not (out / stem.lstrip("#") / name).is_file():
        cargo_example("extract", "arl", str(GAME), stem, str(out))
    return out / stem.lstrip("#") / name


def snake(name):
    for prefix in ("param.", "s_GlobalParam.", "s_Param."):
        name = name.removeprefix(prefix)
    name = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name.replace(".", "_"))
    return re.sub(r"_+", "_", name).lower()


def seed_camera():
    # Names and types only: values stay in the player's own Camera.prm.xml and are read at runtime.
    import xml.etree.ElementTree as ET
    root = ET.parse(extracted("#SystemCommon", "Camera.prm.xml")).getroot()
    rows = []

    def walk(node, path):
        kids = list(node)
        if not kids:
            if path[0] in ("Common", "Sonic"):
                view = "common" if path[0] == "Common" else path[1].lower()
                text = (node.text or "").strip()
                rows.append([view, path[-2], "/".join(path), snake(path[-1]),
                             "bool" if text in ("true", "false") else "f32"])
            return
        for k in kids:
            walk(k, path if k.tag in ("Category", "Param") else path + [k.tag])

    walk(root, [])
    write("camera_fields", ["view", "category", "xml_path", "field", "type"], rows)


VARIANT_BITS = ((4, "_NoLight"), (2, "_NoGI"), (1, "_ConstTexCoord"))


def variants(base, mask):
    return [base + "".join(sfx for bit, sfx in VARIANT_BITS if i & bit) for i in range(8) if mask >> i & 1]


def parse_shader_list(data):
    """CShaderListData file: pixel permutations {sub, name, shader, [vertex permutations {sub, name, shader}]}."""
    import struct
    u32 = lambda o: struct.unpack_from(">I", data, 0x18 + o)[0]
    cstr = lambda o: data[0x18 + o:data.index(b"\0", 0x18 + o)].decode()
    out = []
    count, table = u32(0), u32(4)
    for i in range(count):
        pp = u32(table + 4 * i)
        psub, pname, pshader, vcount, vtable = (u32(pp + 4 * k) for k in range(5))
        for j in range(vcount):
            vp = u32(vtable + 4 * j)
            vsub, vname, vshader = (u32(vp + 4 * k) for k in range(3))
            out.append((cstr(pname), cstr(pshader), psub, cstr(vname), cstr(vshader), vsub))
    return out


def seed_look():
    mats = []
    for line in cargo_example("model_materials", "SonicRoot").splitlines():
        m = re.match(r"  (\S+)\s+(\S+)\s+\[(.*)\]$", line)
        if m:
            tex = ";".join(t.strip().strip('"') for t in m[3].split(",") if t.strip())
            mats.append([m[1], m[2], tex or NULL])
    write("sonic_materials", ["material", "shader_list", "textures"], mats)
    lists, programs = [], {}
    for sl in sorted({m[1] for m in mats}):
        for pname, pshader, psub, vname, vshader, vsub in parse_shader_list((XENOS / "shader" / f"{sl}.shader-list").read_bytes()):
            lists.append([sl, pname, pshader, f"0x{psub:02X}", vname, vshader, f"0x{vsub:02X}"])
            for prog in variants(pshader, psub):
                programs[prog] = "pixel"
            for prog in variants(vshader, vsub):
                programs[prog] = "vertex"
    write("shader_lists", ["shader_list", "pixel_perm", "pixel_shader", "pixel_variants", "vertex_perm",
                           "vertex_shader", "vertex_variants"], lists)
    rows = []
    for prog, stage in sorted(programs.items()):
        src = f"{prog}.{'xpu' if stage == 'pixel' else 'xvu'}"
        if not (XENOS / "shader" / src).is_file():
            sys.exit(f"{src} missing from re/xenos/shader")
        rows.append([prog, stage, src, NULL])
    write("shader_programs", ["program", "stage", "source", "wgsl"], rows)


# Classes behind each ported system (a class and its nested classes, e.g. posture
# states). Their vtable methods, minus methods shared by many unrelated classes
# (engine base stubs), are the roots; every function they statically reach is tracked.
PORT_ROOTS = {
    "camera3d": ["SWA::CPlayer3DNormalCamera3"],
    "camera2d": ["SWA::CPlayer2DNormalCameraNew"],
    "posture3d": ["SWA::Player::CPlayerSpeedPosture3DStandard", "SWA::Player::CPlayerSpeedPosture3DAir",
                  "SWA::Player::CPlayerSpeedPosture3DCommon", "SWA::Player::CPlayerSpeedPostureCommon",
                  "SWA::Player::CPlayerSpeedPostureInput3DStandard", "SWA::Player::CPlayerSpeedPostureBrake",
                  "SWA::Player::CPlayerSpeedPosturePluginGravity", "SWA::Player::CPlayerSpeedPosturePluginAirDrag"],
    "posture2d": ["SWA::Player::CPlayerSpeedPosture2DStandard", "SWA::Player::CPlayerSpeedPosture2DAir",
                  "SWA::Player::CPlayerSpeedPosture2DCommon", "SWA::Player::CPlayerSpeedPostureInput2DStandard"],
}


def seed_port():
    import collections
    sys.path.insert(0, str(RS / "re" / "xenon"))
    import fn
    idx, names = fn.index(), fn.names()
    # Re-seeding keeps every port already recorded.
    ported = {}
    if (HERE / "port_functions.csv").is_file():
        with open(HERE / "port_functions.csv", encoding="utf-8-sig", newline="") as fh:
            ported = {r["address"]: r["rust"] for r in csv.DictReader(fh) if r["rust"] != NULL}
    methods, uses = collections.defaultdict(list), collections.Counter()
    for line in open(DECOMP / "rtti_classes.jsonl", encoding="utf-8"):
        r = json.loads(line)
        for m in set(r["methods"]):
            uses[m] += 1
        for system, prefixes in PORT_ROOTS.items():
            if any(r["class"] == p or r["class"].startswith(p + "::") for p in prefixes):
                methods[system] += r["methods"]
    callees = {}

    def calls(a):
        if a not in callees:
            body = "\n".join(fn.body(idx, a))
            callees[a] = list(dict.fromkeys(re.findall(r"\bsub_([0-9A-F]{8})\(ctx, base\)", body)))
        return callees[a]

    # Boundaries (sheets/boundaries.csv) are host services: tracked as host:<method>,
    # and the walk does not continue behind them.
    with open(HERE / "boundaries.csv", encoding="utf-8-sig", newline="") as fh:
        hosts = {f"{int(r['address'], 16):08X}": r["host"] for r in csv.DictReader(fh)}
    rows = []
    for system in PORT_ROOTS:
        roots = [m for m in dict.fromkeys(methods[system]) if uses[m] <= 5 and m in idx]
        queue, seen = [(m, None, 0) for m in roots], set()
        while queue:  # breadth first: depth = shortest call distance from a root
            a, parent, depth = queue.pop(0)
            if a in seen or a not in idx:
                continue
            seen.add(a)
            addr = f"0x{a}"
            rust = f"host:{hosts[a]}" if a in hosts else ported.get(addr, NULL)
            rows.append([system, addr, names.get(a, NULL), len(fn.body(idx, a)),
                         f"0x{parent}" if parent else NULL, depth, rust])
            if a not in hosts:
                queue += [(c, a, depth + 1) for c in calls(a) if c not in seen]
    write("port_functions", ["system", "address", "name", "lines", "parent", "depth", "rust"], rows)


def seed_samplers():
    # Sampler names the translated programs fetch through, and which texture kinds each
    # uses; the slot fixes its wgpu bindings (re/xenos/wgsl.py).
    with open(HERE / "shader_programs.csv", encoding="utf-8-sig", newline="") as fh:
        programs = [r["program"] for r in csv.DictReader(fh)]
    kinds = {}
    for p in programs:
        body = (XENOS / "hlsl" / f"{p}.hlsl").read_text(encoding="utf-8").split("void main(", 1)[1]
        for name, kind in re.findall(r"\b(\w+?)_(Texture2D|Texture3D|TextureCube)DescriptorIndex\b", body):
            kinds.setdefault(name, set()).add({"Texture2D": "2d", "Texture3D": "3d", "TextureCube": "cube"}[kind])
    write("shader_samplers", ["sampler", "slot", "kinds"],
          [[n, i, ";".join(sorted(k))] for i, (n, k) in enumerate(sorted(kinds.items()))])


# Guest constant tables the ported CRT math reads: (name, base, [offset:kind]).
MATH_TABLES = [
    ("atan2", 0x8335F5C0, [f"{o}:f64" for o in (8, 16, 24, 40, 56, 64, 72, 80, 88, 96, 104, 112, 128, 136, 144, 152)]
     + ["168:f32", "176:f32"]),
    # sub_831B1788 (sin) / sub_831B1868 (cos) share this table.
    ("sincos", 0x8335F4E0, ["0:f64", "8:f64", "16:f64", "24:f32", "28:f32", "32:f32", "36:f32"]
     + [f"{o}:f64" for o in range(40, 120, 8)]),
    ("sincos_c0", 0x8204F160, ["0:f64"]),
    ("sincos_big", 0x8335FCD0, ["0:f64"]),
    # sub_831B3B98: the asin / acos core (r4 = 0 asin, 1 acos).
    ("asin_acos", 0x8335F678, ["0:f32", "4:f32", "8:f32", "12:f32"] + [f"{o}:f64" for o in range(16, 128, 8)]),
]


def seed_math():
    import subprocess
    rows = []
    for name, base, specs in MATH_TABLES:
        out = subprocess.run(["cargo", "run", "-q", "-p", "xenon-oracle", "--example", "dump_table", "--", name,
                              f"{base:08X}", *specs], cwd=RS, capture_output=True, text=True, check=True).stdout
        rows += [line.split(",") for line in out.splitlines() if line.count(",") == 4]
    write("math_tables", ["table", "base", "offset", "kind", "bits"], rows)


GROUPS = {
    # group: (sheets it writes, seeding function)
    "player": (["params", "param_values", "stores", "sonic_fields", "constants", "systems"], None),
    "camera": (["camera_fields"], lambda: seed_camera()),
    "look": (["shader_lists", "shader_programs", "sonic_materials"], lambda: seed_look()),
    "port": (["port_functions"], lambda: seed_port()),
    "samplers": (["shader_samplers"], lambda: seed_samplers()),
    "math": (["math_tables"], lambda: seed_math()),
}


def seed_player():
    names = seed_params()
    seed_stores()
    seed_sonic_fields(names)
    seed_constants()
    seed_systems()


def main():
    """usage: seed.py <group> [--force]; groups seed once, then their sheets are the truth."""
    group = next((a for a in sys.argv[1:] if not a.startswith("--")), None)
    if group not in GROUPS:
        sys.exit(f"usage: seed.py <{'|'.join(GROUPS)}> [--force]")
    sheets, fn = GROUPS[group]
    have = [s for s in sheets if (HERE / f"{s}.csv").exists()]
    if have and "--force" not in sys.argv:
        sys.exit(f"{', '.join(have)} already seeded; they are the source of truth now (pass --force to overwrite)")
    print(f"seeding {group}")
    (fn or seed_player)()


if __name__ == "__main__":
    main()

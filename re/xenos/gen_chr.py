"""Generate Sonic's material shaders from his own programs (sheets drive it).

For every shader list in sheets/sonic_materials.csv, take its vertex program (vertex
permutation `none`) and pixel program (base variant: GI and lights on, as in a stage)
from sheets/shader_lists.csv, compose them (compose.py) into
crates/unleashed-bevy/src/chr/<list>.wgsl, and write chr/layout.rs: the member layout of
each program's constant block (name, vec4 count, vec4 offset), so the host fills the
blocks by constant name with the values the game uses.

usage: python re/xenos/gen_chr.py
"""
import csv
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RS = HERE.parents[1]
sys.path.insert(0, str(HERE))
import compose  # noqa: E402

OUT = RS / "crates" / "unleashed-bevy" / "src" / "chr"


def rows(name):
    with open(RS / "sheets" / f"{name}.csv", encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def ident(shader_list):
    return re.sub(r"[^A-Za-z0-9]+", "_", shader_list).strip("_")


def layout(wgsl, struct):
    """[(member, vec4 count, vec4 offset)] of `struct` (every member is vec4-aligned)."""
    m = re.search(rf"struct {re.escape(struct)} \{{\n(.*?)\n\}}", wgsl, re.S)
    if not m:
        return []
    out, off = [], 0
    for line in m[1].split("\n"):
        f = re.match(r"\s*(\w+): (.*),$", line)
        if not f:
            continue
        name, ty = f[1], f[2]
        if ty in ("vec4<f32>", "vec4<u32>", "vec4<i32>"):
            n = 1
        else:
            a = re.fullmatch(r"array<vec4<f32>, (\d+)>", ty)
            if not a:
                raise SystemExit(f"{struct}.{name}: unsupported type {ty}")
            n = int(a[1])
        out.append((name, n, off))
        off += n
    return out


def main():
    lists = {}
    for r in rows("shader_lists"):
        if r["vertex_perm"] == "none" and r["pixel_perm"] == "default":
            lists[r["shader_list"]] = (r["vertex_shader"], r["pixel_shader"])
    used = sorted({r["shader_list"] for r in rows("sonic_materials")})
    OUT.mkdir(parents=True, exist_ok=True)
    rust = ["//! Constant-block layouts of Sonic's programs (re/xenos/gen_chr.py; do not edit):",
            "//! per shader list, (constant, vec4 count, vec4 offset) for the vertex and pixel blocks.",
            "",
            "pub struct ChrLayout {",
            "    pub shader_list: &'static str,",
            "    pub shader: &'static str,",
            "    pub vertex: &'static [(&'static str, u32, u32)],",
            "    pub pixel: &'static [(&'static str, u32, u32)],",
            "}",
            "",
            "pub const LAYOUTS: &[ChrLayout] = &["]
    for sl in used:
        vs, ps = lists[sl]
        vs_path, ps_path = HERE / "wgsl" / f"{vs}.wgsl", HERE / "wgsl" / f"{ps}.wgsl"
        text = compose.compose(vs_path, ps_path)
        name = ident(sl)
        (OUT / f"{name}.wgsl").write_text(text, encoding="utf-8", newline="\n")
        vl = layout(vs_path.read_text(encoding="utf-8"), "type_u002e_VertexShaderConstants")
        pl = layout(ps_path.read_text(encoding="utf-8"), "type_u002e_PixelShaderConstants")
        fmt = lambda l: ", ".join(f'("{n}", {c}, {o})' for n, c, o in l)
        rust += [f"    ChrLayout {{ shader_list: \"{sl}\", shader: \"{name}\", ",
                 f"        vertex: &[{fmt(vl)}],",
                 f"        pixel: &[{fmt(pl)}] }},"]
        print(f"{sl}: {vs} + {ps} -> chr/{name}.wgsl (vertex {sum(c for _, c, _ in vl)} vec4, pixel {sum(c for _, c, _ in pl)} vec4)")
    rust += ["];", ""]
    (OUT / "layout.rs").write_text("\n".join(rust), encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()

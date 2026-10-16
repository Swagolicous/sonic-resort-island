"""Mechanically convert the translated programs in sheets/shader_programs.csv to WGSL.

  hlsl/<p>.hlsl  (XenosRecomp output)
    -> wgpu/<p>.hlsl  shared header swapped for wgpu_common.hlsli, bindless texture
                      access rewritten to fixed bindings, constants as plain cbuffers
    -> wgpu/<p>.spv   dxc, same flags XenosRecomp uses for SPIR-V (minus -fvk-invert-y:
                      wgpu clip space is D3D's, as on the 360)
    -> wgsl/<p>.wgsl  naga, which also validates it

Bindings (one layout for every Sonic program):
  group 0  b0 VertexShaderConstants, b1 PixelShaderConstants, b2 SharedConstants
           (the game's own register files, packoffset = guest constant register)
  group 1  sampler slot i from sheets/shader_samplers.csv:
           4i Texture2D, 4i+1 Texture3D, 4i+2 TextureCube, 4i+3 SamplerState

On success the program's `wgsl` cell in the sheet is set to its output path. Outputs
are derived from the player's own shader.ar and are not committed.

usage: python re/xenos/wgsl.py [program ...]
"""
import csv
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RS = HERE.parents[1]
SHEETS = RS / "sheets"
DXC = RS.parent / "refs" / "UnleashedRecomp" / "tools" / "XenosRecomp" / "thirdparty" / "dxc-bin" / "bin" / "x64" / "dxc.exe"
NAGA = Path.home() / ".cargo" / "bin" / "naga.exe"
KINDS = {"Texture2D": ("t2d", 0), "Texture3D": ("t3d", 1), "TextureCube": ("tcube", 2)}


def read_sheet(name):
    with open(SHEETS / f"{name}.csv", encoding="utf-8-sig", newline="") as fh:
        rows = list(csv.reader(fh))
    return rows[0], rows[1:]


def strip_common_header(text):
    """Drop XenosRecomp's inlined shader_common.h (the include-guarded block at the top)."""
    lines = text.splitlines(keepends=True)
    assert lines[0].startswith("#ifndef SHADER_COMMON_H_INCLUDED"), "unexpected header"
    depth = 0
    for i, line in enumerate(lines):
        s = line.strip()
        if s.startswith("#if"):
            depth += 1
        elif s.startswith("#endif"):
            depth -= 1
            if depth == 0:
                return "".join(lines[i + 1:])
    raise ValueError("unterminated shader_common.h")


def sort_cbuffers(body):
    """dxc's SPIR-V backend wants cbuffer members in packoffset order; XenosRecomp lists
    them by name. Reordering keeps every member at its explicit offset. Macros declared
    between members move after the block."""
    def member_key(line):
        if "DEFINE_SHARED_CONSTANTS" in line:
            return 16 * 4  # expands to members at c16-c17
        m = re.search(r"packoffset\(c(\d+)(?:\.([xyzw]))?\)", line)
        return int(m[1]) * 4 + "xyzw".index(m[2] or "x")

    def fix(m):
        lines = m[2].splitlines(keepends=True)
        is_member = lambda l: "packoffset(" in l or "DEFINE_SHARED_CONSTANTS" in l
        members = [l for l in lines if is_member(l)]
        rest = [l for l in lines if not is_member(l) and l.strip()]
        return m[1] + "".join(sorted(members, key=member_key)) + m[3] + "".join(rest)

    return re.sub(r"(cbuffer \w+ : register\([^)]*\)\s*\{\n)(.*?)(\};\n)", fix, body, flags=re.S)


def convert_source(text, slots):
    body = strip_common_header(text)
    # The first __spirv__ block holds the buffer-device-address constant macros; take
    # the cbuffer branch instead, moved from space4 into group 0.
    body = body.replace("#ifdef __spirv__", "#if 0", 1)
    body = re.sub(r"register\(b(\d), space4\)", r"register(b\1, space0)", body)
    # Descriptor indices become the fixed resources of their sampler.
    body = re.sub(r"^\s*uint \w+_(?:Texture2D|Texture3D|TextureCube|Sampler)DescriptorIndex : packoffset\([^)]*\);\n",
                  "", body, flags=re.M)
    used = {}
    for kind, (suffix, _) in KINDS.items():
        for name in re.findall(rf"\b(\w+?)_{kind}DescriptorIndex\b", body):
            used.setdefault(name, set()).add(kind)
        body = re.sub(rf"\b(\w+?)_{kind}DescriptorIndex\b", rf"\1_{suffix}", body)
    body = re.sub(r"\b(\w+?)_SamplerDescriptorIndex\b", r"\1_s", body)
    body = sort_cbuffers(body)
    # Varyings get location = semantic index (COLOR0/1 -> 16/17) on both stages so a
    # vertex program's outputs meet the pixel program's inputs regardless of which
    # subset each declares. Vertex inputs already carry XenosRecomp's vk::location.
    def loc(m):
        sem, idx = m[3], int(m[4])
        return f"[[vk::location({idx if sem == 'TEXCOORD' else 16 + idx})]] {m[1]} float4 {m[2]} : {sem}{idx}"
    body = re.sub(r"(?<!\]\] )(in|out) float4 (\w+) : (TEXCOORD|COLOR)(\d+)", loc, body)
    decls = []
    for name in sorted(used):
        if name not in slots:
            raise KeyError(f"sampler {name} not in sheets/shader_samplers.csv")
        i = slots[name]
        for kind in sorted(used[name]):
            suffix, k = KINDS[kind]
            decls.append(f"{kind}<float4> {name}_{suffix} : register(t{4 * i + k}, space1);")
        decls.append(f"SamplerState {name}_s : register(s{4 * i + 3}, space1);")
    return '#include "../wgpu_common.hlsli"\n\n' + "\n".join(decls) + "\n" + body, used


def run(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, (r.stdout + r.stderr).strip()


def main():
    header, rows = read_sheet("shader_programs")
    _, samplers = read_sheet("shader_samplers")
    slots = {r[0]: int(r[1]) for r in samplers}
    col = {c: i for i, c in enumerate(header)}
    only = set(sys.argv[1:])
    (HERE / "wgpu").mkdir(exist_ok=True)
    (HERE / "wgsl").mkdir(exist_ok=True)
    failed = 0
    for r in rows:
        prog, stage = r[col["program"]], r[col["stage"]]
        if only and prog not in only:
            continue
        src = HERE / "hlsl" / f"{prog}.hlsl"
        hlsl, spv, wgsl = HERE / "wgpu" / f"{prog}.hlsl", HERE / "wgpu" / f"{prog}.spv", HERE / "wgsl" / f"{prog}.wgsl"
        try:
            text, _ = convert_source(src.read_text(encoding="utf-8"), slots)
        except (AssertionError, KeyError, ValueError) as e:
            print(f"FAIL {prog}: {e}")
            failed += 1
            continue
        hlsl.write_text(text, encoding="utf-8")
        code, out = run([str(DXC), "-T", "ps_6_0" if stage == "pixel" else "vs_6_0", "-E", "main", "-HV", "2021",
                         "-all-resources-bound", "-spirv", "-fvk-use-dx-layout", "-Wno-ignored-attributes",
                         "-DUNLEASHED_RECOMP", "-Fo", str(spv), str(hlsl)])
        if code:
            print(f"FAIL {prog} (dxc):\n{out[-1500:]}")
            failed += 1
            continue
        code, out = run([str(NAGA), str(spv), str(wgsl)])
        if code:
            print(f"FAIL {prog} (naga):\n{out[-1500:]}")
            failed += 1
            continue
        r[col["wgsl"]] = wgsl.relative_to(RS).as_posix()
        print(f"ok   {prog}")
    with open(SHEETS / "shader_programs.csv", "w", encoding="utf-8-sig", newline="") as fh:
        csv.writer(fh).writerows([header, *rows])
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()

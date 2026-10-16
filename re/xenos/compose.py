"""Compose one Bevy material shader from a converted vertex + pixel program pair.

re/xenos/wgsl holds the game's own vertex and pixel programs (XenosRecomp HLSL -> naga
WGSL). Bevy wants one module with `vertex` / `fragment` entry points and every
resource in the material bind group, so this:
  - prefixes every top-level name of each program (vs_ / ps_) so they cannot collide,
  - renames the entry points to `vertex` / `fragment`,
  - remaps the resources into @group(#{MATERIAL_BIND_GROUP}):
        0  VertexShaderConstants      1  PixelShaderConstants
        2  vertex SharedConstants     3  pixel SharedConstants
        16 + 4*slot (+0 2D, +1 3D, +2 cube texture, +3 sampler) for sampler slot `slot`
    (the converted programs bind texture slot s at 4s..4s+3 of group 1).
The shader code itself is untouched: the result computes what the game's programs do.

usage: python compose.py <vertex.wgsl> <pixel.wgsl> <out.wgsl>
"""
import re
import sys
from pathlib import Path

TOP = re.compile(r"^(?:struct|fn|const|alias|override|var(?:<[^>]*>)?)\s+([A-Za-z_][A-Za-z0-9_]*)", re.M)


def prefix(src, p, entry):
    names = set(TOP.findall(src))
    names.discard("main")
    for n in sorted(names, key=len, reverse=True):
        src = re.sub(rf"\b{re.escape(n)}\b", p + n, src)
    # Entry point.
    src = re.sub(r"\bfn main\(", f"fn {entry}(", src)
    return src


def remap(src, kind):
    def sub(m):
        group, binding = int(m[1]), int(m[2])
        if group == 0:
            uniform = {(0, "vs"): 0, (1, "ps"): 1, (2, "vs"): 2, (2, "ps"): 3}[(binding, kind)]
            return f"@group(#{{MATERIAL_BIND_GROUP}}) @binding({uniform})"
        if group == 1:
            return f"@group(#{{MATERIAL_BIND_GROUP}}) @binding({16 + binding})"
        raise SystemExit(f"unexpected @group({group})")
    return re.sub(r"@group\((\d+)\)\s*@binding\((\d+)\)", sub, src)


def adapt_vertex_output(vs, ps):
    """Clip-space conventions and interpolator count, nothing else:
    - drop the Y flip dxc adds for Vulkan (wgpu's clip space is D3D's, as the game's),
    - depth for Bevy's infinite reverse-Z target: clip z = near (0.1, the Bevy camera's
      near plane), so depth = near / w with w the view depth, exactly as Bevy's own
      projection writes it (the game's x, y, w are kept),
    - keep only the outputs the pixel program reads, TEXCOORDn and COLORn alike (wgpu's
      default limit is 60 inter-stage components; the vertex program writes 18 vec4s)."""
    flip = re.search(r"let (_e\d+) = vs_global\.y;\s*vs_global\.y = -\(\1\);\n", vs)
    if not flip:
        raise SystemExit("vertex program: no Y flip found")
    vs = vs.replace(flip[0], "vs_global.z = 0.1; // Bevy infinite reverse-Z: depth = near / w\n")
    entry = re.search(r"fn fragment\((.*?)\) ->", ps, re.S)
    used = {int(x) for x in re.findall(r"@location\((\d+)\)", entry[1])}
    struct = re.search(r"struct vs_VertexOutput \{\n(.*?)\n\}", vs, re.S)
    lines = struct[1].split("\n")
    loc = lambda l: int(re.search(r"@location\((\d+)\)", l)[1])
    keep = [i for i, l in enumerate(lines) if "@builtin" in l or loc(l) in used]
    missing = used - {loc(lines[i]) for i in keep if "@builtin" not in lines[i]}
    if missing:
        raise SystemExit(f"pixel program reads locations {sorted(missing)} the vertex program does not write")
    vs = vs.replace(struct[0], "struct vs_VertexOutput {\n" + "\n".join(lines[i] for i in keep) + "\n}")
    ret = re.search(r"return vs_VertexOutput\(([^)]*)\);", vs)
    args = [a.strip() for a in ret[1].split(",")]
    vs = vs.replace(ret[0], f"return vs_VertexOutput({', '.join(args[i] for i in keep)});")
    return vs


HOST_SHADOW = """\
#import bevy_pbr::{
    mesh_view_bindings::{view, lights},
    mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    shadows::fetch_directional_shadow,
}

// The host's sun shadow at this fragment (1 = lit), fetched once per fragment.
var<private> chr_shadow: f32 = 1.0;

fn chr_fetch_shadow(frag: vec4<f32>) -> f32 {
    if (lights.n_directional_lights == 0u
        || (lights.directional_lights[0].flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) == 0u) {
        return 1.0;
    }
    // World position from the depth the vertex program wrote (Bevy's projection).
    let uv = (frag.xy - view.viewport.xy) / view.viewport.zw;
    let world_h = view.world_from_clip * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, frag.z, 1.0);
    let world = vec4<f32>(world_h.xyz / world_h.w, 1.0);
    let view_z = dot(vec4<f32>(view.view_from_world[0].z, view.view_from_world[1].z,
        view.view_from_world[2].z, view.view_from_world[3].z), world);
    return fetch_directional_shadow(0u, world, lights.directional_lights[0].direction_to_light, view_z, frag.xy);
}

// One of the pixel program's four shadow-map taps: depth 1 (lit) or 0 (shadowed) against
// the program's own depth 0.5, the taps switching at staggered levels so its bilinear
// blend of them keeps the host shadow's soft edge.
fn chr_shadow_tap(k: i32) -> vec4<f32> {
    return vec4<f32>(select(0.0, 1.0, chr_shadow > (f32(k) + 0.5) * 0.25));
}

"""


def host_shadow(ps):
    """The pixel program takes four taps of the game's shadow map (g_ShadowMapSampler /
    g_VerticalShadowMapSampler) and compares them with the depth the vertex program
    projected through g_MtxLightViewProjection. The host has no such map (its sun shadow
    is Bevy's cascades), so each tap becomes chr_shadow_tap and the host sets the light
    view-projection to project every point to depth 0.5: the program's own comparison and
    filtering then turn the host's shadow into its shadow term. Returns (ps, patched)."""
    tap = re.compile(r"textureSample\(ps_g_\w*ShadowMapSampler_t2d, ps_g_\w*ShadowMapSampler_s, .*\);$", re.M)
    n = len(tap.findall(ps))
    if n == 0:
        return ps, False
    if n % 4:
        raise SystemExit(f"pixel program: {n} shadow-map taps, expected groups of four")
    count = iter(range(n))
    ps = tap.sub(lambda m: f"chr_shadow_tap({next(count) % 4}i);", ps)
    entry = re.search(r"fn fragment\((.*?)\) -> ([^{]*)\{\n", ps, re.S)
    # The fragment position: the program's own vPos input if it takes one.
    own = re.search(r"@builtin\(position\) (\w+): vec4<f32>", entry[1])
    params = entry[1] if own else f"@builtin(position) chr_frag: vec4<f32>, {entry[1]}"
    ps = ps.replace(entry[0], f"fn fragment({params}) -> {entry[2]}{{\n"
                              f"    chr_shadow = chr_fetch_shadow({own[1] if own else 'chr_frag'});\n")
    return ps, True


def compose(vs_path, ps_path):
    vs = Path(vs_path).read_text(encoding="utf-8")
    ps = Path(ps_path).read_text(encoding="utf-8")
    vs = remap(prefix(vs, "vs_", "vertex"), "vs")
    ps = remap(prefix(ps, "ps_", "fragment"), "ps")
    vs = adapt_vertex_output(vs, ps)
    ps, shadowed = host_shadow(ps)
    head = (f"// Composed by re/xenos/compose.py from {Path(vs_path).name} (vertex) and "
            f"{Path(ps_path).name} (pixel): the game's own programs, resources in the material group"
            f"{'; shadow-map taps answered by the host (host_shadow)' if shadowed else ''}.\n\n")
    return head + (HOST_SHADOW if shadowed else "") + vs + "\n\n" + ps


if __name__ == "__main__":
    out = compose(sys.argv[1], sys.argv[2])
    Path(sys.argv[3]).write_text(out, encoding="utf-8", newline="\n")
    print(f"wrote {sys.argv[3]}")

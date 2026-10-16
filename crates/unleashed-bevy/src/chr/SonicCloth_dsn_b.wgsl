// Composed by re/xenos/compose.py from CharactorSSS_@bdn@.wgsl (vertex) and SonicCloth_dsn@@.wgsl (pixel): the game's own programs, resources in the material group; shadow-map taps answered by the host (host_shadow).

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

struct vs_type_u002e_VertexShaderConstants {
    g_MtxProjection: array<vec4<f32>, 4>,
    g_MtxView: array<vec4<f32>, 4>,
    g_MtxWorld: array<vec4<f32>, 4>,
    g_MtxLightViewProjection: array<vec4<f32>, 4>,
    g_MtxPalette: array<vec4<f32>, 75>,
    g_EyePosition: vec4<f32>,
    mrgGlobalLight_Direction: vec4<f32>,
    mrgTexcoordIndex: vec4<f32>,
    g_LightScattering_Ray_Mie_Ray2_Mie2_: vec4<f32>,
    g_LightScattering_ConstG_FogDensity: vec4<f32>,
    g_LightScatteringFarNearScale: vec4<f32>,
    g_LightScatteringColor: vec4<f32>,
}

struct vs_type_u002e_SharedConstants {
    g_Booleans: u32,
    g_SwappedTexcoords: u32,
    g_HalfPixelOffset: vec2<f32>,
    g_AlphaThreshold: f32,
    g_SpecConstantsValue: u32,
}

struct vs_VertexOutput {
    @builtin(position) member: vec4<f32>,
    @location(0) member_1: vec4<f32>,
    @location(1) member_2: vec4<f32>,
    @location(2) member_3: vec4<f32>,
    @location(3) member_4: vec4<f32>,
    @location(4) member_5: vec4<f32>,
    @location(5) member_6: vec4<f32>,
    @location(6) member_7: vec4<f32>,
    @location(7) member_8: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) 
var<uniform> vs_VertexShaderConstants: vs_type_u002e_VertexShaderConstants;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) 
var<uniform> vs_SharedConstants: vs_type_u002e_SharedConstants;
var<private> vs_in_u002e_var_u002e_POSITION0_1: vec4<f32>;
var<private> vs_in_u002e_var_u002e_TEXCOORD0_1: vec4<f32>;
var<private> vs_in_u002e_var_u002e_TEXCOORD1_1: vec4<f32>;
var<private> vs_in_u002e_var_u002e_TEXCOORD2_1: vec4<f32>;
var<private> vs_in_u002e_var_u002e_TEXCOORD3_1: vec4<f32>;
var<private> vs_in_u002e_var_u002e_NORMAL0_1: vec4<u32>;
var<private> vs_in_u002e_var_u002e_TANGENT0_1: vec4<u32>;
var<private> vs_in_u002e_var_u002e_BINORMAL0_1: vec4<u32>;
var<private> vs_in_u002e_var_u002e_BLENDWEIGHT0_1: vec4<f32>;
var<private> vs_in_u002e_var_u002e_BLENDINDICES0_1: vec4<u32>;
var<private> vs_global: vec4<f32> = vec4<f32>(0f, 0f, 0f, 1f);
var<private> vs_out_u002e_var_u002e_TEXCOORD0_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD1_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD2_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD3_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD4_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD5_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD6_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD7_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD8_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD9_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD10_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD11_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD12_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD13_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD14_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_TEXCOORD15_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_COLOR0_: vec4<f32>;
var<private> vs_out_u002e_var_u002e_COLOR1_: vec4<f32>;

fn vs_main_1() {
    var phi_131_: vec4<f32>;
    var phi_138_: vec4<f32>;
    var phi_145_: vec4<f32>;
    var phi_152_: vec4<f32>;
    var phi_188_: vec4<f32>;
    var local: bool;
    var phi_221_: vec4<f32>;
    var local_1: bool;
    var phi_254_: vec4<f32>;
    var phi_288_: vec4<f32>;
    var phi_291_: vec4<f32>;
    var phi_293_: vec4<f32>;
    var phi_295_: vec4<f32>;
    var phi_297_: vec4<f32>;
    var phi_299_: vec4<f32>;
    var phi_301_: i32;
    var local_2: vec4<f32>;
    var local_3: vec4<f32>;
    var local_4: vec4<f32>;
    var local_5: vec4<f32>;
    var local_6: vec4<f32>;
    var local_7: vec4<f32>;
    var local_8: vec4<f32>;
    var local_9: vec4<f32>;
    var local_10: vec4<f32>;
    var local_11: vec4<f32>;
    var local_12: vec4<f32>;
    var local_13: vec4<f32>;
    var phi_462_: vec4<f32>;
    var phi_463_: vec4<f32>;
    var phi_464_: vec4<f32>;
    var phi_465_: vec4<f32>;
    var phi_797_: vec4<f32>;
    var phi_804_: vec4<f32>;

    let _e84 = vs_in_u002e_var_u002e_POSITION0_1;
    let _e85 = vs_in_u002e_var_u002e_TEXCOORD0_1;
    let _e86 = vs_in_u002e_var_u002e_TEXCOORD1_1;
    let _e87 = vs_in_u002e_var_u002e_TEXCOORD2_1;
    let _e88 = vs_in_u002e_var_u002e_TEXCOORD3_1;
    let _e89 = vs_in_u002e_var_u002e_NORMAL0_1;
    let _e90 = vs_in_u002e_var_u002e_TANGENT0_1;
    let _e91 = vs_in_u002e_var_u002e_BINORMAL0_1;
    let _e92 = vs_in_u002e_var_u002e_BLENDWEIGHT0_1;
    let _e93 = vs_in_u002e_var_u002e_BLENDINDICES0_1;
    let _e95 = vs_SharedConstants.g_SwappedTexcoords;
    if ((_e95 & 1u) != 0u) {
        phi_131_ = _e85.yxwz;
    } else {
        phi_131_ = _e85;
    }
    let _e100 = phi_131_;
    if ((_e95 & 2u) != 0u) {
        phi_138_ = _e86.yxwz;
    } else {
        phi_138_ = _e86;
    }
    let _e105 = phi_138_;
    if ((_e95 & 4u) != 0u) {
        phi_145_ = _e87.yxwz;
    } else {
        phi_145_ = _e87;
    }
    let _e110 = phi_145_;
    if ((_e95 & 8u) != 0u) {
        phi_152_ = _e88.yxwz;
    } else {
        phi_152_ = _e88;
    }
    let _e115 = phi_152_;
    let _e117 = vs_SharedConstants.g_SpecConstantsValue;
    switch bitcast<i32>(0u) {
        default: {
            let _e120 = ((_e117 & 1u) != 0u);
            local = _e120;
            local_1 = _e120;
            if _e120 {
                phi_188_ = vec4<f32>((select(0f, -1f, ((_e89.x & 1024u) != 0u)) + (f32((_e89.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e89.x & 2097152u) != 0u)) + (f32(((_e89.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e89.x & 2147483648u) != 0u)) + (f32(((_e89.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_188_ = bitcast<vec4<f32>>(_e89);
                break;
            }
        }
    }
    let _e150 = phi_188_;
    let _e155 = vec4<f32>(_e150.x, _e150.y, _e150.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    switch bitcast<i32>(0u) {
        default: {
            let _e158 = local;
            if _e158 {
                phi_221_ = vec4<f32>((select(0f, -1f, ((_e90.x & 1024u) != 0u)) + (f32((_e90.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e90.x & 2097152u) != 0u)) + (f32(((_e90.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e90.x & 2147483648u) != 0u)) + (f32(((_e90.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_221_ = bitcast<vec4<f32>>(_e90);
                break;
            }
        }
    }
    let _e188 = phi_221_;
    let _e193 = vec4<f32>(vec4<f32>(0f, 0f, 0f, 0f).x, _e188.x, _e188.y, _e188.z);
    switch bitcast<i32>(0u) {
        default: {
            let _e196 = local_1;
            if _e196 {
                phi_254_ = vec4<f32>((select(0f, -1f, ((_e91.x & 1024u) != 0u)) + (f32((_e91.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e91.x & 2097152u) != 0u)) + (f32(((_e91.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e91.x & 2147483648u) != 0u)) + (f32(((_e91.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_254_ = bitcast<vec4<f32>>(_e91);
                break;
            }
        }
    }
    let _e226 = phi_254_;
    let _e233 = vec4<f32>(_e93.wxyz);
    let _e235 = -(_e226.zyx);
    let _e236 = max(_e235, _e235);
    let _e243 = vs_SharedConstants.g_Booleans;
    phi_462_ = _e84;
    phi_463_ = vec4<f32>(_e236.x, _e236.y, _e236.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    phi_464_ = _e193;
    phi_465_ = _e155;
    if ((_e243 & 1u) != 0u) {
        let _e249 = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(vec4<f32>(0f, 0f, 0f, 0f))) > vec4<f32>(0f, 0f, 0f, 0f)));
        let _e253 = select(0f, 1f, (-(abs(0f)) > 0f));
        let _e258 = vec4<f32>(_e253, _e193.y, _e193.z, _e193.w);
        phi_288_ = vec4<f32>(_e226.x, _e226.y, _e226.z, vec4<f32>(0f, 0f, 0f, 0f).w);
        phi_291_ = _e249;
        phi_293_ = _e249;
        phi_295_ = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(_e258.xxxx)) > vec4<f32>(0f, 0f, 0f, 0f)));
        phi_297_ = _e258;
        phi_299_ = vec4<f32>(_e155.x, _e155.y, _e155.z, select(0f, 1f, (-(abs(_e253)) > 0f)));
        phi_301_ = 0i;
        loop {
            let _e274 = phi_288_;
            let _e276 = phi_291_;
            let _e278 = phi_293_;
            let _e280 = phi_295_;
            let _e282 = phi_297_;
            let _e284 = phi_299_;
            let _e286 = phi_301_;
            local_2 = _e284;
            local_3 = _e280;
            local_4 = _e278;
            local_5 = _e276;
            local_6 = _e282;
            local_7 = _e282;
            local_8 = _e282;
            local_9 = _e274;
            local_10 = _e274;
            local_11 = _e284;
            local_12 = _e284;
            local_13 = _e284;
            if (_e286 < 3i) {
                continue;
            } else {
                break;
            }
            continuing {
                let _e289 = (_e282.x == 0f);
                let _e294 = (_e282.xx + vec2<f32>(-2f, -3f));
                let _e299 = (_e294.x == 0f);
                let _e305 = (_e294.y == 0f);
                let _e307 = select(select(select(_e92.y, _e92.x, _e289), _e92.z, _e299), _e92.w, _e305);
                let _e312 = vec4<f32>(_e274.x, _e274.y, _e274.z, _e307);
                let _e332 = i32(clamp(floor(((select(select(select(_e233.z, _e233.y, _e289), _e233.w, _e299), _e233.x, _e305) * 3f) + 0.5f)), -256f, 255f));
                let _e333 = _e312.wwww;
                let _e339 = vs_VertexShaderConstants.g_MtxPalette[min(_e332, 227i)];
                let _e344 = (1i + _e332);
                let _e350 = vs_VertexShaderConstants.g_MtxPalette[min(_e344, 227i)];
                let _e355 = (2i + _e332);
                let _e361 = vs_VertexShaderConstants.g_MtxPalette[min(_e355, 227i)];
                phi_288_ = _e312;
                phi_291_ = ((_e333 * select(vec4<f32>(0f, 0f, 0f, 0f), _e339, vec4((_e332 < 228i))).wzyx) + _e276);
                phi_293_ = ((_e333 * select(vec4<f32>(0f, 0f, 0f, 0f), _e350, vec4((_e344 < 228i))).wzyx) + _e278);
                phi_295_ = ((_e333 * select(vec4<f32>(0f, 0f, 0f, 0f), _e361, vec4((_e355 < 228i))).wzyx) + _e280);
                phi_297_ = vec4<f32>((1f + _e282.x), _e282.y, _e282.z, _e282.w);
                phi_299_ = vec4<f32>(_e284.x, _e284.y, _e284.z, (_e307 + _e284.w));
                phi_301_ = (_e286 + 1i);
            }
        }
        let _e370 = (3f * _e233.x);
        let _e372 = local_2;
        let _e374 = (1f - _e372.w);
        let _e383 = i32(clamp(floor((_e370 + 0.5f)), -256f, 255f));
        let _e384 = vec4<f32>(_e374, vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxxx;
        let _e385 = (2i + _e383);
        let _e391 = vs_VertexShaderConstants.g_MtxPalette[min(_e385, 227i)];
        let _e396 = local_3;
        let _e397 = ((_e384 * select(vec4<f32>(0f, 0f, 0f, 0f), _e391, vec4((_e385 < 228i))).wzyx) + _e396);
        let _e398 = (1i + _e383);
        let _e404 = vs_VertexShaderConstants.g_MtxPalette[min(_e398, 227i)];
        let _e409 = local_4;
        let _e410 = ((_e384 * select(vec4<f32>(0f, 0f, 0f, 0f), _e404, vec4((_e398 < 228i))).wzyx) + _e409);
        let _e416 = vs_VertexShaderConstants.g_MtxPalette[min(_e383, 227i)];
        let _e421 = local_5;
        let _e422 = ((_e384 * select(vec4<f32>(0f, 0f, 0f, 0f), _e416, vec4((_e383 < 228i))).wzyx) + _e421);
        let _e427 = vec4<f32>(_e370, max(_e84.y, _e84.y), max(_e84.z, _e84.z), max(_e84.w, _e84.w));
        let _e436 = vec4<f32>(max(_e84.x, _e84.x), _e427.y, _e427.z, _e427.w).wzxy;
        let _e442 = vec4<f32>(dot(_e422.xywz, _e436), _e84.y, _e84.z, _e84.w);
        let _e449 = vec4<f32>(_e442.x, dot(_e410.xywz, _e436), _e442.z, _e442.w);
        let _e458 = local_6;
        let _e465 = vec4<f32>(max(_e458.y, _e458.y), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e467 = local_7;
        let _e474 = vec4<f32>(_e465.x, _e465.y, max(_e467.w, _e467.w), _e465.w);
        let _e476 = local_8;
        let _e484 = _e410.ywz;
        let _e485 = vec4<f32>(_e474.x, max(_e476.z, _e476.z), _e474.z, _e474.w).zxy;
        let _e487 = _e397.ywz;
        let _e490 = local_9;
        let _e494 = -(_e490.zxy);
        let _e500 = vec4<f32>(vec4<f32>().x, dot(_e484, _e494), vec4<f32>().z, vec4<f32>().w);
        let _e501 = _e422.ywz;
        let _e505 = local_11;
        let _e512 = vec4<f32>(vec4<f32>().x, max(_e505.y, _e505.y), vec4<f32>().z, vec4<f32>().w);
        let _e518 = vec4<f32>(dot(_e487, _e494), _e500.y, _e500.z, _e500.w);
        let _e520 = local_12;
        let _e527 = vec4<f32>(max(_e520.x, _e520.x), _e512.y, _e512.z, _e512.w);
        let _e535 = local_13;
        let _e543 = vec4<f32>(_e527.x, _e527.y, max(_e535.z, _e535.z), _e527.w).zxy;
        let _e549 = vec4<f32>(dot(_e501, _e543), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e555 = vec4<f32>(_e549.x, dot(_e484, _e543), _e549.z, _e549.w);
        phi_462_ = vec4<f32>(_e449.x, _e449.y, dot(_e397.xywz, _e436), _e449.w);
        phi_463_ = vec4<f32>(_e518.x, _e518.y, dot(_e501, _e494), _e518.w);
        phi_464_ = vec4<f32>(_e374, dot(_e501, _e485), dot(_e484, _e485), dot(_e487, _e485));
        phi_465_ = vec4<f32>(_e555.x, _e555.y, dot(_e487, _e543), _e555.w);
    }
    let _e563 = phi_462_;
    let _e565 = phi_463_;
    let _e567 = phi_464_;
    let _e569 = phi_465_;
    let _e572 = vs_VertexShaderConstants.mrgTexcoordIndex[0i];
    let _e578 = vec4<f32>(trunc(_e572), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e582 = vs_VertexShaderConstants.g_MtxWorld[2i];
    let _e583 = _e582.xyz;
    let _e592 = vec4<f32>((_e582.x * _e567.w), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e601 = vec4<f32>(_e592.x, (_e582.y * _e567.w), _e592.z, _e592.w);
    let _e620 = vs_VertexShaderConstants.g_MtxWorld[1i];
    let _e625 = _e620.xyz;
    let _e636 = ((_e569.yyy * _e625) + (_e569.zzz * _e583).xyz);
    let _e640 = vs_VertexShaderConstants.g_MtxWorld[0i];
    let _e641 = _e640.xzy;
    let _e644 = ((_e569.xxx * _e641) + _e636.xzy);
    let _e648 = ((_e567.yyy * _e641) + ((_e567.zzz * _e625) + vec4<f32>(_e601.x, _e601.y, (_e582.z * _e567.w), _e601.w).xyz).xzy);
    let _e652 = ((_e565.zzz * _e641) + ((_e565.yyy * _e625) + (_e565.xxx * _e583).xyz).xzy);
    let _e657 = ((_e563.xxxx * _e640.wxzy) + ((_e563.yyyy * _e620.xzyw) + (_e563.zzzz * _e582.xzyw)).wxyz);
    let _e661 = vs_VertexShaderConstants.g_MtxWorld[3i];
    let _e665 = ((_e563.w * _e661.w) + _e657.x);
    let _e670 = ((_e563.www * _e661.xyz) + _e657.ywz);
    let _e675 = vec4<f32>(_e670.x, _e670.y, _e670.z, _e636.z);
    let _e676 = _e652.yxz;
    let _e678 = _e648.yxz;
    let _e679 = dot(_e678, _e678);
    let _e694 = _e644.yxz;
    let _e695 = dot(_e694, _e694);
    let _e696 = vec4<f32>(_e665, _e679, clamp(inverseSqrt(abs(_e679)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e695);
    let _e700 = vs_VertexShaderConstants.g_EyePosition;
    let _e702 = (-(_e670.xyz) + _e700.xyz);
    let _e707 = vec4<f32>(_e569.x, _e702.x, _e702.y, _e702.z);
    let _e708 = _e670.zzzz;
    let _e711 = vs_VertexShaderConstants.g_MtxView[2i];
    let _e716 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[0i];
    let _e719 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[1i];
    let _e720 = (_e716 + _e719);
    let _e723 = vs_VertexShaderConstants.g_MtxLightViewProjection[2i];
    let _e729 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[0i];
    let _e732 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[1i];
    let _e739 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[3i];
    let _e741 = _e702.zxy;
    let _e745 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[0i];
    let _e751 = vec4<f32>((_e745 * max(_e739, _e739)), _e707.y, _e707.z, _e707.w);
    let _e752 = _e670.yyyy;
    let _e755 = vs_VertexShaderConstants.g_MtxLightViewProjection[1i];
    let _e762 = vs_VertexShaderConstants.g_MtxView[1i];
    let _e767 = _e670.xxxx;
    let _e770 = vs_VertexShaderConstants.g_MtxView[0i];
    let _e774 = ((_e767 * _e770.xywz) + ((_e752 * _e762.xyzw) + (_e708 * _e711.xwzy).xwzy).xywz);
    let _e777 = vs_VertexShaderConstants.g_MtxLightViewProjection[0i];
    let _e781 = ((_e767 * _e777.xwzy) + ((_e752 * _e755.wxzy) + (_e708 * _e723.xyzw).wxzy).yxzw);
    let _e785 = vs_VertexShaderConstants.g_MtxLightViewProjection[3i];
    let _e795 = vec4<f32>(_e696.x, _e696.y, clamp(inverseSqrt(abs(dot(_e741, _e741))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e696.w);
    let _e804 = vs_VertexShaderConstants.g_MtxView[3i];
    let _e808 = ((_e795.xxx * _e804.wxy) + _e774.zxy);
    let _e813 = vec4<f32>(_e751.x, _e808.x, _e808.y, _e808.z);
    let _e817 = ((_e665 * _e804.z) + _e774.w);
    let _e826 = vs_VertexShaderConstants.g_MtxProjection[3i];
    let _e830 = -(_e817);
    let _e832 = (_e751.yzw * _e795.zzz).zxy;
    let _e834 = vs_VertexShaderConstants.mrgGlobalLight_Direction;
    let _e837 = dot(_e832, -(_e834.zxy));
    let _e842 = vec4<f32>(_e813.x, _e837, _e813.z, _e813.w);
    let _e847 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[2i];
    let _e851 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[1i];
    let _e856 = vs_VertexShaderConstants.g_MtxProjection[2i];
    let _e868 = (_e781.zy + _e785.zw);
    let _e871 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[2i];
    let _e876 = vs_VertexShaderConstants.g_MtxProjection[1i];
    let _e884 = vs_VertexShaderConstants.g_MtxProjection[0i];
    let _e888 = ((_e842.zzzz * _e884.xyzw) + ((_e842.wwww * _e876.xzyw) + ((vec4<f32>(_e817, vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxxx * _e856.xwzy) + (_e808.xxxx * _e826.xwzy)).xzwy).xzyw);
    let _e896 = (vec4<f32>((_e781.x + _e785.x), (-(_e785.y) + -(max(_e781.w, _e781.w))), clamp(log2(abs(((_e837 * _e847) + _e851))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), ((_e871 * clamp(((-(_e732) + _e830) * clamp((1f / (_e729 - _e732)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)), 0f, 1f)) * -(_e720))).wxyz * vec4<f32>(1.442695f, 0.5f, 0.5f, 1.5f));
    let _e908 = ((_e868.yy * vec2<f32>(0.5f, 0.5f)) + _e896.yz);
    let _e913 = vec4<f32>(_e908.x, _e908.y, _e868.x, _e868.y);
    let _e916 = (_e644.yzx * vec4<f32>(vec4<f32>().x, clamp(inverseSqrt(abs(_e695)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().z, vec4<f32>().w).yyy);
    let _e921 = vec4<f32>(_e916.x, _e916.y, _e916.z, vec4<f32>().w);
    let _e925 = dot(_e916.xzy, _e832);
    let _e927 = exp2(_e896.x);
    let _e929 = vec4<f32>(_e817, _e925, _e927, (1f - _e927));
    let _e936 = vec4<f32>(_e842.x, _e842.y, clamp((1f / exp2(_e896.w)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e842.w);
    let _e939 = (_e936.xy * _e936.zy);
    let _e946 = (vec4<f32>(_e578.x, _e939.x, _e939.y, _e578.w).xxz + vec3<f32>(-2f, -3f, 1f));
    let _e951 = vec4<f32>(_e936.x, _e946.x, _e946.y, _e946.z);
    let _e953 = clamp((1f / _e720), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e958 = vec4<f32>(_e953, _e951.y, _e951.z, _e951.w);
    let _e962 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[2i];
    let _e965 = ((_e946.z * _e962) + _e939.x);
    let _e981 = vec4<f32>(_e958.x, (_e965 * _e953), _e958.z, _e958.w);
    let _e984 = vs_VertexShaderConstants.g_LightScatteringColor;
    let _e986 = (_e981.xyy * _e984.xyz);
    let _e991 = vec4<f32>(_e981.x, _e986.x, _e986.y, _e986.z);
    let _e998 = vec4<f32>((_e986.x * _e965), _e991.y, _e991.z, _e991.w);
    let _e1001 = (_e998.xzw * _e929.www);
    let _e1008 = vs_VertexShaderConstants.g_LightScatteringFarNearScale;
    let _e1010 = (_e1001.xyz * _e1008.www);
    let _e1015 = vec4<f32>(_e1010.x, _e1010.y, _e1010.z, _e896.w);
    let _e1017 = (-(_e925) > 0f);
    phi_797_ = vec4<f32>(_e1001.x, _e1001.y, _e1001.z, _e998.w);
    if !(_e1017) {
        let _e1019 = _e916.zyx;
        let _e1020 = max(_e1019, _e1019);
        phi_797_ = vec4<f32>(_e1020.x, _e1020.y, _e1020.z, _e998.w);
    }
    let _e1027 = phi_797_;
    phi_804_ = _e1027;
    if _e1017 {
        let _e1029 = -(_e916.zyx);
        let _e1030 = max(_e1029, _e1029);
        phi_804_ = vec4<f32>(_e1030.x, _e1030.y, _e1030.z, _e1027.w);
    }
    let _e1037 = phi_804_;
    let _e1038 = max(_e888, _e888);
    let _e1040 = dot(_e832, _e1037.zxy);
    let _e1047 = vec4<f32>(_e675.x, _e675.y, _e675.z, max(_e830, _e830));
    let _e1048 = (1f - clamp(_e1040, 0f, 1f));
    let _e1049 = select(select(select(_e105.xy, _e100.xy, (_e578.xx == vec2<f32>(0f, 0f))).xy, _e110.xy, (_e958.yy == vec2<f32>(0f, 0f))).xy, _e115.xy, (_e958.zz == vec2<f32>(0f, 0f))).xy;
    let _e1050 = max(_e1049, _e1049);
    let _e1055 = vec4<f32>(_e1050.x, _e1050.y, vec4<f32>(0f, 0f, 0f, 0f).z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1060 = vec4<f32>(_e1055.x, _e1055.y, 0f, _e1055.w);
    let _e1066 = (_e648.xzy * _e696.zzz).xyz;
    let _e1067 = max(_e1066, _e1066);
    let _e1072 = vec4<f32>(_e1067.x, _e1067.y, _e1067.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1078 = (_e652.xzy * vec4<f32>(vec4<f32>().x, vec4<f32>().y, clamp(inverseSqrt(abs(dot(_e676, _e676))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().w).zzz).xyz;
    let _e1079 = max(_e1078, _e1078);
    let _e1084 = vec4<f32>(_e1079.x, _e1079.y, _e1079.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1092 = (_e1048 * _e1048);
    let _e1095 = clamp((1f - _e1040), 0f, 1f);
    let _e1101 = vec4<f32>(_e1095, _e929.y, _e929.z, _e929.w).zzzx;
    let _e1109 = vec4<f32>(_e1015.x, _e1015.y, _e1015.z, (_e1095 * _e1095));
    let _e1118 = vec4<f32>(_e921.x, _e921.y, _e921.z, ((((_e1092 * _e1092) * _e1048) * 0.9638459f) + 0.036154065f)).zyxw;
    let _e1121 = vs_SharedConstants.g_HalfPixelOffset;
    let _e1125 = (_e1038.xy + (_e1121 * _e1038.w));
    vs_global = vec4<f32>(_e1125.x, _e1125.y, _e1038.z, _e1038.w);
    vs_out_u002e_var_u002e_TEXCOORD0_ = vec4<f32>(_e1060.x, _e1060.y, _e1060.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD1_ = max(_e1047, _e1047);
    vs_out_u002e_var_u002e_TEXCOORD2_ = max(_e1118, _e1118);
    vs_out_u002e_var_u002e_TEXCOORD3_ = max(_e913, _e913);
    vs_out_u002e_var_u002e_TEXCOORD4_ = max(_e1101, _e1101);
    vs_out_u002e_var_u002e_TEXCOORD5_ = max(_e1109, _e1109);
    vs_out_u002e_var_u002e_TEXCOORD6_ = vec4<f32>(_e1072.x, _e1072.y, _e1072.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD7_ = vec4<f32>(_e1084.x, _e1084.y, _e1084.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD8_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD9_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD10_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD11_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD12_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD13_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD14_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD15_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_COLOR0_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_COLOR1_ = vec4<f32>(0f, 0f, 0f, 0f);
    return;
}

@vertex 
fn vertex(@location(0) in_u002e_var_u002e_POSITION0_: vec4<f32>, @location(4) in_u002e_var_u002e_TEXCOORD0_: vec4<f32>, @location(5) in_u002e_var_u002e_TEXCOORD1_: vec4<f32>, @location(6) in_u002e_var_u002e_TEXCOORD2_: vec4<f32>, @location(7) in_u002e_var_u002e_TEXCOORD3_: vec4<f32>, @location(1) in_u002e_var_u002e_NORMAL0_: vec4<u32>, @location(2) in_u002e_var_u002e_TANGENT0_: vec4<u32>, @location(3) in_u002e_var_u002e_BINORMAL0_: vec4<u32>, @location(10) in_u002e_var_u002e_BLENDWEIGHT0_: vec4<f32>, @location(9) in_u002e_var_u002e_BLENDINDICES0_: vec4<u32>) -> vs_VertexOutput {
    vs_in_u002e_var_u002e_POSITION0_1 = in_u002e_var_u002e_POSITION0_;
    vs_in_u002e_var_u002e_TEXCOORD0_1 = in_u002e_var_u002e_TEXCOORD0_;
    vs_in_u002e_var_u002e_TEXCOORD1_1 = in_u002e_var_u002e_TEXCOORD1_;
    vs_in_u002e_var_u002e_TEXCOORD2_1 = in_u002e_var_u002e_TEXCOORD2_;
    vs_in_u002e_var_u002e_TEXCOORD3_1 = in_u002e_var_u002e_TEXCOORD3_;
    vs_in_u002e_var_u002e_NORMAL0_1 = in_u002e_var_u002e_NORMAL0_;
    vs_in_u002e_var_u002e_TANGENT0_1 = in_u002e_var_u002e_TANGENT0_;
    vs_in_u002e_var_u002e_BINORMAL0_1 = in_u002e_var_u002e_BINORMAL0_;
    vs_in_u002e_var_u002e_BLENDWEIGHT0_1 = in_u002e_var_u002e_BLENDWEIGHT0_;
    vs_in_u002e_var_u002e_BLENDINDICES0_1 = in_u002e_var_u002e_BLENDINDICES0_;
    vs_main_1();
    vs_global.z = 0.1; // Bevy infinite reverse-Z: depth = near / w
    let _e42 = vs_global;
    let _e43 = vs_out_u002e_var_u002e_TEXCOORD0_;
    let _e44 = vs_out_u002e_var_u002e_TEXCOORD1_;
    let _e45 = vs_out_u002e_var_u002e_TEXCOORD2_;
    let _e46 = vs_out_u002e_var_u002e_TEXCOORD3_;
    let _e47 = vs_out_u002e_var_u002e_TEXCOORD4_;
    let _e48 = vs_out_u002e_var_u002e_TEXCOORD5_;
    let _e49 = vs_out_u002e_var_u002e_TEXCOORD6_;
    let _e50 = vs_out_u002e_var_u002e_TEXCOORD7_;
    let _e51 = vs_out_u002e_var_u002e_TEXCOORD8_;
    let _e52 = vs_out_u002e_var_u002e_TEXCOORD9_;
    let _e53 = vs_out_u002e_var_u002e_TEXCOORD10_;
    let _e54 = vs_out_u002e_var_u002e_TEXCOORD11_;
    let _e55 = vs_out_u002e_var_u002e_TEXCOORD12_;
    let _e56 = vs_out_u002e_var_u002e_TEXCOORD13_;
    let _e57 = vs_out_u002e_var_u002e_TEXCOORD14_;
    let _e58 = vs_out_u002e_var_u002e_TEXCOORD15_;
    let _e59 = vs_out_u002e_var_u002e_COLOR0_;
    let _e60 = vs_out_u002e_var_u002e_COLOR1_;
    return vs_VertexOutput(_e42, _e43, _e44, _e45, _e46, _e47, _e48, _e49, _e50);
}


struct ps_type_u002e_PixelShaderConstants {
    g_Diffuse: vec4<f32>,
    g_Ambient: vec4<f32>,
    g_Specular: vec4<f32>,
    g_PowerGlossLevel: vec4<f32>,
    g_EyePosition: vec4<f32>,
    g_EyeDirection: vec4<f32>,
    mrgGlobalLight_Direction: vec4<f32>,
    mrgGlobalLight_Diffuse: vec4<f32>,
    mrgGlobalLight_Specular: vec4<f32>,
    mrgLocalLight0_Position: vec4<f32>,
    mrgLocalLight0_Color: vec4<f32>,
    mrgLocalLight0_Range: vec4<f32>,
    mrgLocalLight1_Position: vec4<f32>,
    mrgLocalLight1_Color: vec4<f32>,
    mrgLocalLight1_Range: vec4<f32>,
    mrgLocalLight2_Position: vec4<f32>,
    mrgLocalLight2_Color: vec4<f32>,
    mrgLocalLight2_Range: vec4<f32>,
    mrgLocalLight3_Position: vec4<f32>,
    mrgLocalLight3_Color: vec4<f32>,
    mrgLocalLight3_Range: vec4<f32>,
    mrgLocalLight4_Position: vec4<f32>,
    mrgLocalLight4_Color: vec4<f32>,
    mrgLocalLight4_Range: vec4<f32>,
    mrgEyeLight_Diffuse: vec4<f32>,
    mrgEyeLight_Specular: vec4<f32>,
    mrgEyeLight_Range: vec4<f32>,
    mrgEyeLight_Attribute: vec4<f32>,
    g_ShadowMapParams: vec4<f32>,
    g_aLightField: array<vec4<f32>, 6>,
    g_NPCClothParam: vec4<f32>,
}

struct ps_type_u002e_SharedConstants {
    g_Booleans: u32,
    g_SwappedTexcoords: u32,
    g_HalfPixelOffset: vec2<f32>,
    g_AlphaThreshold: f32,
    g_SpecConstantsValue: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(16) 
var ps_g_ShadowMapSampler_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(19) 
var ps_g_ShadowMapSampler_s: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(20) 
var ps_sampDif_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(23) 
var ps_sampDif_s: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(32) 
var ps_sampNrm_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(35) 
var ps_sampNrm_s: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(40) 
var ps_sampSpec_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(43) 
var ps_sampSpec_s: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) 
var<uniform> ps_PixelShaderConstants: ps_type_u002e_PixelShaderConstants;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) 
var<uniform> ps_SharedConstants: ps_type_u002e_SharedConstants;
var<private> ps_global: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD0_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD1_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD2_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD3_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD4_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD5_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD6_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD7_1: vec4<f32>;
var<private> ps_global_1: bool;
var<private> ps_out_u002e_var_u002e_SV_Target0_: vec4<f32>;

fn ps_main_1() {
    var phi_169_: vec4<f32>;
    var phi_515_: vec4<f32>;
    var phi_518_: vec4<f32>;
    var phi_520_: vec4<f32>;
    var phi_522_: vec4<f32>;
    var phi_524_: vec4<f32>;
    var phi_526_: vec4<f32>;
    var phi_528_: i32;
    var local: vec4<f32>;
    var local_1: vec4<f32>;
    var local_2: vec4<f32>;
    var local_3: vec4<f32>;
    var local_4: vec4<f32>;
    var local_5: vec4<f32>;
    var local_6: vec4<f32>;
    var local_7: vec4<f32>;
    var local_8: vec4<f32>;
    var local_9: vec4<f32>;
    var local_10: vec4<f32>;
    var local_11: vec4<f32>;
    var local_12: vec4<f32>;
    var local_13: vec4<f32>;
    var local_14: vec4<f32>;
    var local_15: vec4<f32>;
    var local_16: vec4<f32>;
    var local_17: vec4<f32>;

    let _e91 = ps_global;
    let _e92 = ps_in_u002e_var_u002e_TEXCOORD0_1;
    let _e93 = ps_in_u002e_var_u002e_TEXCOORD1_1;
    let _e94 = ps_in_u002e_var_u002e_TEXCOORD2_1;
    let _e95 = ps_in_u002e_var_u002e_TEXCOORD3_1;
    let _e96 = ps_in_u002e_var_u002e_TEXCOORD4_1;
    let _e97 = ps_in_u002e_var_u002e_TEXCOORD5_1;
    let _e98 = ps_in_u002e_var_u002e_TEXCOORD6_1;
    let _e99 = ps_in_u002e_var_u002e_TEXCOORD7_1;
    let _e100 = ps_global_1;
    let _e110 = ps_PixelShaderConstants.g_EyePosition;
    let _e111 = _e110.xyz;
    let _e112 = (-(_e93.xyz) + _e111);
    let _e113 = _e112.zxy;
    let _e126 = vec4<f32>(vec4<f32>().x, clamp(inverseSqrt(abs(dot(_e113, _e113))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().z, vec4<f32>().w);
    let _e130 = (_e112.xyz * _e126.yyy);
    let _e135 = vec4<f32>(_e130.x, _e130.y, _e130.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    phi_169_ = _e94;
    if ((1f - (select(0f, 1f, (clamp((1f / ((_e91.xy - vec2<f32>(0.5f, 0.5f)) * vec2<f32>(select(-1f, 1f, _e100), 1f)).x), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) >= 0f)) * 2f)) > 0f) {
        let _e139 = -(_e94.xyz);
        let _e140 = max(_e139, _e139);
        phi_169_ = vec4<f32>(_e140.x, _e140.y, _e140.z, _e94.w);
    }
    let _e147 = phi_169_;
    let _e149 = textureSample(ps_sampNrm_t2d, ps_sampNrm_s, _e92.xy);
    let _e154 = vec4<f32>(_e149.y, _e149.x, _e149.w, _e126.w);
    let _e162 = vec4<f32>(((_e149.y * 2f) + -1f), _e154.y, _e154.z, _e154.w);
    let _e171 = vec4<f32>((dot(_e162.yy, _e162.zz) + -1f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e172 = _e99.zxy;
    let _e174 = _e98.zxy;
    let _e183 = vec4<f32>(_e92.x, _e92.y, _e92.z, clamp(inverseSqrt(abs(dot(_e172, _e172))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e186 = (_e183.www * _e99.xzy);
    let _e199 = vec4<f32>(_e183.x, _e183.y, clamp(inverseSqrt(abs(dot(_e174, _e174))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e183.w);
    let _e201 = (_e199.zzz * _e174);
    let _e213 = ((vec4<f32>(_e162.x, _e186.x, _e186.y, _e186.z) * _e162.xxxx) + (vec4<f32>(_e171.x, _e201.x, _e201.y, _e201.z).xzwy * _e171.xxxx).xywz);
    let _e218 = clamp((1f / _e95.w), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e223 = vec4<f32>(_e218, vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e229 = vec4<f32>(_e223.x, (_e218 + _e218), _e223.z, _e223.w);
    let _e236 = vec4<f32>(_e229.x, _e229.y, sqrt(abs((1f - _e213.x))), _e229.w);
    let _e239 = (_e236.yxxy * _e95.xxyy);
    let _e241 = textureSample(ps_sampSpec_t2d, ps_sampSpec_s, _e199.xy);
    let _e242 = _e239.yz;
    let _e243 = textureDimensions(ps_g_ShadowMapSampler_t2d, 0i);
    let _e244 = vec2<f32>(_e243);
    let _e247 = chr_shadow_tap(0i);
    let _e251 = chr_shadow_tap(1i);
    let _e255 = chr_shadow_tap(2i);
    let _e259 = chr_shadow_tap(3i);
    let _e268 = select(fract(((_e242 * _e244) - vec2<f32>(0.5f, 0.5f))), vec2<f32>(0f, 0f), ((bitcast<vec2<u32>>(_e242) & vec2<u32>(2147483647u, 2147483647u)) > vec2<u32>(2139095040u, 2139095040u)));
    let _e271 = ps_PixelShaderConstants.g_PowerGlossLevel[1i];
    let _e285 = ps_PixelShaderConstants.g_aLightField[1i];
    let _e289 = ps_PixelShaderConstants.g_aLightField[0i];
    let _e292 = (_e285.xyz + -(_e289.xyz));
    let _e297 = vec4<f32>(_e162.x, _e292.x, _e292.y, _e292.z);
    let _e302 = ps_PixelShaderConstants.g_aLightField[3i];
    let _e306 = ps_PixelShaderConstants.g_aLightField[2i];
    let _e309 = (_e302.xyz + -(_e306.xyz));
    let _e313 = ps_PixelShaderConstants.g_aLightField[5i];
    let _e317 = ps_PixelShaderConstants.g_aLightField[4i];
    let _e320 = (_e313.xyz + -(_e317.xyz));
    let _e321 = max(max(_e95.z, _e95.z), max(0f, 0f));
    let _e322 = _e147.zxy;
    let _e325 = (1f - _e268.x);
    let _e329 = (1f - _e268.y);
    let _e340 = ps_PixelShaderConstants.mrgGlobalLight_Direction;
    let _e343 = (_e130.xyz + -(_e340.xyz));
    let _e348 = vec4<f32>(vec4<f32>(0f, 0f, 0f, 0f).x, _e343.x, _e343.y, _e343.z);
    let _e356 = vec4<f32>(_e199.x, _e199.y, clamp(inverseSqrt(abs(dot(_e322, _e322))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e199.w);
    let _e357 = _e343.zxy;
    let _e362 = (_e356.zzz * _e147.xyz);
    let _e370 = (vec4<f32>(_e362.x, _e362.y, _e362.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, ((-(_e321) * select(0f, 1f, (_e321 >= _e95.w))) + _e321)).w) * _e236.zzzx);
    let _e374 = (_e213.ywz + _e370.xyz);
    let _e379 = vec4<f32>(_e374.x, _e374.y, _e374.z, _e370.w);
    let _e390 = vec4<f32>(_e356.x, _e356.y, (-1f + max(_e239.w, _e239.w)), _e356.w);
    let _e391 = _e374.zxy;
    let _e394 = dot(_e391, -(_e340.zxy));
    let _e400 = vec4<f32>(_e135.x, _e135.y, _e135.z, clamp(_e394, 0f, 1f));
    let _e407 = vec4<f32>(_e390.x, _e390.y, _e390.z, (-1f + _e239.x));
    let _e409 = abs(_e407.wz);
    let _e411 = clamp(max(_e409, _e409), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e416 = vec4<f32>(_e407.x, _e407.y, _e411.x, _e411.y);
    let _e419 = _e374.xyz;
    let _e425 = dot(vec4<f32>((_e325 * _e329), (_e329 * _e268.x), (_e325 * _e268.y), (_e268.x * _e268.y)).wzxy, select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (vec4<f32>(_e247.x, _e251.x, _e255.x, _e259.x) >= _e370.wwww)).wzxy);
    let _e442 = -(_e394);
    let _e449 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, clamp(max(_e442, _e442), 0f, 1f));
    let _e451 = (vec4<f32>((_e271 * 10f), (0.5f * _e374.z), (0.5f * _e374.y), (0.5f * _e374.x)).yxzw + vec4<f32>(0.5f, 2f, 0.5f, 0.5f));
    let _e454 = ps_PixelShaderConstants.g_ShadowMapParams[0i];
    let _e455 = -(_e454);
    let _e462 = (_e320.zyx * _e451.xxx);
    let _e469 = (_e309.yzx * _e451.zzz);
    let _e472 = ps_PixelShaderConstants.g_ShadowMapParams[1i];
    let _e476 = (_e292.zxy * _e451.www);
    let _e481 = vec4<f32>(_e476.x, _e476.y, _e476.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e394).w);
    let _e490 = vec4<f32>((_e476.z + _e289.y), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e496 = vec4<f32>(max((_e472 * clamp((_e455 - -(_e411.x)), 0f, 1f)), (_e472 * clamp((_e455 - -(_e411.y)), 0f, 1f))), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e504 = vec4<f32>(_e490.x, _e490.y, (_e462.y + _e317.y), _e490.w);
    let _e520 = vec4<f32>(vec4<f32>().x, (_e469.z + _e306.x), vec4<f32>().z, vec4<f32>().w);
    let _e528 = vec4<f32>((_e289.x + _e476.y), _e520.y, _e520.z, _e520.w);
    let _e536 = vec4<f32>((_e476.x + _e289.z), _e481.y, _e481.z, _e481.w);
    let _e552 = vec4<f32>(_e536.x, (_e469.y + _e306.z), _e536.z, _e536.w);
    let _e555 = max(exp2(_e451.y), 1f);
    let _e566 = (vec4<f32>((1f - _e425), _e348.y, _e348.z, _e348.w).xwzy * vec4<f32>(_e496.x, clamp(inverseSqrt(abs(dot(_e357, _e357))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e496.z, _e496.w).xyyy);
    let _e574 = vec4<f32>(_e552.x, _e552.y, (_e317.z + _e462.x), _e552.w);
    let _e575 = (_e419 * _e419).zxy;
    let _e584 = min(max(_e555, _e555), max(512f, 512f));
    let _e598 = vec4<f32>(_e416.x, _e416.y, _e416.z, (0.125f * _e584));
    let _e605 = vec4<f32>(_e379.x, _e379.y, _e379.z, (_e425 + _e566.x));
    let _e608 = _e605.www;
    let _e610 = ps_PixelShaderConstants.mrgGlobalLight_Diffuse;
    let _e612 = (_e608 * _e610.xyz);
    let _e616 = (_e612.xyz * _e400.www);
    let _e625 = (_e612.xzy * _e449.www);
    let _e636 = vec4<f32>(exp2((clamp(log2(clamp(dot(_e566.ywz, _e391), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * max(_e584, _e584))), _e574.y, _e574.z, _e574.w);
    let _e639 = ps_PixelShaderConstants.mrgGlobalLight_Specular;
    let _e644 = ps_PixelShaderConstants.g_Ambient[2i];
    let _e651 = ps_PixelShaderConstants.g_Ambient[0i];
    let _e654 = (((_e636.xxx * _e639.xyz).xyz * _e636.www).zyx * _e608);
    let _e662 = ps_PixelShaderConstants.g_Ambient[1i];
    phi_515_ = vec4<f32>(_e654.x, _e654.y, _e654.z, _e636.w);
    phi_518_ = vec4<f32>(_e625.x, _e625.y, _e625.z, _e566.w);
    phi_520_ = vec4<f32>(_e616.x, _e616.y, _e616.z, _e449.w);
    phi_522_ = vec4<f32>(_e93.x, _e93.y, _e93.z, select(0f, 1f, (-(abs(_e92.x)) > 0f)));
    phi_524_ = _e400;
    phi_526_ = _e605;
    phi_528_ = 0i;
    loop {
        let _e665 = phi_515_;
        let _e667 = phi_518_;
        let _e669 = phi_520_;
        let _e671 = phi_522_;
        let _e673 = phi_524_;
        let _e675 = phi_526_;
        let _e677 = phi_528_;
        local = _e673;
        local_1 = _e673;
        local_2 = _e675;
        local_3 = _e675;
        local_4 = _e671;
        local_5 = _e671;
        local_6 = _e675;
        local_7 = _e675;
        local_8 = _e673;
        local_9 = _e673;
        local_10 = _e673;
        local_11 = _e673;
        local_12 = _e669;
        local_13 = _e669;
        local_14 = _e667;
        local_15 = _e667;
        local_16 = _e665;
        local_17 = _e665;
        if (_e677 < 4i) {
            continue;
        } else {
            break;
        }
        continuing {
            let _e679 = _e671.www;
            let _e680 = (_e679 == vec3<f32>(0f, 0f, 0f));
            let _e682 = ps_PixelShaderConstants.mrgLocalLight0_Position;
            let _e685 = ps_PixelShaderConstants.mrgLocalLight1_Position;
            let _e691 = ps_PixelShaderConstants.mrgLocalLight0_Range;
            let _e694 = ps_PixelShaderConstants.mrgLocalLight1_Range;
            let _e697 = (_e679 + vec3<f32>(-2f, -3f, -4f));
            let _e699 = ps_PixelShaderConstants.mrgLocalLight0_Color;
            let _e702 = ps_PixelShaderConstants.mrgLocalLight1_Color;
            let _e706 = (_e697.xxx == vec3<f32>(0f, 0f, 0f));
            let _e708 = ps_PixelShaderConstants.mrgLocalLight2_Color;
            let _e715 = ps_PixelShaderConstants.mrgLocalLight2_Range;
            let _e720 = ps_PixelShaderConstants.mrgLocalLight2_Position;
            let _e725 = (_e697.yyy == vec3<f32>(0f, 0f, 0f));
            let _e727 = ps_PixelShaderConstants.mrgLocalLight3_Position;
            let _e734 = ps_PixelShaderConstants.mrgLocalLight3_Range;
            let _e739 = ps_PixelShaderConstants.mrgLocalLight3_Color;
            let _e744 = (_e697.zzz == vec3<f32>(0f, 0f, 0f));
            let _e746 = ps_PixelShaderConstants.mrgLocalLight4_Color;
            let _e753 = ps_PixelShaderConstants.mrgLocalLight4_Range;
            let _e756 = select(select(select(select(_e694.zw, _e691.zw, (_e671.ww == vec2<f32>(0f, 0f))).xy, _e715.zw, (_e697.xx == vec2<f32>(0f, 0f))).xy, _e734.zw, (_e697.yy == vec2<f32>(0f, 0f))).xy, _e753.zw, (_e697.zz == vec2<f32>(0f, 0f)));
            let _e758 = ps_PixelShaderConstants.mrgLocalLight4_Position;
            let _e765 = (-(select(select(select(select(_e685.xyz, _e682.xyz, _e680).xyz, _e720.xyz, _e706).xyz, _e727.xyz, _e725).xyz, _e758.xyz, _e744).xyz) + _e671.xyz);
            let _e766 = _e765.zxy;
            let _e770 = -(_e756.x);
            let _e772 = abs(dot(_e766, _e766));
            let _e774 = (_e770 + sqrt(_e772));
            let _e781 = vec4<f32>(_e675.x, _e675.y, _e675.z, clamp(inverseSqrt(_e772), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
            let _e784 = (_e765.xyz * _e781.www);
            let _e788 = -(_e784.zxy);
            let _e796 = vec4<f32>(_e781.x, _e781.y, _e781.z, clamp(dot(_e788, _e781.zxy), 0f, 1f));
            let _e798 = _e796.zxy;
            let _e805 = (-(_e784.zyx) + _e673.zyx);
            let _e806 = -(dot(_e788, _e798));
            let _e813 = vec4<f32>(_e669.x, _e669.y, _e669.z, clamp(max(_e806, _e806), 0f, 1f));
            let _e814 = _e805.xzy;
            let _e821 = vec4<f32>(_e673.x, _e673.y, _e673.z, (1f - clamp((clamp((1f / (_e756.y + _e770)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * max(_e774, _e774)), 0f, 1f)));
            let _e824 = (_e821.www * select(select(select(select(_e702.xyz, _e699.xyz, _e680).xyz, _e708.xyz, _e706).xyz, _e739.xyz, _e725).xyz, _e746.xyz, _e744).xyz);
            let _e832 = vec4<f32>(_e665.x, _e665.y, _e665.z, clamp(inverseSqrt(abs(dot(_e814, _e814))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
            let _e833 = _e824.xyz;
            let _e834 = _e796.www;
            let _e837 = ((_e833 * _e834) + _e813.xyz);
            let _e847 = ((_e824.xzy * _e813.www) + _e667.xyz);
            let _e874 = vec4<f32>(_e832.x, _e832.y, _e832.z, exp2((_e584 * clamp(log2(clamp(dot((_e805.zyx * _e832.www).zxy, _e798), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f))));
            let _e880 = (((_e833 * _e874.www).zyx * _e834) + _e874.xyz);
            phi_515_ = vec4<f32>(_e880.x, _e880.y, _e880.z, _e874.w);
            phi_518_ = vec4<f32>(_e847.x, _e847.y, _e847.z, _e667.w);
            phi_520_ = vec4<f32>(_e837.x, _e837.y, _e837.z, _e813.w);
            phi_522_ = vec4<f32>(_e671.x, _e671.y, _e671.z, (_e671.w + 1f));
            phi_524_ = _e821;
            phi_526_ = _e796;
            phi_528_ = (_e677 + 1i);
        }
    }
    let _e888 = textureSample(ps_sampDif_t2d, ps_sampDif_s, _e598.xy);
    let _e891 = ps_PixelShaderConstants.g_Specular;
    let _e895 = ps_PixelShaderConstants.mrgEyeLight_Specular;
    let _e900 = ps_PixelShaderConstants.mrgEyeLight_Diffuse;
    let _e906 = ps_PixelShaderConstants.mrgEyeLight_Range[3i];
    let _e909 = ps_PixelShaderConstants.mrgEyeLight_Range[2i];
    let _e913 = ps_PixelShaderConstants.mrgEyeLight_Attribute[0i];
    let _e918 = ps_PixelShaderConstants.mrgEyeLight_Attribute[1i];
    let _e921 = ps_PixelShaderConstants.mrgEyeLight_Attribute[2i];
    let _e925 = ps_PixelShaderConstants.g_NPCClothParam;
    let _e930 = ps_PixelShaderConstants.g_NPCClothParam[0i];
    let _e933 = ps_PixelShaderConstants.g_NPCClothParam[1i];
    let _e934 = (_e930 + _e933);
    let _e936 = local;
    let _e941 = local_2;
    let _e944 = _e941.zxy;
    let _e945 = dot(_e936.zxy, _e944);
    let _e949 = local_4;
    let _e954 = (_e949.xyz + -(_e111));
    let _e958 = ps_PixelShaderConstants.g_Diffuse[3i];
    let _e960 = min((_e958 * max(_e888.w, _e888.w)), 4f);
    let _e966 = _e954.zxy;
    let _e984 = local_6;
    let _e990 = local_8;
    let _e997 = ps_PixelShaderConstants.g_Diffuse;
    let _e1000 = abs(dot(_e966, _e966));
    let _e1002 = ((vec4<f32>((_e945 + _e945), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx * _e984.xyz) + -(_e990.xyz)).xyz;
    let _e1003 = (_e1002 * _e1002);
    let _e1008 = vec4<f32>(vec4<f32>((_e97.w * max(_e934, _e934)), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).x, _e1003.x, _e1003.y, _e1003.z);
    let _e1017 = ((_e1002 * vec3<f32>(0.5f, 0.5f, 0.5f)) + vec3<f32>(0.5f, 0.5f, 0.5f));
    let _e1022 = vec4<f32>(_e1017.x, _e1017.y, _e1017.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1025 = (_e954.xyz * vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, clamp(inverseSqrt(_e1000), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)).www);
    let _e1033 = vec4<f32>(vec4<f32>().x, (-(_e909) - -(sqrt(_e1000))), vec4<f32>().z, vec4<f32>().w);
    let _e1037 = local_10;
    let _e1041 = (-(_e1025.xzy) + _e1037.xzy);
    let _e1048 = vec4<f32>(clamp((1f / (_e918 - _e921)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1051 = (_e320.zxy * _e1017.zzz);
    let _e1061 = (_e309.zxy * _e1017.yyy);
    let _e1068 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, (_e317.z + _e1051.x), vec4<f32>().w);
    let _e1069 = _e1025.zxy;
    let _e1071 = ps_PixelShaderConstants.g_EyeDirection;
    let _e1080 = vec4<f32>(_e1068.x, (_e306.z + _e1061.x), _e1068.z, _e1068.w);
    let _e1081 = _e1041.yxz;
    let _e1094 = clamp((vec4<f32>((-(_e921) - -(dot(_e1069, _e1071.zxy))), _e1033.y, _e1033.z, _e1033.w).yx * vec4<f32>(_e1048.x, clamp((1f / (_e906 - _e909)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e1048.z, _e1048.w).yx), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e1112 = vec4<f32>(vec4<f32>().x, (_e306.y + _e1061.z), vec4<f32>().z, vec4<f32>().w);
    let _e1122 = vec4<f32>(_e1112.x, _e1112.y, (_e317.y + _e1051.z), _e1112.w);
    let _e1124 = (vec2<f32>(1f, 3f) - _e1094.xy);
    let _e1129 = select(1f, _e1124.x, (_e913 > 0f));
    let _e1134 = vec4<f32>(_e1129, vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1148 = (vec4<f32>(_e584, _e297.y, _e297.z, _e297.w).yzwx * vec4<f32>(_e1022.x, _e1022.y, _e1022.z, clamp(log2(clamp(dot((_e1041.xzy * vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, clamp(inverseSqrt(abs(dot(_e1081, _e1081))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)).www).zxy, _e944), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)).xxxw);
    let _e1152 = dot(-(_e1069), _e944);
    let _e1153 = clamp(_e1152, 0f, 1f);
    let _e1158 = vec4<f32>(_e1134.x, _e1134.y, _e1153, _e1134.w);
    let _e1163 = vec4<f32>((_e1148.x + _e289.x), (_e306.x + _e1061.y), (_e317.x + _e1051.y), (_e1124.y + -(_e1094.y)));
    let _e1166 = _e1003.zxy;
    let _e1173 = vec4<f32>(dot(_e1166, _e1163.zxy), _e1163.y, _e1163.z, _e1163.w);
    let _e1186 = vec4<f32>(_e1173.x, dot(_e1166, vec4<f32>((_e289.y + max(_e1148.y, _e1148.y)), _e1122.y, _e1122.z, _e1122.w).zxy), _e1173.z, _e1173.w);
    let _e1208 = (vec4<f32>(_e1008.x, (_e1094.y * _e1094.y), _e1008.z, _e1008.w).yxxx * vec4<f32>(_e1186.x, _e1186.y, dot(_e1166, vec4<f32>((_e289.z + max(_e1148.z, _e1148.z)), _e1080.y, _e1080.z, _e1080.w).zxy), _e1186.w).wxyz);
    let _e1215 = (_e241.xyz * _e147.www);
    let _e1220 = vec4<f32>(_e1215.x, _e1215.y, _e1215.z, vec4<f32>().w);
    let _e1223 = ps_PixelShaderConstants.mrgEyeLight_Attribute[3i];
    let _e1224 = (_e1223 * clamp(log2(abs(_e1208.x)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e1226 = -(_e1152);
    let _e1233 = vec4<f32>(_e1220.x, _e1220.y, _e1220.z, clamp(max(_e1226, _e1226), 0f, 1f));
    let _e1237 = select(1f, clamp(exp2(_e1224), 0f, 1f), (select(0f, 1f, (_e913 > 1f)) > 0f));
    let _e1242 = vec4<f32>(_e1237, vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1248 = vec4<f32>(_e1242.x, (_e1129 * _e1237), _e1242.z, _e1242.w);
    let _e1251 = ((_e900.xyz * _e900.www).xyz * _e1248.yyy);
    let _e1256 = local_12;
    let _e1265 = local_14;
    let _e1279 = vec4<f32>(_e1158.x, exp2(_e1148.w), _e1158.z, _e1158.w);
    let _e1292 = ((((_e1251.xyz * _e1158.zzz) + _e1256.xyz).xyz * _e997.xyz) + vec4<f32>((_e644 * dot(_e575, _e574.zxy)), (_e651 * dot(_e575, vec4<f32>(_e528.x, _e528.y, (_e317.x + _e462.z), _e528.w).zxy)), (_e662 * dot(_e575, vec4<f32>(_e504.x, (_e306.y + _e469.x), _e504.z, _e504.w).zxy)), _e1224).yzx);
    let _e1300 = ((((_e895.xyz * _e895.www).xyz * _e1248.xyy).xzy * _e1279.xyy).xyz * _e1279.yzz);
    let _e1305 = vec4<f32>(_e1292.x, _e1300.x, _e1300.y, _e1300.z);
    let _e1314 = local_16;
    let _e1334 = ((min(((((_e598.www * _e891.xyz).xzy * (_e1314.zxy + vec4<f32>((_e1300.x * _e1153), _e1305.y, _e1305.z, _e1305.w).xzw).xyz).xyz * _e1233.xzy) + ((_e1292.xzy * _e888.xzy) + ((((_e888.xyz * _e925.zzz).xzy * _e997.xzy).xzy * ((_e1251.xzy * _e1233.www) + _e1265.xyz).xzy) + _e1208.yzw).xzy).xyz).xzy, vec3<f32>(4f, 4f, 4f)).xyz * _e96.xyz) + _e97.xyz);
    let _e1341 = ps_SharedConstants.g_SpecConstantsValue;
    if ((_e1341 & 2u) != 0u) {
        let _e1345 = ps_SharedConstants.g_AlphaThreshold;
        if ((_e960 - _e1345) < 0f) {
            discard;
        }
    }
    ps_out_u002e_var_u002e_SV_Target0_ = vec4<f32>(_e1334.x, _e1334.y, _e1334.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e960).w);
    return;
}

@fragment 
fn fragment(@builtin(position) param: vec4<f32>, @location(0) in_u002e_var_u002e_TEXCOORD0_: vec4<f32>, @location(1) in_u002e_var_u002e_TEXCOORD1_: vec4<f32>, @location(2) in_u002e_var_u002e_TEXCOORD2_: vec4<f32>, @location(3) in_u002e_var_u002e_TEXCOORD3_: vec4<f32>, @location(4) in_u002e_var_u002e_TEXCOORD4_: vec4<f32>, @location(5) in_u002e_var_u002e_TEXCOORD5_: vec4<f32>, @location(6) in_u002e_var_u002e_TEXCOORD6_: vec4<f32>, @location(7) in_u002e_var_u002e_TEXCOORD7_: vec4<f32>, @builtin(front_facing) param_1: bool) -> @location(0) vec4<f32> {
    chr_shadow = chr_fetch_shadow(param);
    ps_global = param;
    ps_in_u002e_var_u002e_TEXCOORD0_1 = in_u002e_var_u002e_TEXCOORD0_;
    ps_in_u002e_var_u002e_TEXCOORD1_1 = in_u002e_var_u002e_TEXCOORD1_;
    ps_in_u002e_var_u002e_TEXCOORD2_1 = in_u002e_var_u002e_TEXCOORD2_;
    ps_in_u002e_var_u002e_TEXCOORD3_1 = in_u002e_var_u002e_TEXCOORD3_;
    ps_in_u002e_var_u002e_TEXCOORD4_1 = in_u002e_var_u002e_TEXCOORD4_;
    ps_in_u002e_var_u002e_TEXCOORD5_1 = in_u002e_var_u002e_TEXCOORD5_;
    ps_in_u002e_var_u002e_TEXCOORD6_1 = in_u002e_var_u002e_TEXCOORD6_;
    ps_in_u002e_var_u002e_TEXCOORD7_1 = in_u002e_var_u002e_TEXCOORD7_;
    ps_global_1 = param_1;
    ps_main_1();
    let _e21 = ps_out_u002e_var_u002e_SV_Target0_;
    return _e21;
}

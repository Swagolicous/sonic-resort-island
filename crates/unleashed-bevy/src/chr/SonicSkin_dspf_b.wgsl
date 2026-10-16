// Composed by re/xenos/compose.py from CharactorDefault_@b@.wgsl (vertex) and SonicSkin_dspf@@.wgsl (pixel): the game's own programs, resources in the material group; shadow-map taps answered by the host (host_shadow).

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
    mrgFresnelParam: vec4<f32>,
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
    var phi_126_: vec4<f32>;
    var phi_134_: vec4<f32>;
    var phi_141_: vec4<f32>;
    var phi_148_: vec4<f32>;
    var phi_184_: vec4<f32>;
    var phi_214_: vec4<f32>;
    var phi_217_: vec4<f32>;
    var phi_219_: vec4<f32>;
    var phi_221_: vec4<f32>;
    var phi_223_: vec4<f32>;
    var phi_225_: i32;
    var local: vec4<f32>;
    var local_1: vec4<f32>;
    var local_2: vec4<f32>;
    var local_3: vec4<f32>;
    var local_4: vec4<f32>;
    var local_5: vec4<f32>;
    var local_6: vec4<f32>;
    var local_7: vec4<f32>;
    var phi_365_: vec4<f32>;
    var phi_366_: vec4<f32>;
    var phi_367_: vec4<f32>;

    let _e81 = vs_in_u002e_var_u002e_POSITION0_1;
    let _e82 = vs_in_u002e_var_u002e_TEXCOORD0_1;
    let _e83 = vs_in_u002e_var_u002e_TEXCOORD1_1;
    let _e84 = vs_in_u002e_var_u002e_TEXCOORD2_1;
    let _e85 = vs_in_u002e_var_u002e_TEXCOORD3_1;
    let _e86 = vs_in_u002e_var_u002e_NORMAL0_1;
    let _e87 = vs_in_u002e_var_u002e_BLENDWEIGHT0_1;
    let _e88 = vs_in_u002e_var_u002e_BLENDINDICES0_1;
    let _e90 = vs_SharedConstants.g_SwappedTexcoords;
    if ((_e90 & 1u) != 0u) {
        phi_126_ = _e82.yxwz;
    } else {
        phi_126_ = _e82;
    }
    let _e95 = phi_126_;
    let _e100 = vec4<f32>(vec4<f32>(0f, 0f, 0f, 0f).x, _e95.x, _e95.y, vec4<f32>(0f, 0f, 0f, 0f).w);
    if ((_e90 & 2u) != 0u) {
        phi_134_ = _e83.yxwz;
    } else {
        phi_134_ = _e83;
    }
    let _e105 = phi_134_;
    if ((_e90 & 4u) != 0u) {
        phi_141_ = _e84.yxwz;
    } else {
        phi_141_ = _e84;
    }
    let _e110 = phi_141_;
    if ((_e90 & 8u) != 0u) {
        phi_148_ = _e85.yxwz;
    } else {
        phi_148_ = _e85;
    }
    let _e115 = phi_148_;
    let _e117 = vs_SharedConstants.g_SpecConstantsValue;
    switch bitcast<i32>(0u) {
        default: {
            if ((_e117 & 1u) != 0u) {
                phi_184_ = vec4<f32>((select(0f, -1f, ((_e86.x & 1024u) != 0u)) + (f32((_e86.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e86.x & 2097152u) != 0u)) + (f32(((_e86.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e86.x & 2147483648u) != 0u)) + (f32(((_e86.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_184_ = bitcast<vec4<f32>>(_e86);
                break;
            }
        }
    }
    let _e150 = phi_184_;
    let _e155 = vec4<f32>(_e150.x, _e150.y, _e150.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e157 = vec4<f32>(_e88.wxyz);
    let _e159 = vs_SharedConstants.g_Booleans;
    phi_365_ = _e100;
    phi_366_ = _e155;
    phi_367_ = _e81;
    if ((_e159 & 1u) != 0u) {
        let _e165 = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(vec4<f32>(0f, 0f, 0f, 0f))) > vec4<f32>(0f, 0f, 0f, 0f)));
        let _e169 = select(0f, 1f, (-(abs(0f)) > 0f));
        let _e174 = vec4<f32>(_e169, _e100.y, _e100.z, _e100.w);
        phi_214_ = _e155;
        phi_217_ = _e165;
        phi_219_ = _e165;
        phi_221_ = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(_e174.xxxx)) > vec4<f32>(0f, 0f, 0f, 0f)));
        phi_223_ = vec4<f32>(_e174.x, _e174.y, _e174.z, select(0f, 1f, (-(abs(_e169)) > 0f)));
        phi_225_ = 0i;
        loop {
            let _e190 = phi_214_;
            let _e192 = phi_217_;
            let _e194 = phi_219_;
            let _e196 = phi_221_;
            let _e198 = phi_223_;
            let _e200 = phi_225_;
            local = _e198;
            local_1 = _e198;
            local_2 = _e196;
            local_3 = _e194;
            local_4 = _e192;
            local_5 = _e190;
            local_6 = _e190;
            local_7 = _e190;
            if (_e200 < 3i) {
                continue;
            } else {
                break;
            }
            continuing {
                let _e203 = (_e198.x == 0f);
                let _e208 = (_e198.xx + vec2<f32>(-2f, -3f));
                let _e213 = (_e208.x == 0f);
                let _e219 = (_e208.y == 0f);
                let _e221 = select(select(select(_e87.y, _e87.x, _e203), _e87.z, _e213), _e87.w, _e219);
                let _e226 = vec4<f32>(_e190.x, _e190.y, _e190.z, _e221);
                let _e235 = vec4<f32>((1f + _e198.x), _e198.y, _e198.z, _e198.w);
                let _e246 = i32(clamp(floor(((select(select(select(_e157.z, _e157.y, _e203), _e157.w, _e213), _e157.x, _e219) * 3f) + 0.5f)), -256f, 255f));
                let _e247 = _e226.wwww;
                let _e253 = vs_VertexShaderConstants.g_MtxPalette[min(_e246, 227i)];
                let _e258 = (1i + _e246);
                let _e264 = vs_VertexShaderConstants.g_MtxPalette[min(_e258, 227i)];
                let _e269 = (2i + _e246);
                let _e275 = vs_VertexShaderConstants.g_MtxPalette[min(_e269, 227i)];
                phi_214_ = _e226;
                phi_217_ = ((_e247 * select(vec4<f32>(0f, 0f, 0f, 0f), _e253, vec4((_e246 < 228i))).wzyx) + _e192);
                phi_219_ = ((_e247 * select(vec4<f32>(0f, 0f, 0f, 0f), _e264, vec4((_e258 < 228i))).wzyx) + _e194);
                phi_221_ = ((_e247 * select(vec4<f32>(0f, 0f, 0f, 0f), _e275, vec4((_e269 < 228i))).wzyx) + _e196);
                phi_223_ = vec4<f32>(_e235.x, _e235.y, _e235.z, (_e221 + _e198.w));
                phi_225_ = (_e200 + 1i);
            }
        }
        let _e284 = (3f * _e157.x);
        let _e286 = local;
        let _e290 = local_1;
        let _e295 = vec4<f32>((1f - _e286.w), _e290.y, _e290.z, _e290.w);
        let _e299 = i32(clamp(floor((_e284 + 0.5f)), -256f, 255f));
        let _e300 = _e295.xxxx;
        let _e301 = (2i + _e299);
        let _e307 = vs_VertexShaderConstants.g_MtxPalette[min(_e301, 227i)];
        let _e312 = local_2;
        let _e313 = ((_e300 * select(vec4<f32>(0f, 0f, 0f, 0f), _e307, vec4((_e301 < 228i))).wzyx) + _e312);
        let _e314 = (1i + _e299);
        let _e320 = vs_VertexShaderConstants.g_MtxPalette[min(_e314, 227i)];
        let _e325 = local_3;
        let _e326 = ((_e300 * select(vec4<f32>(0f, 0f, 0f, 0f), _e320, vec4((_e314 < 228i))).wzyx) + _e325);
        let _e332 = vs_VertexShaderConstants.g_MtxPalette[min(_e299, 227i)];
        let _e337 = local_4;
        let _e338 = ((_e300 * select(vec4<f32>(0f, 0f, 0f, 0f), _e332, vec4((_e299 < 228i))).wzyx) + _e337);
        let _e343 = vec4<f32>(_e284, max(_e81.y, _e81.y), max(_e81.z, _e81.z), max(_e81.w, _e81.w));
        let _e352 = vec4<f32>(max(_e81.x, _e81.x), _e343.y, _e343.z, _e343.w).wzxy;
        let _e358 = vec4<f32>(dot(_e338.xywz, _e352), _e81.y, _e81.z, _e81.w);
        let _e365 = vec4<f32>(_e358.x, dot(_e326.xywz, _e352), _e358.z, _e358.w);
        let _e374 = local_5;
        let _e381 = vec4<f32>(max(_e374.x, _e374.x), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e383 = local_6;
        let _e390 = vec4<f32>(_e381.x, _e381.y, max(_e383.z, _e383.z), _e381.w);
        let _e392 = local_7;
        let _e401 = vec4<f32>(_e390.x, max(_e392.y, _e392.y), _e390.z, _e390.w).zxy;
        let _e407 = vec4<f32>(dot(_e338.ywz, _e401), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e414 = vec4<f32>(_e407.x, dot(_e326.ywz, _e401), _e407.z, _e407.w);
        phi_365_ = _e295;
        phi_366_ = vec4<f32>(_e414.x, _e414.y, dot(_e313.ywz, _e401), _e414.w);
        phi_367_ = vec4<f32>(_e365.x, _e365.y, dot(_e313.xywz, _e352), _e365.w);
    }
    let _e423 = phi_365_;
    let _e425 = phi_366_;
    let _e427 = phi_367_;
    let _e431 = vs_VertexShaderConstants.g_MtxWorld[3i];
    let _e437 = vs_VertexShaderConstants.g_MtxWorld[2i];
    let _e445 = vs_VertexShaderConstants.g_MtxWorld[1i];
    let _e452 = vs_VertexShaderConstants.g_MtxWorld[0i];
    let _e455 = ((_e427.xxxx * _e452.xzyw) + ((_e427.yyyy * _e445.xzyw) + ((_e427.zzzz * _e437.xzyw) + (_e427.wwww * _e431.xyzw).xzyw)));
    let _e459 = vs_VertexShaderConstants.g_MtxView[3i];
    let _e462 = _e455.yyyy;
    let _e465 = vs_VertexShaderConstants.g_MtxView[2i];
    let _e470 = _e455.zzzz;
    let _e473 = vs_VertexShaderConstants.g_MtxView[1i];
    let _e478 = _e455.xxxx;
    let _e481 = vs_VertexShaderConstants.g_MtxView[0i];
    let _e485 = ((_e478 * _e481.xyzw) + ((_e470 * _e473.xzyw) + ((_e462 * _e465.xwyz) + (_e455.wwww * _e459.xwzy).xywz).xwzy).xzyw);
    let _e489 = vs_VertexShaderConstants.g_MtxProjection[3i];
    let _e495 = vs_VertexShaderConstants.g_MtxProjection[2i];
    let _e502 = vs_VertexShaderConstants.g_MtxProjection[1i];
    let _e506 = ((_e485.yyyy * _e502.xzyw) + ((_e485.zzzz * _e495.xwzy) + (_e485.wwww * _e489.xwzy)).xzwy);
    let _e510 = vs_VertexShaderConstants.g_MtxProjection[0i];
    let _e514 = ((_e485.xxxx * _e510.xyzw) + _e506.xzyw);
    let _e517 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[3i];
    let _e520 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[0i];
    let _e527 = vs_VertexShaderConstants.mrgTexcoordIndex[0i];
    let _e533 = vec4<f32>(trunc(_e527), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e543 = ((_e425.yyy * _e445.xyz) + (_e425.zzz * _e437.xyz).xyz);
    let _e548 = ((_e425.xxx * _e452.xzy) + _e543.xzy);
    let _e553 = vec4<f32>(_e543.x, _e548.x, _e548.y, _e548.z);
    let _e554 = _e548.yxz;
    let _e556 = _e455.xzy;
    let _e559 = vs_VertexShaderConstants.g_EyePosition;
    let _e561 = (-(_e556) + _e559.xyz);
    let _e564 = vs_VertexShaderConstants.g_MtxLightViewProjection[2i];
    let _e574 = vec4<f32>(clamp(inverseSqrt(abs(dot(_e554, _e554))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e553.y, _e553.z, _e553.w);
    let _e577 = (_e574.ywz * _e574.xxx);
    let _e582 = vec4<f32>(_e577.x, _e577.y, _e577.z, _e506.w);
    let _e583 = _e561.zxy;
    let _e587 = vs_VertexShaderConstants.g_MtxLightViewProjection[1i];
    let _e593 = vs_VertexShaderConstants.g_MtxLightViewProjection[0i];
    let _e596 = ((_e478 * _e593.xzwy) + ((_e470 * _e587.xzwy) + (_e462 * _e564.xzwy)));
    let _e601 = vs_VertexShaderConstants.g_MtxLightViewProjection[3i];
    let _e613 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[0i];
    let _e616 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[1i];
    let _e620 = (_e596.xzy + _e601.xwz);
    let _e639 = (_e561.xyz * vec4<f32>(clamp(inverseSqrt(abs(dot(_e583, _e583))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx).zxy;
    let _e641 = vs_VertexShaderConstants.mrgGlobalLight_Direction;
    let _e644 = dot(_e639, -(_e641.zxy));
    let _e649 = (vec4<f32>(_e620.x, vec4<f32>(vec4<f32>().x, (-(_e596.w) + -(_e601.y)), vec4<f32>().z, vec4<f32>().w).y, _e620.y, _e620.z).zxy * vec3<f32>(0.5f, 0.5f, 0.5f));
    let _e654 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[2i];
    let _e658 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[1i];
    let _e664 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[2i];
    let _e668 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[0i];
    let _e671 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[1i];
    let _e672 = (_e668 + _e671);
    let _e679 = vec4<f32>(((clamp(((-(_e616) + -(max(_e485.z, _e485.z))) * clamp((1f / (_e613 - _e616)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)), 0f, 1f) * _e664) * -(_e672)), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e690 = (_e649.yz + _e649.xx);
    let _e691 = max(_e556, _e556);
    let _e696 = vec4<f32>(_e691.x, _e691.y, _e691.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e697 = -(_e485.z);
    let _e706 = (_e596.yz + _e601.zw);
    let _e713 = (vec4<f32>(_e679.x, clamp(log2(abs(((_e644 * _e654) + _e658))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e679.z, _e679.w).yx * vec2<f32>(1.5f, 1.442695f));
    let _e715 = exp2(_e713.y);
    let _e716 = vec3(_e715);
    let _e721 = vec4<f32>(_e716.x, _e716.y, _e716.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e732 = (1f - clamp(dot(_e577.zxy, _e639), 0f, 1f));
    let _e736 = vec4<f32>((_e517 * _e520), _e644, clamp((1f / exp2(_e713.x)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e732);
    let _e739 = (_e736.wxy * _e736.wzy);
    let _e749 = vs_VertexShaderConstants.mrgFresnelParam[0i];
    let _e750 = -(_e749);
    let _e752 = ((_e739.x * _e739.x) * _e732);
    let _e753 = (1f + max(_e750, _e750));
    let _e755 = (vec4<f32>(_e533.x, _e739.x, _e739.y, _e739.z).xwx + vec3<f32>(-2f, 1f, -3f));
    let _e760 = vec4<f32>(_e755.x, _e736.y, _e755.y, _e755.z);
    let _e762 = clamp((1f / _e672), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e767 = vec4<f32>(_e760.x, _e762, _e760.z, _e760.w);
    let _e771 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[2i];
    let _e774 = ((_e755.y * _e771) + _e739.y);
    let _e782 = select(select(_e105.xy, _e423.yz, (_e533.xx == vec2<f32>(0f, 0f))).xy, _e110.xy, (_e767.xx == vec2<f32>(0f, 0f)));
    let _e787 = vec4<f32>(_e782.x, _e767.y, _e782.y, _e767.w);
    let _e792 = select(_e782.xy, _e115.xy, (_e767.ww == vec2<f32>(0f, 0f)));
    let _e797 = vec4<f32>(_e792.x, _e792.y, vec4<f32>(0f, 0f, 0f, 0f).z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e802 = vec4<f32>(_e797.x, _e797.y, 0f, _e797.w);
    let _e813 = vec4<f32>((_e774 * _e762), _e787.y, _e787.z, _e787.w);
    let _e816 = vs_VertexShaderConstants.g_LightScatteringColor;
    let _e818 = (_e813.xxy * _e816.xyz);
    let _e823 = vec4<f32>(_e818.x, _e818.y, _e818.z, _e813.w);
    let _e826 = vs_VertexShaderConstants.mrgFresnelParam[1i];
    let _e832 = vec4<f32>(_e582.x, _e582.y, _e582.z, (_e826 * ((_e752 * _e753) + _e749)));
    let _e845 = vs_VertexShaderConstants.g_LightScatteringFarNearScale;
    let _e847 = ((vec4<f32>(_e823.x, _e823.y, _e823.z, (_e818.z * _e774)).xyw * vec4<f32>(_e753, (1f - _e715), _e774, _e752).yyy).xyz * _e845.www);
    let _e852 = vec4<f32>(_e847.x, _e847.y, _e847.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e859 = vs_SharedConstants.g_HalfPixelOffset;
    let _e863 = (_e514.xy + (_e859 * _e514.w));
    vs_global = vec4<f32>(_e863.x, _e863.y, _e514.z, _e514.w);
    vs_out_u002e_var_u002e_TEXCOORD0_ = vec4<f32>(_e802.x, _e802.y, _e802.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD1_ = vec4<f32>(_e696.x, _e696.y, _e696.z, max(_e697, _e697));
    vs_out_u002e_var_u002e_TEXCOORD2_ = max(_e832, _e832);
    vs_out_u002e_var_u002e_TEXCOORD3_ = vec4<f32>(_e690.x, _e690.y, _e706.x, _e706.y);
    vs_out_u002e_var_u002e_TEXCOORD4_ = vec4<f32>(_e721.x, _e721.y, _e721.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD5_ = vec4<f32>(_e852.x, _e852.y, _e852.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD6_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD7_ = vec4<f32>(0f, 0f, 0f, 0f);
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
fn vertex(@location(0) in_u002e_var_u002e_POSITION0_: vec4<f32>, @location(4) in_u002e_var_u002e_TEXCOORD0_: vec4<f32>, @location(5) in_u002e_var_u002e_TEXCOORD1_: vec4<f32>, @location(6) in_u002e_var_u002e_TEXCOORD2_: vec4<f32>, @location(7) in_u002e_var_u002e_TEXCOORD3_: vec4<f32>, @location(1) in_u002e_var_u002e_NORMAL0_: vec4<u32>, @location(10) in_u002e_var_u002e_BLENDWEIGHT0_: vec4<f32>, @location(9) in_u002e_var_u002e_BLENDINDICES0_: vec4<u32>) -> vs_VertexOutput {
    vs_in_u002e_var_u002e_POSITION0_1 = in_u002e_var_u002e_POSITION0_;
    vs_in_u002e_var_u002e_TEXCOORD0_1 = in_u002e_var_u002e_TEXCOORD0_;
    vs_in_u002e_var_u002e_TEXCOORD1_1 = in_u002e_var_u002e_TEXCOORD1_;
    vs_in_u002e_var_u002e_TEXCOORD2_1 = in_u002e_var_u002e_TEXCOORD2_;
    vs_in_u002e_var_u002e_TEXCOORD3_1 = in_u002e_var_u002e_TEXCOORD3_;
    vs_in_u002e_var_u002e_NORMAL0_1 = in_u002e_var_u002e_NORMAL0_;
    vs_in_u002e_var_u002e_BLENDWEIGHT0_1 = in_u002e_var_u002e_BLENDWEIGHT0_;
    vs_in_u002e_var_u002e_BLENDINDICES0_1 = in_u002e_var_u002e_BLENDINDICES0_;
    vs_main_1();
    vs_global.z = 0.1; // Bevy infinite reverse-Z: depth = near / w
    let _e38 = vs_global;
    let _e39 = vs_out_u002e_var_u002e_TEXCOORD0_;
    let _e40 = vs_out_u002e_var_u002e_TEXCOORD1_;
    let _e41 = vs_out_u002e_var_u002e_TEXCOORD2_;
    let _e42 = vs_out_u002e_var_u002e_TEXCOORD3_;
    let _e43 = vs_out_u002e_var_u002e_TEXCOORD4_;
    let _e44 = vs_out_u002e_var_u002e_TEXCOORD5_;
    let _e45 = vs_out_u002e_var_u002e_TEXCOORD6_;
    let _e46 = vs_out_u002e_var_u002e_TEXCOORD7_;
    let _e47 = vs_out_u002e_var_u002e_TEXCOORD8_;
    let _e48 = vs_out_u002e_var_u002e_TEXCOORD9_;
    let _e49 = vs_out_u002e_var_u002e_TEXCOORD10_;
    let _e50 = vs_out_u002e_var_u002e_TEXCOORD11_;
    let _e51 = vs_out_u002e_var_u002e_TEXCOORD12_;
    let _e52 = vs_out_u002e_var_u002e_TEXCOORD13_;
    let _e53 = vs_out_u002e_var_u002e_TEXCOORD14_;
    let _e54 = vs_out_u002e_var_u002e_TEXCOORD15_;
    let _e55 = vs_out_u002e_var_u002e_COLOR0_;
    let _e56 = vs_out_u002e_var_u002e_COLOR1_;
    return vs_VertexOutput(_e38, _e39, _e40, _e41, _e42, _e43, _e44);
}


struct ps_type_u002e_PixelShaderConstants {
    g_Diffuse: vec4<f32>,
    g_Ambient: vec4<f32>,
    g_Specular: vec4<f32>,
    g_PowerGlossLevel: vec4<f32>,
    g_OpacityReflectionRefractionSpectype: vec4<f32>,
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
    mrgLuminanceRange: vec4<f32>,
    g_ShadowMapParams: vec4<f32>,
    g_aLightField: array<vec4<f32>, 6>,
    g_SonicSkinFalloffParam: vec4<f32>,
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
@group(#{MATERIAL_BIND_GROUP}) @binding(28) 
var ps_sampFalloff_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(31) 
var ps_sampFalloff_s: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(40) 
var ps_sampSpec_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(43) 
var ps_sampSpec_s: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) 
var<uniform> ps_PixelShaderConstants: ps_type_u002e_PixelShaderConstants;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) 
var<uniform> ps_SharedConstants: ps_type_u002e_SharedConstants;
var<private> ps_in_u002e_var_u002e_TEXCOORD0_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD1_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD2_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD3_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD4_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD5_1: vec4<f32>;
var<private> ps_out_u002e_var_u002e_SV_Target0_: vec4<f32>;

fn ps_main_1() {
    var phi_471_: vec4<f32>;
    var phi_474_: vec4<f32>;
    var phi_476_: i32;
    var local: vec4<f32>;
    var local_1: vec4<f32>;

    let _e98 = ps_in_u002e_var_u002e_TEXCOORD0_1;
    let _e99 = ps_in_u002e_var_u002e_TEXCOORD1_1;
    let _e100 = ps_in_u002e_var_u002e_TEXCOORD2_1;
    let _e101 = ps_in_u002e_var_u002e_TEXCOORD3_1;
    let _e102 = ps_in_u002e_var_u002e_TEXCOORD4_1;
    let _e103 = ps_in_u002e_var_u002e_TEXCOORD5_1;
    let _e104 = _e100.zxy;
    let _e108 = clamp((1f / _e101.w), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e113 = vec4<f32>(vec4<f32>().x, _e108, vec4<f32>().z, vec4<f32>().w);
    let _e119 = vec4<f32>((_e108 + _e108), _e113.y, _e113.z, _e113.w);
    let _e122 = clamp(inverseSqrt(abs(dot(_e104, _e104))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e127 = vec4<f32>(_e119.x, _e119.y, _e119.z, _e122);
    let _e130 = (_e127.xyyx * _e101.xxyy);
    let _e131 = _e98.xy;
    let _e132 = textureSample(ps_sampDif_t2d, ps_sampDif_s, _e131);
    let _e133 = _e130.yz;
    let _e134 = textureDimensions(ps_g_ShadowMapSampler_t2d, 0i);
    let _e135 = vec2<f32>(_e134);
    let _e138 = chr_shadow_tap(0i);
    let _e142 = chr_shadow_tap(1i);
    let _e146 = chr_shadow_tap(2i);
    let _e150 = chr_shadow_tap(3i);
    let _e153 = textureSample(ps_sampSpec_t2d, ps_sampSpec_s, _e131);
    let _e160 = select(fract(((_e133 * _e135) - vec2<f32>(0.5f, 0.5f))), vec2<f32>(0f, 0f), ((bitcast<vec2<u32>>(_e133) & vec2<u32>(2147483647u, 2147483647u)) > vec2<u32>(2139095040u, 2139095040u)));
    let _e165 = vec4<f32>(vec4<f32>(0f, 0f, 0f, 0f).x, _e160.x, _e160.y, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e168 = ps_PixelShaderConstants.g_Specular[2i];
    let _e172 = ps_PixelShaderConstants.g_PowerGlossLevel[1i];
    let _e181 = ps_PixelShaderConstants.g_aLightField[5i];
    let _e185 = ps_PixelShaderConstants.g_aLightField[4i];
    let _e188 = (_e181.xyz + -(_e185.xyz));
    let _e191 = ps_PixelShaderConstants.g_aLightField[1i];
    let _e195 = ps_PixelShaderConstants.g_aLightField[0i];
    let _e201 = ps_PixelShaderConstants.g_aLightField[3i];
    let _e205 = ps_PixelShaderConstants.g_aLightField[2i];
    let _e211 = _e99.xyz;
    let _e214 = ps_PixelShaderConstants.g_EyePosition;
    let _e215 = _e214.xyz;
    let _e216 = (-(_e211) + _e215);
    let _e218 = _e216.zxy;
    let _e220 = max(max(_e101.z, _e101.z), max(0f, 0f));
    let _e230 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, ((-(_e220) * select(0f, 1f, (_e220 >= _e101.w))) + _e220));
    let _e233 = ps_PixelShaderConstants.mrgGlobalLight_Direction;
    let _e235 = -(_e233.xyz);
    let _e236 = (_e127.www * _e235);
    let _e241 = vec4<f32>(_e236.x, _e236.y, _e236.z, vec4<f32>().w);
    let _e247 = vec4<f32>(_e98.x, _e98.y, _e98.z, (0.5f * _e122));
    let _e250 = (_e247.www * _e100.zyx);
    let _e263 = vec4<f32>(_e247.x, _e247.y, clamp(inverseSqrt(abs(dot(_e218, _e218))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e247.w);
    let _e266 = (_e216.xyz * _e263.zzz);
    let _e276 = vec4<f32>(_e241.x, _e241.y, _e241.z, select(0f, 1f, (-(abs(_e98.x)) > 0f)));
    let _e285 = vec4<f32>((1f - _e160.x), _e165.y, _e165.z, _e165.w);
    let _e286 = _e266.xyz;
    let _e287 = (_e286 + _e235);
    let _e291 = (vec4<f32>(_e250.x, _e250.y, _e250.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, (_e172 * 10f)).w).wxzy + vec4<f32>(2f, 0.5f, 0.5f, 0.5f));
    let _e297 = vec4<f32>((-1f + max(_e130.w, _e130.w)), _e276.y, _e276.z, _e276.w);
    let _e298 = _e287.zxy;
    let _e306 = vec4<f32>(_e297.x, (-1f + _e130.x), _e297.z, _e297.w);
    let _e308 = abs(_e306.yx);
    let _e312 = (1f - _e160.y);
    let _e317 = vec4<f32>(_e285.x, _e285.y, _e285.z, _e312);
    let _e320 = ps_PixelShaderConstants.g_ShadowMapParams;
    let _e324 = clamp((clamp(max(_e308, _e308), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f)).xy + -(_e320.xx)), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e326 = (_e201.xyz + -(_e205.xyz)).zxy;
    let _e328 = (_e326 * _e291.www);
    let _e336 = vec4<f32>(_e263.x, _e263.y, _e263.z, clamp(inverseSqrt(abs(dot(_e298, _e298))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e337 = (_e191.xyz + -(_e195.xyz)).zxy;
    let _e339 = (_e337 * _e291.zzz);
    let _e344 = vec4<f32>(_e339.x, _e339.y, _e339.z, _e306.w);
    let _e352 = vec4<f32>(vec4<f32>().x, (_e205.y + _e328.z), vec4<f32>().z, vec4<f32>().w);
    let _e355 = (_e188.zxy * _e291.yyy);
    let _e363 = vec4<f32>((_e195.y + _e339.z), _e352.y, _e352.z, _e352.w);
    let _e365 = (_e153.ww * vec2<f32>(1.2f, 0.2f));
    let _e370 = vec4<f32>(_e291.x, _e365.x, _e365.y, _e291.w);
    let _e378 = vec4<f32>(vec4<f32>().x, (_e205.x + _e328.y), vec4<f32>().z, vec4<f32>().w);
    let _e381 = (_e127.www * _e100.xyz);
    let _e389 = vec4<f32>((_e195.x + _e339.y), _e378.y, _e378.z, _e378.w);
    let _e390 = _e381.xyz;
    let _e399 = vec4<f32>(vec4<f32>().x, (_e205.z + _e328.x), vec4<f32>().z, vec4<f32>().w);
    let _e410 = vec4<f32>((_e195.z + _e339.x), _e399.y, _e399.z, _e399.w);
    let _e413 = (_e287.xyz * _e336.www);
    let _e427 = (_e390 * _e390).zxy;
    let _e453 = ps_PixelShaderConstants.g_ShadowMapParams[1i];
    let _e457 = (vec4<f32>(_e413.x, _e413.y, _e413.z, _e230.w) * _e127.wwwy);
    let _e468 = vec4<f32>(_e344.x, max((_e453 * _e324.x), (_e453 * _e324.y)), _e344.z, _e344.w);
    let _e471 = dot((_e317.xwxy * _e317.wyzz).wzxy, select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (vec4<f32>(_e138.x, _e142.x, _e146.x, _e150.x) >= _e457.wwww)).wzxy);
    let _e478 = vec4<f32>(_e468.x, _e468.y, exp2(_e291.x), _e468.w);
    let _e490 = (vec4<f32>((1f - _e471), _e370.y, _e370.z, _e370.w).xyz * _e478.yzz);
    let _e497 = vec4<f32>(clamp((1.0526316f * (-0.05f + dot(_e276.zxy, _e104))), 0f, 1f), _e100.y, _e100.z, _e100.w);
    let _e500 = ps_PixelShaderConstants.g_Ambient;
    let _e507 = (_e490.x + max(_e471, _e471));
    let _e512 = vec4<f32>(_e336.x, _e336.y, _e336.z, _e507);
    let _e514 = min(max(_e490.yz, vec2<f32>(1f, 1f)).xy, vec2<f32>(512f, 512f));
    let _e521 = vec4<f32>(_e512.x, _e512.y, clamp(log2(clamp(dot(_e457.zxy, _e104), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e512.w);
    let _e524 = (_e514.yx * _e521.zz);
    let _e529 = vec4<f32>(_e478.x, _e524.x, _e524.y, _e478.w);
    let _e532 = ps_PixelShaderConstants.mrgGlobalLight_Specular[0i];
    let _e538 = vec4<f32>((_e532 * _e507), _e529.y, _e529.z, _e529.w);
    let _e541 = ps_PixelShaderConstants.mrgGlobalLight_Diffuse;
    let _e554 = vec4<f32>(vec4<f32>().x, (_e514.y + -(_e514.x)), vec4<f32>().z, vec4<f32>().w);
    let _e556 = exp2(_e524.y);
    let _e563 = vec4<f32>((exp2(_e524.x) + -(_e556)), _e554.y, _e554.z, _e554.w);
    let _e566 = ps_PixelShaderConstants.mrgGlobalLight_Specular[1i];
    let _e572 = vec4<f32>(_e538.x, (_e566 * _e507), _e538.z, _e538.w);
    let _e574 = (_e563.xy * vec2<f32>(0.6f, 0.6f));
    let _e577 = ps_PixelShaderConstants.mrgGlobalLight_Specular[2i];
    let _e583 = vec4<f32>(_e572.x, _e572.y, (_e577 * _e507), _e572.w);
    let _e590 = (_e574.y + _e514.x);
    let _e596 = vec4<f32>(_e521.x, _e521.y, (_e556 + max(_e574.x, _e574.x)), _e521.w);
    let _e599 = (_e583.zyx * _e596.zzz);
    let _e605 = (0.0625f * _e590);
    phi_471_ = vec4<f32>(_e599.x, _e599.y, _e599.z, _e563.w);
    phi_474_ = _e583;
    phi_476_ = 0i;
    loop {
        let _e607 = phi_471_;
        let _e609 = phi_474_;
        let _e611 = phi_476_;
        local = _e607;
        local_1 = _e607;
        if (_e611 < 4i) {
            continue;
        } else {
            break;
        }
        continuing {
            let _e616 = ps_PixelShaderConstants.mrgLocalLight0_Position;
            let _e619 = ps_PixelShaderConstants.mrgLocalLight1_Position;
            let _e625 = ps_PixelShaderConstants.mrgLocalLight0_Range;
            let _e628 = ps_PixelShaderConstants.mrgLocalLight1_Range;
            let _e631 = _e609.www;
            let _e632 = (_e631 + vec3<f32>(-2f, -3f, -4f));
            let _e635 = ps_PixelShaderConstants.mrgLocalLight0_Color;
            let _e638 = ps_PixelShaderConstants.mrgLocalLight1_Color;
            let _e642 = (_e632.xxx == vec3<f32>(0f, 0f, 0f));
            let _e644 = ps_PixelShaderConstants.mrgLocalLight2_Color;
            let _e651 = ps_PixelShaderConstants.mrgLocalLight2_Range;
            let _e656 = ps_PixelShaderConstants.mrgLocalLight2_Position;
            let _e661 = (_e632.yyy == vec3<f32>(0f, 0f, 0f));
            let _e663 = ps_PixelShaderConstants.mrgLocalLight3_Position;
            let _e670 = ps_PixelShaderConstants.mrgLocalLight3_Range;
            let _e675 = ps_PixelShaderConstants.mrgLocalLight3_Color;
            let _e680 = (_e632.zzz == vec3<f32>(0f, 0f, 0f));
            let _e682 = ps_PixelShaderConstants.mrgLocalLight4_Color;
            let _e689 = ps_PixelShaderConstants.mrgLocalLight4_Range;
            let _e692 = select(select(select(select(_e628.zw, _e625.zw, (_e609.ww == vec2<f32>(0f, 0f))).xy, _e651.zw, (_e632.xx == vec2<f32>(0f, 0f))).xy, _e670.zw, (_e632.yy == vec2<f32>(0f, 0f))).xy, _e689.zw, (_e632.zz == vec2<f32>(0f, 0f)));
            let _e697 = vec4<f32>(_e692.x, _e692.y, vec4<f32>().z, vec4<f32>().w);
            let _e699 = ps_PixelShaderConstants.mrgLocalLight4_Position;
            let _e705 = (-(select(select(select(select(_e619.xyz, _e616.xyz, (_e609.www == vec3<f32>(0f, 0f, 0f))).xyz, _e656.xyz, _e642).xyz, _e663.xyz, _e661).xyz, _e699.xyz, _e680).xyz) + _e211);
            let _e706 = _e705.zxy;
            let _e708 = abs(dot(_e706, _e706));
            let _e720 = ((-(_e705.xyz) * vec4<f32>(clamp(inverseSqrt(_e708), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx) + _e286);
            let _e721 = _e720.zxy;
            let _e747 = (_e514.xy * vec4<f32>(clamp(log2(clamp(dot((_e720.zyx * vec4<f32>(clamp(inverseSqrt(abs(dot(_e721, _e721))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx).xzy, _e381.zxy), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xx);
            let _e754 = vec4<f32>(_e697.x, _e697.y, exp2(_e747.y), _e697.w);
            let _e761 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, (_e609.w + 1f));
            let _e763 = exp2(_e747.x);
            let _e768 = vec4<f32>(_e754.x, _e754.y, _e754.z, _e763);
            let _e772 = (_e768.zy + -(_e768.wx));
            let _e777 = vec4<f32>(_e772.x, _e761.y, _e772.y, _e761.w);
            let _e786 = vec4<f32>(((_e772.x * 0.6f) + _e763), _e777.y, _e777.z, _e777.w);
            let _e800 = vec4<f32>(_e786.x, (1f - clamp(((-(_e692.x) + sqrt(_e708)) * clamp((1f / _e772.y), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)), 0f, 1f)), _e786.z, _e786.w);
            let _e808 = (((_e800.yyy * select(select(select(select(_e638.xyz, _e635.xyz, (_e631 == vec3<f32>(0f, 0f, 0f))).xyz, _e644.xyz, _e642).xyz, _e675.xyz, _e661).xyz, _e682.xyz, _e680).xyz).zyx * _e800.xxx) + _e607.xyz);
            phi_471_ = vec4<f32>(_e808.x, _e808.y, _e808.z, _e607.w);
            phi_474_ = _e800;
            phi_476_ = (_e611 + 1i);
        }
    }
    let _e816 = textureSample(ps_sampFalloff_t2d, ps_sampFalloff_s, _e596.xy);
    let _e822 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, ((_e168 * 0.0625f) * _e590), vec4<f32>().w);
    let _e825 = ps_PixelShaderConstants.mrgEyeLight_Attribute[1i];
    let _e828 = ps_PixelShaderConstants.mrgEyeLight_Attribute[2i];
    let _e831 = (_e211 + -(_e215));
    let _e835 = ps_PixelShaderConstants.g_Diffuse[3i];
    let _e838 = _e816.xyz;
    let _e840 = ps_PixelShaderConstants.mrgLuminanceRange;
    let _e845 = ps_PixelShaderConstants.g_OpacityReflectionRefractionSpectype[0i];
    let _e847 = min((_e845 * (_e835 * _e132.w)), 4f);
    let _e856 = (((_e838 * _e840.xxx).xyz * _e816.www) + _e838);
    let _e861 = vec4<f32>(_e856.x, _e856.y, _e856.z, _e230.w);
    let _e862 = _e831.zxy;
    let _e866 = ps_PixelShaderConstants.mrgEyeLight_Range[3i];
    let _e869 = ps_PixelShaderConstants.mrgEyeLight_Range[2i];
    let _e872 = _e381.zxy;
    let _e873 = dot(_e266.zxy, _e872);
    let _e874 = abs(dot(_e862, _e862));
    let _e886 = ps_PixelShaderConstants.mrgEyeLight_Diffuse;
    let _e894 = (_e831.xyz * vec4<f32>(vec4<f32>().x, vec4<f32>().y, clamp(inverseSqrt(_e874), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().w).zzz);
    let _e902 = vec4<f32>((-(_e869) - -(sqrt(_e874))), _e822.y, _e822.z, _e822.w);
    let _e906 = (-(_e894.zyx) + _e266.zyx);
    let _e909 = _e894.zxy;
    let _e911 = ps_PixelShaderConstants.g_EyeDirection;
    let _e916 = _e906.xzy;
    let _e917 = dot(_e916, _e916);
    let _e918 = vec4<f32>(clamp((1f / (_e866 - _e869)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e873, clamp((1f / (_e825 - _e828)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e917);
    let _e926 = vec4<f32>(_e902.x, (-(_e828) - -(dot(_e909, _e911.zxy))), _e902.z, _e902.w);
    let _e930 = clamp((_e926.yx * _e918.zx), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e935 = vec4<f32>(_e930.x, _e918.y, _e930.y, _e918.w);
    let _e952 = ps_PixelShaderConstants.g_SonicSkinFalloffParam[2i];
    let _e955 = (vec2<f32>(1f, 3f) - _e930.yx);
    let _e959 = ps_PixelShaderConstants.mrgEyeLight_Attribute[0i];
    let _e962 = select(1f, _e955.x, (_e959 > 0f));
    let _e967 = vec4<f32>(_e962, _e926.y, _e926.z, _e926.w);
    let _e970 = ps_PixelShaderConstants.g_SonicSkinFalloffParam[1i];
    let _e986 = (_e514.xy * vec4<f32>(clamp(log2(clamp(dot((_e906.xyz * vec4<f32>(clamp(inverseSqrt(abs(_e917)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx).xzy, _e872), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xx);
    let _e990 = exp2(_e986.x);
    let _e995 = vec4<f32>(_e935.x, _e935.y, _e990, _e935.w);
    let _e1013 = vec4<f32>((0.6f * (exp2(_e986.y) + -(_e990))), _e995.y, _e995.z, _e995.w);
    let _e1016 = (_e1013.xy + _e1013.zy);
    let _e1021 = vec4<f32>(_e967.x, _e1016.x, _e967.z, _e1016.y);
    let _e1025 = ((_e390 * _e1016.yyy) + -(_e286));
    let _e1030 = vec4<f32>(_e1025.x, _e1025.y, _e1025.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, (exp2((_e952 * clamp(log2(clamp((1f - _e873), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f))) * _e970)).w);
    let _e1031 = _e1025.xyz;
    let _e1033 = ((_e1031 * vec3<f32>(0.5f, 0.5f, 0.5f)) + vec3<f32>(0.5f, 0.5f, 0.5f));
    let _e1035 = (_e326 * _e1033.yyy);
    let _e1037 = (_e337 * _e1033.xxx);
    let _e1042 = vec4<f32>(_e1037.x, _e1037.y, _e1037.z, _e497.w);
    let _e1049 = vec4<f32>(vec4<f32>().x, (_e205.y + _e1035.z), vec4<f32>().z, vec4<f32>().w);
    let _e1052 = (_e188.yzx * _e1033.zzz);
    let _e1059 = vec4<f32>((_e195.y + _e1037.z), _e1049.y, _e1049.z, _e1049.w);
    let _e1068 = vec4<f32>(vec4<f32>().x, (_e205.z + _e1035.x), vec4<f32>().z, vec4<f32>().w);
    let _e1075 = vec4<f32>(vec4<f32>().x, dot(_e872, -(_e909)), vec4<f32>().z, vec4<f32>().w);
    let _e1082 = vec4<f32>((_e195.z + _e1037.x), _e1068.y, _e1068.z, _e1068.w);
    let _e1090 = vec4<f32>((_e195.x + _e1037.y), _e1042.y, _e1042.z, _e1042.w);
    let _e1097 = vec4<f32>(_e1090.x, _e1090.y, (_e1052.z + _e185.x), _e1090.w);
    let _e1104 = vec4<f32>(_e1097.x, (_e205.x + _e1035.y), _e1097.z, _e1097.w);
    let _e1105 = (_e1031 * _e1031).zxy;
    let _e1128 = (vec4<f32>(dot(_e1105, _e1104.zxy), dot(_e1105, vec4<f32>(_e1082.x, _e1082.y, (_e185.z + _e1052.y), _e1082.w).zxy), dot(_e1105, vec4<f32>(_e1059.x, _e1059.y, (_e185.y + _e1052.x), _e1059.w).zxy), (_e930.x * _e930.x)) * vec4<f32>(_e861.x, _e861.y, _e861.z, (_e955.y + -(_e930.x))).xzyw);
    let _e1136 = (vec4<f32>((0.5f * _e507), _e1075.y, _e1075.z, _e1075.w).xy + vec2<f32>(0.5f, -0.05f));
    let _e1141 = vec4<f32>(_e1052.x, _e1052.y, _e1136.x, _e1136.y);
    let _e1148 = ps_PixelShaderConstants.mrgEyeLight_Attribute[3i];
    let _e1152 = ps_PixelShaderConstants.mrgEyeLight_Specular[0i];
    let _e1155 = ps_PixelShaderConstants.mrgEyeLight_Specular[3i];
    let _e1161 = vec4<f32>((_e1152 * _e1155), _e1030.y, _e1030.z, _e1030.w);
    let _e1173 = select(1f, clamp(exp2((clamp(log2(abs(_e1128.w)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * _e1148)), 0f, 1f), (select(0f, 1f, (_e959 > 1f)) > 0f));
    let _e1178 = vec4<f32>(_e1173, _e1141.y, _e1141.z, _e1141.w);
    let _e1184 = vec4<f32>(_e1178.x, (_e962 * _e1173), _e1178.z, _e1178.w);
    let _e1187 = ps_PixelShaderConstants.mrgEyeLight_Specular[1i];
    let _e1193 = vec4<f32>(_e1161.x, (_e1187 * _e1155), _e1161.z, _e1161.w);
    let _e1199 = ps_PixelShaderConstants.mrgEyeLight_Specular[2i];
    let _e1208 = (vec4<f32>(_e1193.x, _e1193.y, (_e1199 * _e1155), _e1193.w).wxyz * _e1184.zxyy);
    let _e1215 = ps_PixelShaderConstants.g_Diffuse;
    let _e1222 = (_e1208.yzw * _e1021.xyy);
    let _e1227 = vec4<f32>(_e1208.x, _e1222.x, _e1222.y, _e1222.z);
    let _e1240 = ps_PixelShaderConstants.g_SonicSkinFalloffParam[0i];
    let _e1252 = ps_PixelShaderConstants.g_Specular[0i];
    let _e1258 = vec4<f32>((_e1252 * _e605), _e1021.y, _e1021.z, _e1021.w);
    let _e1260 = local;
    let _e1268 = ps_PixelShaderConstants.g_Specular[1i];
    let _e1297 = ((min(((((((((_e886.xyz * _e886.www).xyz * _e1184.yyy).xyz * vec4<f32>(clamp((_e1136.y * 1.0526316f), 0f, 1f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx) + ((_e521.www * _e541.xyz).xyz * _e497.xxx).xyz).xzy * _e1215.xzy) + (vec4<f32>(dot(_e427, vec4<f32>(_e410.x, _e410.y, (_e185.z + _e355.x), _e410.w).zxy), dot(_e427, vec4<f32>(_e389.x, _e389.y, (_e185.x + _e355.y), _e389.w).zxy), dot(_e427, vec4<f32>(_e363.x, _e363.y, (_e185.y + _e355.z), _e363.w).zxy), _e312).yxz * _e500.xzy).xyz).xyz * _e132.xzy) + ((((vec4<f32>(_e1258.x, (_e1268 * _e605), _e1258.z, _e1258.w).xyz * (_e1260.zxy + vec4<f32>((_e1222.x * _e1016.x), _e1227.y, _e1227.z, _e1227.w).xwz).xzy).xzy * _e153.xzy).xyz * _e1104.www) + (_e1128.xzy * vec4<f32>((_e1240 + max(_e1208.x, _e1208.x)), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx).xzy).xyz).xzy, vec3<f32>(4f, 4f, 4f)).xyz * _e102.xyz) + _e103.xyz);
    let _e1304 = ps_SharedConstants.g_SpecConstantsValue;
    if ((_e1304 & 2u) != 0u) {
        let _e1308 = ps_SharedConstants.g_AlphaThreshold;
        if ((_e847 - _e1308) < 0f) {
            discard;
        }
    }
    ps_out_u002e_var_u002e_SV_Target0_ = vec4<f32>(_e1297.x, _e1297.y, _e1297.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e847).w);
    return;
}

@fragment 
fn fragment(@builtin(position) chr_frag: vec4<f32>, @location(0) in_u002e_var_u002e_TEXCOORD0_: vec4<f32>, @location(1) in_u002e_var_u002e_TEXCOORD1_: vec4<f32>, @location(2) in_u002e_var_u002e_TEXCOORD2_: vec4<f32>, @location(3) in_u002e_var_u002e_TEXCOORD3_: vec4<f32>, @location(4) in_u002e_var_u002e_TEXCOORD4_: vec4<f32>, @location(5) in_u002e_var_u002e_TEXCOORD5_: vec4<f32>) -> @location(0) vec4<f32> {
    chr_shadow = chr_fetch_shadow(chr_frag);
    ps_in_u002e_var_u002e_TEXCOORD0_1 = in_u002e_var_u002e_TEXCOORD0_;
    ps_in_u002e_var_u002e_TEXCOORD1_1 = in_u002e_var_u002e_TEXCOORD1_;
    ps_in_u002e_var_u002e_TEXCOORD2_1 = in_u002e_var_u002e_TEXCOORD2_;
    ps_in_u002e_var_u002e_TEXCOORD3_1 = in_u002e_var_u002e_TEXCOORD3_;
    ps_in_u002e_var_u002e_TEXCOORD4_1 = in_u002e_var_u002e_TEXCOORD4_;
    ps_in_u002e_var_u002e_TEXCOORD5_1 = in_u002e_var_u002e_TEXCOORD5_;
    ps_main_1();
    let _e13 = ps_out_u002e_var_u002e_SV_Target0_;
    return _e13;
}

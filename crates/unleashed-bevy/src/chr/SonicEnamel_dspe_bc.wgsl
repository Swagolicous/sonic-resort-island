// Composed by re/xenos/compose.py from CharactorDefault_@b@.wgsl (vertex) and SonicEnamel_dspe@c@.wgsl (pixel): the game's own programs, resources in the material group; shadow-map taps answered by the host (host_shadow).

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
@group(#{MATERIAL_BIND_GROUP}) @binding(26) 
var ps_sampEnv_tcube: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(27) 
var ps_sampEnv_s: sampler;
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
    var local: array<vec3<f32>, 2>;
    var phi_426_: vec4<f32>;
    var phi_429_: vec4<f32>;
    var phi_431_: vec4<f32>;
    var phi_433_: vec4<f32>;
    var phi_435_: i32;
    var local_1: vec4<f32>;
    var local_2: vec4<f32>;
    var local_3: vec4<f32>;
    var local_4: vec4<f32>;
    var local_5: vec4<f32>;
    var local_6: vec4<f32>;

    let _e94 = ps_in_u002e_var_u002e_TEXCOORD0_1;
    let _e95 = ps_in_u002e_var_u002e_TEXCOORD1_1;
    let _e96 = ps_in_u002e_var_u002e_TEXCOORD2_1;
    let _e97 = ps_in_u002e_var_u002e_TEXCOORD3_1;
    let _e98 = ps_in_u002e_var_u002e_TEXCOORD4_1;
    let _e99 = ps_in_u002e_var_u002e_TEXCOORD5_1;
    local = array<vec3<f32>, 2>();
    let _e100 = _e96.zxy;
    let _e104 = clamp((1f / _e97.w), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e109 = vec4<f32>(vec4<f32>().x, _e104, vec4<f32>().z, vec4<f32>().w);
    let _e115 = vec4<f32>(_e109.x, _e109.y, (_e104 + _e104), _e109.w);
    let _e118 = clamp(inverseSqrt(abs(dot(_e100, _e100))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e123 = vec4<f32>(_e118, _e115.y, _e115.z, _e115.w);
    let _e126 = (_e123.yzyz * _e97.xxyy);
    let _e127 = _e126.xz;
    let _e128 = textureDimensions(ps_g_ShadowMapSampler_t2d, 0i);
    let _e129 = vec2<f32>(_e128);
    let _e132 = chr_shadow_tap(0i);
    let _e136 = chr_shadow_tap(1i);
    let _e140 = chr_shadow_tap(2i);
    let _e144 = chr_shadow_tap(3i);
    let _e148 = textureSample(ps_sampSpec_t2d, ps_sampSpec_s, _e94.xy);
    let _e155 = select(fract(((_e127 * _e129) - vec2<f32>(0.5f, 0.5f))), vec2<f32>(0f, 0f), ((bitcast<vec2<u32>>(_e127) & vec2<u32>(2147483647u, 2147483647u)) > vec2<u32>(2139095040u, 2139095040u)));
    let _e160 = vec4<f32>(_e155.x, vec4<f32>(0f, 0f, 0f, 0f).y, _e155.y, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e163 = ps_PixelShaderConstants.g_PowerGlossLevel[1i];
    let _e172 = ps_PixelShaderConstants.g_aLightField[3i];
    let _e176 = ps_PixelShaderConstants.g_aLightField[2i];
    let _e182 = ps_PixelShaderConstants.g_aLightField[1i];
    let _e186 = ps_PixelShaderConstants.g_aLightField[0i];
    let _e192 = ps_PixelShaderConstants.g_aLightField[5i];
    let _e196 = ps_PixelShaderConstants.g_aLightField[4i];
    let _e202 = _e95.xyz;
    let _e205 = ps_PixelShaderConstants.g_EyePosition;
    let _e206 = _e205.xyz;
    let _e208 = max(0f, 0f);
    let _e209 = (-(_e202) + _e206).zxy;
    let _e211 = max(max(_e97.z, _e97.z), _e208);
    let _e222 = _e123.xxx;
    let _e224 = (_e222 * _e96.xyz);
    let _e229 = vec4<f32>(vec4<f32>(0f, 0f, 0f, 0f).x, _e224.x, _e224.y, _e224.z);
    let _e235 = vec4<f32>(_e94.x, _e94.y, _e94.z, (0.5f * _e118));
    let _e238 = (_e235.www * _e96.zyx);
    let _e251 = vec4<f32>(_e235.x, _e235.y, clamp(inverseSqrt(abs(dot(_e209, _e209))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e235.w);
    let _e253 = (_e209 * _e251.zzz);
    let _e256 = ps_PixelShaderConstants.mrgGlobalLight_Direction;
    let _e259 = (_e253.xzy + -(_e256.zyx));
    let _e262 = (vec4<f32>(vec4<f32>((_e163 * 10f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).x, _e238.x, _e238.y, _e238.z) + vec4<f32>(2f, 0.5f, 0.5f, 0.5f));
    let _e268 = vec4<f32>(_e251.x, _e251.y, (-1f + max(_e126.w, _e126.w)), _e251.w);
    let _e269 = _e259.xzy;
    let _e277 = vec4<f32>(_e268.x, _e268.y, _e268.z, (-1f + _e126.y));
    let _e279 = abs(_e277.wz);
    let _e288 = vec4<f32>(_e160.x, (1f - _e155.x), _e160.z, _e160.w);
    let _e291 = ps_PixelShaderConstants.g_ShadowMapParams;
    let _e295 = clamp((clamp(max(_e279, _e279), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f)).xy + -(_e291.xx)), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e300 = vec4<f32>(_e277.x, _e277.y, _e295.x, _e295.y);
    let _e302 = (1f - _e155.y);
    let _e307 = vec4<f32>(_e288.x, _e288.y, _e288.z, _e302);
    let _e310 = ((_e192.zyx + -(_e196.zyx)).xzy * _e262.yyy);
    let _e321 = ((_e182.zyx + -(_e186.zyx)).xzy * _e262.www);
    let _e323 = exp2(_e262.x);
    let _e326 = ((_e172.zyx + -(_e176.zyx)).xzy * _e262.zzz);
    let _e337 = dot((_e222 * -(_e256.xyz)).zxy, _e100);
    let _e343 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, clamp(_e337, 0f, 1f));
    let _e353 = vec4<f32>((_e186.y + _e321.z), (_e186.x + _e321.y), (_e176.x + _e326.y), (_e196.x + _e310.y));
    let _e357 = _e224.xyz;
    let _e362 = vec4<f32>((_e186.z + _e321.x), (_e176.z + _e326.x), _e337, _e302);
    let _e365 = (_e259.xyz * vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, clamp(inverseSqrt(abs(dot(_e269, _e269))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)).www);
    let _e378 = vec4<f32>(_e362.x, _e362.y, _e362.z, (_e196.z + _e310.x));
    let _e379 = (_e357 * _e357).zxy;
    let _e386 = (_e148.w * max(_e323, _e323));
    let _e393 = max(vec4<f32>(_e378.x, _e378.y, _e378.z, _e386).wz, vec2<f32>(1f, 0f));
    let _e401 = vec4<f32>(_e353.x, (_e176.y + _e326.z), _e353.z, _e353.w);
    let _e403 = (vec4<f32>(_e365.x, _e365.y, _e365.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, ((-(_e211) * select(0f, 1f, (_e211 >= _e97.w))) + _e211)).w) * _e123.xxxy);
    let _e414 = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (vec4<f32>(_e132.x, _e136.x, _e140.x, _e144.x) >= _e403.wwww));
    let _e417 = ps_PixelShaderConstants.g_ShadowMapParams[1i];
    let _e426 = min(_e393.x, 512f);
    let _e431 = vec4<f32>(_e426, _e229.y, _e229.z, _e229.w);
    let _e435 = dot((_e307.ywyx * _e307.wxzz).wzxy, _e414.wzxy);
    let _e444 = ps_PixelShaderConstants.g_Ambient;
    let _e446 = (vec4<f32>(dot(_e379, _e378.wxy), dot(_e379, _e353.wyz), dot(_e379, vec4<f32>(_e401.x, _e401.y, (_e196.y + _e310.z), _e401.w).zxy), _e386).yzx * _e444.xyz);
    let _e454 = (((1f - _e435) * max((_e417 * _e295.x), (_e417 * _e295.y))) + _e435);
    let _e459 = vec4<f32>(_e454, _e96.y, _e96.z, _e96.w);
    let _e462 = ps_PixelShaderConstants.mrgGlobalLight_Specular;
    let _e466 = (_e459.xxx * _e462.xyz).xyz;
    let _e468 = (_e466 * _e393.yyy);
    let _e479 = vec4<f32>(_e300.x, _e300.y, _e300.z, exp2((clamp(log2(clamp(dot(_e403.xzy, _e100), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * max(_e426, _e426))));
    let _e494 = ((_e466 * _e479.www).xyz * _e343.www);
    let _e505 = vec4<f32>(_e459.x, (0.125f * _e426), _e459.z, _e459.w);
    phi_426_ = vec4<f32>(_e468.x, _e468.y, _e468.z, _e414.w);
    phi_429_ = vec4<f32>(_e494.x, _e494.y, _e494.z, _e343.w);
    phi_431_ = vec4<f32>(_e446.x, _e446.y, _e446.z, _e393.y);
    phi_433_ = vec4<f32>(_e479.x, _e479.y, _e479.z, select(0f, 1f, (-(abs(_e94.x)) > 0f)));
    phi_435_ = 0i;
    loop {
        let _e507 = phi_426_;
        let _e509 = phi_429_;
        let _e511 = phi_431_;
        let _e513 = phi_433_;
        let _e515 = phi_435_;
        local_1 = _e513;
        local_2 = _e509;
        local_3 = _e507;
        local_4 = _e507;
        local_5 = _e511;
        local_6 = _e511;
        if (_e515 < 4i) {
            continue;
        } else {
            break;
        }
        continuing {
            let _e517 = _e513.www;
            let _e518 = (_e517 == vec3<f32>(0f, 0f, 0f));
            let _e520 = ps_PixelShaderConstants.mrgLocalLight0_Position;
            let _e523 = ps_PixelShaderConstants.mrgLocalLight1_Position;
            let _e529 = ps_PixelShaderConstants.mrgLocalLight0_Range;
            let _e532 = ps_PixelShaderConstants.mrgLocalLight1_Range;
            let _e535 = (_e517 + vec3<f32>(-2f, -3f, -4f));
            let _e537 = ps_PixelShaderConstants.mrgLocalLight0_Color;
            let _e540 = ps_PixelShaderConstants.mrgLocalLight1_Color;
            let _e544 = (_e535.xxx == vec3<f32>(0f, 0f, 0f));
            let _e546 = ps_PixelShaderConstants.mrgLocalLight2_Color;
            let _e553 = ps_PixelShaderConstants.mrgLocalLight2_Range;
            let _e558 = ps_PixelShaderConstants.mrgLocalLight2_Position;
            let _e563 = (_e535.yyy == vec3<f32>(0f, 0f, 0f));
            let _e565 = ps_PixelShaderConstants.mrgLocalLight3_Position;
            let _e572 = ps_PixelShaderConstants.mrgLocalLight3_Range;
            let _e577 = ps_PixelShaderConstants.mrgLocalLight3_Color;
            let _e582 = (_e535.zzz == vec3<f32>(0f, 0f, 0f));
            let _e584 = ps_PixelShaderConstants.mrgLocalLight4_Color;
            let _e591 = ps_PixelShaderConstants.mrgLocalLight4_Range;
            let _e594 = select(select(select(select(_e532.zw, _e529.zw, (_e513.ww == vec2<f32>(0f, 0f))).xy, _e553.zw, (_e535.xx == vec2<f32>(0f, 0f))).xy, _e572.zw, (_e535.yy == vec2<f32>(0f, 0f))).xy, _e591.zw, (_e535.zz == vec2<f32>(0f, 0f)));
            let _e596 = ps_PixelShaderConstants.mrgLocalLight4_Position;
            let _e602 = (-(select(select(select(select(_e523.xyz, _e520.xyz, _e518).xyz, _e558.xyz, _e544).xyz, _e565.xyz, _e563).xyz, _e596.xyz, _e582).xyz) + _e202);
            let _e603 = _e602.zxy;
            let _e607 = -(_e594.x);
            let _e609 = abs(dot(_e603, _e603));
            let _e611 = (_e607 + sqrt(_e609));
            let _e618 = vec4<f32>(_e509.x, _e509.y, _e509.z, clamp(inverseSqrt(_e609), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
            let _e621 = (_e602.xyz * _e618.www);
            let _e627 = (-(_e621.xyz) + _e253.yzx);
            let _e628 = _e431.wyz;
            let _e631 = dot(_e628, -(_e621.zxy));
            let _e638 = vec4<f32>(_e511.x, _e511.y, _e511.z, max(_e631, 0f));
            let _e641 = _e627.zxy;
            let _e648 = vec4<f32>(_e507.x, _e507.y, _e507.z, (1f - clamp((clamp((1f / (_e594.y + _e607)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * max(_e611, _e611)), 0f, 1f)));
            let _e659 = vec4<f32>(_e618.x, _e618.y, _e618.z, clamp(inverseSqrt(abs(dot(_e641, _e641))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
            let _e660 = (_e648.www * select(select(select(select(_e540.xyz, _e537.xyz, _e518).xyz, _e546.xyz, _e544).xyz, _e577.xyz, _e563).xyz, _e584.xyz, _e582).xyz).xyz;
            let _e664 = ((_e660 * _e638.www) + _e648.xyz);
            let _e691 = vec4<f32>(_e659.x, _e659.y, _e659.z, clamp(_e631, 0f, 1f));
            let _e697 = vec4<f32>(_e638.x, _e638.y, _e638.z, exp2((_e426 * clamp(log2(clamp(dot((_e627.zyx * _e659.www).xzy, _e628), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f))));
            let _e704 = (((_e660 * _e697.www).xyz * _e691.www) + _e691.xyz);
            phi_426_ = vec4<f32>(_e664.x, _e664.y, _e664.z, _e648.w);
            phi_429_ = vec4<f32>(_e704.x, _e704.y, _e704.z, _e691.w);
            phi_431_ = _e697;
            phi_433_ = vec4<f32>(_e513.x, _e513.y, _e513.z, (_e513.w + 1f));
            phi_435_ = (_e515 + 1i);
        }
    }
    let _e712 = (_e202 + -(_e206));
    let _e713 = _e712.zxy;
    let _e717 = ps_PixelShaderConstants.mrgEyeLight_Attribute[1i];
    let _e720 = ps_PixelShaderConstants.mrgEyeLight_Attribute[2i];
    let _e724 = ps_PixelShaderConstants.mrgEyeLight_Range[3i];
    let _e727 = ps_PixelShaderConstants.mrgEyeLight_Range[2i];
    let _e728 = -(_e727);
    let _e729 = (_e724 + _e728);
    let _e730 = abs(dot(_e713, _e713));
    let _e738 = _e253.xyz;
    let _e740 = _e431.wyz;
    let _e741 = dot(-(_e738), _e740);
    let _e752 = vec4<f32>((_e728 - -(sqrt(_e730))), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e753 = (_e712.xyz * vec4<f32>(vec4<f32>().x, vec4<f32>().y, clamp(inverseSqrt(_e730), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().w).zzz).zxy;
    let _e754 = -(_e753);
    let _e755 = (_e754 + _e738);
    let _e760 = vec4<f32>(_e712.x, _e755.x, _e755.y, _e755.z);
    let _e764 = ps_PixelShaderConstants.g_EyeDirection;
    let _e770 = _e755.xyz;
    let _e783 = clamp((vec4<f32>(_e752.x, (-(_e720) - -(dot(_e753, _e764.zxy))), _e752.z, _e752.w).xy * vec4<f32>(_e741, _e729, clamp((1f / _e729), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), clamp((1f / (_e717 - _e720)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)).zw), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e785 = local_1;
    let _e790 = vec4<f32>(_e785.x, _e785.y, _e783.x, _e783.y);
    let _e792 = (vec2<f32>(3f, 1f) - _e783.yx);
    let _e807 = vec4<f32>(_e790.x, _e790.y, (_e792.x + -(_e783.y)), _e790.w);
    let _e815 = vec4<f32>(_e807.x, _e807.y, _e807.z, clamp(inverseSqrt(abs(dot(_e770, _e770))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e818 = (vec4<f32>((_e783.y * _e783.y), _e760.y, _e760.z, _e760.w).xywz * _e815.zwww);
    let _e827 = vec4<f32>(_e815.x, _e815.y, _e815.z, (_e741 + _e741));
    let _e834 = vec4<f32>(_e827.x, _e827.y, clamp(log2(clamp(dot(_e818.ywz, _e740), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e827.w);
    let _e837 = (_e431.yzwx * _e834.wwwz);
    local[u32()] = (-(_e253.yzx) + -(_e837.xyz)).xyz;
    let _e855 = local[u32(_e208)];
    let _e856 = textureSample(ps_sampEnv_tcube, ps_sampEnv_s, _e855);
    let _e858 = textureSample(ps_sampDif_t2d, ps_sampDif_s, _e834.xy);
    let _e863 = ps_PixelShaderConstants.g_Diffuse[3i];
    let _e865 = _e856.xyz;
    let _e867 = ps_PixelShaderConstants.mrgLuminanceRange;
    let _e872 = ps_PixelShaderConstants.g_OpacityReflectionRefractionSpectype[0i];
    let _e878 = min((_e872 * (_e858.w * _e863)), 4f);
    let _e886 = ps_PixelShaderConstants.g_Specular;
    let _e890 = ps_PixelShaderConstants.mrgEyeLight_Specular;
    let _e895 = ps_PixelShaderConstants.mrgEyeLight_Diffuse;
    let _e901 = ps_PixelShaderConstants.mrgEyeLight_Attribute[0i];
    let _e904 = dot(_e740, _e754);
    let _e907 = local_2;
    let _e912 = vec4<f32>(_e907.x, _e907.y, _e907.z, clamp(_e904, 0f, 1f));
    let _e921 = _e148.xyz;
    let _e922 = ((((_e865 * _e867.xxx).xyz * _e856.www) + _e865).xyz * _e921);
    let _e927 = vec4<f32>(vec4<f32>().x, _e922.x, _e922.y, _e922.z);
    let _e930 = ps_PixelShaderConstants.mrgEyeLight_Attribute[3i];
    let _e937 = vec4<f32>(max(_e904, 0f), _e927.y, _e927.z, _e927.w);
    let _e947 = vec4<f32>(_e98.x, _e98.y, _e98.z, (select(1f, _e792.y, (_e901 > 0f)) * select(1f, clamp(exp2((_e930 * clamp(log2(abs(_e818.x)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f))), 0f, 1f), (select(0f, 1f, (_e901 > 1f)) > 0f))));
    let _e948 = _e947.www;
    let _e959 = vec4<f32>(_e947.x, _e947.y, _e947.z, exp2(_e837.w));
    let _e967 = local_3;
    let _e973 = ps_PixelShaderConstants.g_Diffuse;
    let _e977 = local_5;
    let _e1011 = ((min((((((((_e948 * (_e895.xyz * _e895.www).xyz).xyz * _e937.xxx) + _e967.xyz).xyz * _e973.xyz) + _e977.xyz).xyz * _e858.xyz) + (((((_e505.yyy * _e886.xyz).xyz * ((((_e948 * (_e890.xyz * _e890.www).xyz).xyz * _e959.www).xyz * _e912.www) + _e912.xyz).xyz).xyz * _e921) + (_e937.yzw * (vec4<f32>(vec4<f32>().x, vec4<f32>().y, (0.7f * _e454), vec4<f32>().w).xyz + vec3<f32>(1.5f, 1.5f, 0.3f)).zzz).xyz).xyz * _e505.www).xyz).xyz, vec3<f32>(4f, 4f, 4f)).xyz * _e959.xyz) + _e99.xyz);
    let _e1018 = ps_SharedConstants.g_SpecConstantsValue;
    if ((_e1018 & 2u) != 0u) {
        let _e1022 = ps_SharedConstants.g_AlphaThreshold;
        if ((_e878 - _e1022) < 0f) {
            discard;
        }
    }
    ps_out_u002e_var_u002e_SV_Target0_ = vec4<f32>(_e1011.x, _e1011.y, _e1011.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e878).w);
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

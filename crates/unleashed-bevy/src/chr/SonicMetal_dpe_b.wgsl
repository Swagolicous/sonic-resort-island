// Composed by re/xenos/compose.py from CharactorSphereRef_@b@.wgsl (vertex) and SonicMetal_dpe@@.wgsl (pixel): the game's own programs, resources in the material group; shadow-map taps answered by the host (host_shadow).

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
    var phi_127_: vec4<f32>;
    var phi_135_: vec4<f32>;
    var phi_142_: vec4<f32>;
    var phi_150_: vec4<f32>;
    var phi_186_: vec4<f32>;
    var phi_216_: vec4<f32>;
    var phi_219_: vec4<f32>;
    var phi_221_: vec4<f32>;
    var phi_223_: vec4<f32>;
    var phi_225_: vec4<f32>;
    var phi_227_: i32;
    var local: vec4<f32>;
    var local_1: vec4<f32>;
    var local_2: vec4<f32>;
    var local_3: vec4<f32>;
    var local_4: vec4<f32>;
    var local_5: vec4<f32>;
    var local_6: vec4<f32>;
    var local_7: vec4<f32>;
    var phi_367_: vec4<f32>;
    var phi_368_: vec4<f32>;
    var phi_369_: vec4<f32>;

    let _e82 = vs_in_u002e_var_u002e_POSITION0_1;
    let _e83 = vs_in_u002e_var_u002e_TEXCOORD0_1;
    let _e84 = vs_in_u002e_var_u002e_TEXCOORD1_1;
    let _e85 = vs_in_u002e_var_u002e_TEXCOORD2_1;
    let _e86 = vs_in_u002e_var_u002e_TEXCOORD3_1;
    let _e87 = vs_in_u002e_var_u002e_NORMAL0_1;
    let _e88 = vs_in_u002e_var_u002e_BLENDWEIGHT0_1;
    let _e89 = vs_in_u002e_var_u002e_BLENDINDICES0_1;
    let _e91 = vs_SharedConstants.g_SwappedTexcoords;
    if ((_e91 & 1u) != 0u) {
        phi_127_ = _e83.yxwz;
    } else {
        phi_127_ = _e83;
    }
    let _e96 = phi_127_;
    let _e101 = vec4<f32>(vec4<f32>(0f, 0f, 0f, 0f).x, _e96.x, _e96.y, vec4<f32>(0f, 0f, 0f, 0f).w);
    if ((_e91 & 2u) != 0u) {
        phi_135_ = _e84.yxwz;
    } else {
        phi_135_ = _e84;
    }
    let _e106 = phi_135_;
    if ((_e91 & 4u) != 0u) {
        phi_142_ = _e85.yxwz;
    } else {
        phi_142_ = _e85;
    }
    let _e111 = phi_142_;
    let _e116 = vec4<f32>(_e106.x, _e106.y, _e111.x, _e111.y);
    if ((_e91 & 8u) != 0u) {
        phi_150_ = _e86.yxwz;
    } else {
        phi_150_ = _e86;
    }
    let _e121 = phi_150_;
    let _e123 = vs_SharedConstants.g_SpecConstantsValue;
    switch bitcast<i32>(0u) {
        default: {
            if ((_e123 & 1u) != 0u) {
                phi_186_ = vec4<f32>((select(0f, -1f, ((_e87.x & 1024u) != 0u)) + (f32((_e87.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e87.x & 2097152u) != 0u)) + (f32(((_e87.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e87.x & 2147483648u) != 0u)) + (f32(((_e87.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_186_ = bitcast<vec4<f32>>(_e87);
                break;
            }
        }
    }
    let _e156 = phi_186_;
    let _e161 = vec4<f32>(_e156.x, _e156.y, _e156.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e163 = vec4<f32>(_e89.wxyz);
    let _e165 = vs_SharedConstants.g_Booleans;
    phi_367_ = _e101;
    phi_368_ = _e161;
    phi_369_ = _e82;
    if ((_e165 & 1u) != 0u) {
        let _e171 = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(vec4<f32>(0f, 0f, 0f, 0f))) > vec4<f32>(0f, 0f, 0f, 0f)));
        let _e175 = select(0f, 1f, (-(abs(0f)) > 0f));
        let _e180 = vec4<f32>(_e175, _e101.y, _e101.z, _e101.w);
        phi_216_ = _e161;
        phi_219_ = _e171;
        phi_221_ = _e171;
        phi_223_ = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(_e180.xxxx)) > vec4<f32>(0f, 0f, 0f, 0f)));
        phi_225_ = vec4<f32>(_e180.x, _e180.y, _e180.z, select(0f, 1f, (-(abs(_e175)) > 0f)));
        phi_227_ = 0i;
        loop {
            let _e196 = phi_216_;
            let _e198 = phi_219_;
            let _e200 = phi_221_;
            let _e202 = phi_223_;
            let _e204 = phi_225_;
            let _e206 = phi_227_;
            local = _e204;
            local_1 = _e204;
            local_2 = _e202;
            local_3 = _e200;
            local_4 = _e198;
            local_5 = _e196;
            local_6 = _e196;
            local_7 = _e196;
            if (_e206 < 3i) {
                continue;
            } else {
                break;
            }
            continuing {
                let _e209 = (_e204.x == 0f);
                let _e214 = (_e204.xx + vec2<f32>(-2f, -3f));
                let _e219 = (_e214.x == 0f);
                let _e225 = (_e214.y == 0f);
                let _e227 = select(select(select(_e88.y, _e88.x, _e209), _e88.z, _e219), _e88.w, _e225);
                let _e232 = vec4<f32>(_e196.x, _e196.y, _e196.z, _e227);
                let _e241 = vec4<f32>((1f + _e204.x), _e204.y, _e204.z, _e204.w);
                let _e252 = i32(clamp(floor(((select(select(select(_e163.z, _e163.y, _e209), _e163.w, _e219), _e163.x, _e225) * 3f) + 0.5f)), -256f, 255f));
                let _e253 = _e232.wwww;
                let _e259 = vs_VertexShaderConstants.g_MtxPalette[min(_e252, 227i)];
                let _e264 = (1i + _e252);
                let _e270 = vs_VertexShaderConstants.g_MtxPalette[min(_e264, 227i)];
                let _e275 = (2i + _e252);
                let _e281 = vs_VertexShaderConstants.g_MtxPalette[min(_e275, 227i)];
                phi_216_ = _e232;
                phi_219_ = ((_e253 * select(vec4<f32>(0f, 0f, 0f, 0f), _e259, vec4((_e252 < 228i))).wzyx) + _e198);
                phi_221_ = ((_e253 * select(vec4<f32>(0f, 0f, 0f, 0f), _e270, vec4((_e264 < 228i))).wzyx) + _e200);
                phi_223_ = ((_e253 * select(vec4<f32>(0f, 0f, 0f, 0f), _e281, vec4((_e275 < 228i))).wzyx) + _e202);
                phi_225_ = vec4<f32>(_e241.x, _e241.y, _e241.z, (_e227 + _e204.w));
                phi_227_ = (_e206 + 1i);
            }
        }
        let _e290 = (3f * _e163.x);
        let _e292 = local;
        let _e296 = local_1;
        let _e301 = vec4<f32>((1f - _e292.w), _e296.y, _e296.z, _e296.w);
        let _e305 = i32(clamp(floor((_e290 + 0.5f)), -256f, 255f));
        let _e306 = _e301.xxxx;
        let _e307 = (2i + _e305);
        let _e313 = vs_VertexShaderConstants.g_MtxPalette[min(_e307, 227i)];
        let _e318 = local_2;
        let _e319 = ((_e306 * select(vec4<f32>(0f, 0f, 0f, 0f), _e313, vec4((_e307 < 228i))).wzyx) + _e318);
        let _e320 = (1i + _e305);
        let _e326 = vs_VertexShaderConstants.g_MtxPalette[min(_e320, 227i)];
        let _e331 = local_3;
        let _e332 = ((_e306 * select(vec4<f32>(0f, 0f, 0f, 0f), _e326, vec4((_e320 < 228i))).wzyx) + _e331);
        let _e338 = vs_VertexShaderConstants.g_MtxPalette[min(_e305, 227i)];
        let _e343 = local_4;
        let _e344 = ((_e306 * select(vec4<f32>(0f, 0f, 0f, 0f), _e338, vec4((_e305 < 228i))).wzyx) + _e343);
        let _e349 = vec4<f32>(_e290, max(_e82.y, _e82.y), max(_e82.z, _e82.z), max(_e82.w, _e82.w));
        let _e358 = vec4<f32>(max(_e82.x, _e82.x), _e349.y, _e349.z, _e349.w).wzxy;
        let _e364 = vec4<f32>(dot(_e344.xywz, _e358), _e82.y, _e82.z, _e82.w);
        let _e371 = vec4<f32>(_e364.x, dot(_e332.xywz, _e358), _e364.z, _e364.w);
        let _e380 = local_5;
        let _e387 = vec4<f32>(max(_e380.x, _e380.x), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e389 = local_6;
        let _e396 = vec4<f32>(_e387.x, _e387.y, max(_e389.z, _e389.z), _e387.w);
        let _e398 = local_7;
        let _e407 = vec4<f32>(_e396.x, max(_e398.y, _e398.y), _e396.z, _e396.w).zxy;
        let _e413 = vec4<f32>(dot(_e344.ywz, _e407), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e420 = vec4<f32>(_e413.x, dot(_e332.ywz, _e407), _e413.z, _e413.w);
        phi_367_ = _e301;
        phi_368_ = vec4<f32>(_e420.x, _e420.y, dot(_e319.ywz, _e407), _e420.w);
        phi_369_ = vec4<f32>(_e371.x, _e371.y, dot(_e319.xywz, _e358), _e371.w);
    }
    let _e429 = phi_367_;
    let _e431 = phi_368_;
    let _e433 = phi_369_;
    let _e437 = vs_VertexShaderConstants.g_MtxWorld[3i];
    let _e443 = vs_VertexShaderConstants.g_MtxWorld[2i];
    let _e451 = vs_VertexShaderConstants.g_MtxWorld[1i];
    let _e458 = vs_VertexShaderConstants.g_MtxWorld[0i];
    let _e461 = ((_e433.xxxx * _e458.xzyw) + ((_e433.yyyy * _e451.xzyw) + ((_e433.zzzz * _e443.xzyw) + (_e433.wwww * _e437.xyzw).xzyw)));
    let _e465 = vs_VertexShaderConstants.g_MtxView[3i];
    let _e468 = _e461.yyyy;
    let _e471 = vs_VertexShaderConstants.g_MtxView[2i];
    let _e476 = _e461.zzzz;
    let _e479 = vs_VertexShaderConstants.g_MtxView[1i];
    let _e484 = _e461.xxxx;
    let _e487 = vs_VertexShaderConstants.g_MtxView[0i];
    let _e491 = ((_e484 * _e487.zywx) + ((_e476 * _e479.xzyw) + ((_e468 * _e471.xwyz) + (_e461.wwww * _e465.xwzy).xywz).xwzy).yzwx);
    let _e495 = vs_VertexShaderConstants.g_MtxProjection[3i];
    let _e501 = vs_VertexShaderConstants.g_MtxProjection[2i];
    let _e508 = vs_VertexShaderConstants.g_MtxProjection[1i];
    let _e516 = vs_VertexShaderConstants.g_MtxProjection[0i];
    let _e520 = ((_e491.wwww * _e516.xyzw) + ((_e491.yyyy * _e508.xzyw) + ((_e491.xxxx * _e501.xwzy) + (_e491.zzzz * _e495.xwzy)).xzwy).xzyw);
    let _e523 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[3i];
    let _e526 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[0i];
    let _e532 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, (_e523 * _e526));
    let _e538 = vs_VertexShaderConstants.mrgTexcoordIndex[0i];
    let _e539 = trunc(_e538);
    let _e554 = ((_e431.yyy * _e451.xyz) + (_e431.zzz * _e443.xyz).xyz);
    let _e559 = ((_e431.xxx * _e458.xzy) + _e554.xzy);
    let _e560 = _e559.yxz;
    let _e562 = _e461.xzy;
    let _e565 = vs_VertexShaderConstants.g_EyePosition;
    let _e567 = (-(_e562) + _e565.xyz);
    let _e572 = vec4<f32>(_e554.x, _e567.x, _e567.y, _e567.z);
    let _e575 = vs_VertexShaderConstants.g_MtxLightViewProjection[2i];
    let _e580 = vs_VertexShaderConstants.g_MtxLightViewProjection[1i];
    let _e584 = _e567.zxy;
    let _e593 = vec4<f32>(clamp(inverseSqrt(abs(dot(_e560, _e560))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e116.y, _e116.z, _e116.w);
    let _e596 = (_e559.xzy * _e593.xxx);
    let _e601 = vec4<f32>(_e596.x, _e596.y, _e596.z, _e431.w);
    let _e609 = vec4<f32>(clamp(inverseSqrt(abs(dot(_e584, _e584))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e572.y, _e572.z, _e572.w);
    let _e612 = vs_VertexShaderConstants.g_MtxLightViewProjection[0i];
    let _e615 = ((_e484 * _e612.xzwy) + ((_e476 * _e580.xzwy) + (_e468 * _e575.xzwy)));
    let _e618 = (_e609.yzw * _e609.xxx);
    let _e621 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[0i];
    let _e624 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[1i];
    let _e625 = (_e621 + _e624);
    let _e627 = -(_e491.x);
    let _e631 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[1i];
    let _e636 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[0i];
    let _e638 = _e618.zxy;
    let _e640 = vs_VertexShaderConstants.mrgGlobalLight_Direction;
    let _e643 = dot(_e638, -(_e640.zxy));
    let _e648 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[2i];
    let _e652 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[1i];
    let _e653 = ((_e643 * _e648) + _e652);
    let _e657 = max(_e615.w, _e615.w);
    let _e659 = dot(_e638, _e596.zxy);
    let _e663 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[2i];
    let _e664 = (_e663 * clamp(((_e627 + -(_e631)) * clamp((1f / (_e636 - _e631)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)), 0f, 1f));
    let _e667 = vec4<f32>(_e664, _e653, _e657, (_e664 * -(_e625)));
    let _e670 = vs_VertexShaderConstants.g_MtxLightViewProjection[3i];
    let _e682 = (_e615.xzy + _e670.xwz);
    let _e692 = (vec4<f32>(_e682.x, vec4<f32>(vec4<f32>().x, (-(_e670.y) + -(_e657)), vec4<f32>().z, vec4<f32>().w).y, _e682.y, _e682.z).zxy * vec3<f32>(0.5f, 0.5f, 0.5f));
    let _e693 = (1f - clamp(_e659, 0f, 1f));
    let _e694 = (_e659 + _e659);
    let _e699 = vec4<f32>(_e593.x, _e694, _e593.z, _e593.w);
    let _e700 = (_e693 * _e693);
    let _e703 = (_e692.yz + _e692.xx);
    let _e704 = max(_e562, _e562);
    let _e709 = vec4<f32>(_e704.x, _e704.y, _e704.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e718 = (_e615.yz + _e670.zw);
    let _e731 = vec4<f32>(((_e596.z * _e694) + (1f - max(_e618.z, _e618.z))), _e532.y, _e532.z, _e532.w);
    let _e737 = ((_e596.xy * _e699.yy) + -(_e618.xy));
    let _e742 = vec4<f32>(_e731.x, _e737.x, _e737.y, _e731.w);
    let _e745 = vs_VertexShaderConstants.mrgFresnelParam[0i];
    let _e749 = vec4<f32>(_e539, ((_e700 * _e700) * _e693), _e643, (1f - _e745));
    let _e759 = (vec4<f32>(_e667.x, clamp(log2(abs(_e653)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e667.z, _e667.w).wy * vec2<f32>(1.442695f, 1.5f));
    let _e761 = exp2(_e759.x);
    let _e762 = vec3(_e761);
    let _e767 = vec4<f32>(_e762.x, _e762.y, _e762.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e773 = _e742.xyz;
    let _e777 = (_e749.yz * _e749.wz);
    let _e784 = sqrt(abs(dot(_e773, _e773)));
    let _e792 = vs_VertexShaderConstants.mrgFresnelParam[1i];
    let _e798 = vec4<f32>(_e601.x, _e601.y, _e601.z, ((_e777.x + _e745) * _e792));
    let _e805 = vec4<f32>(clamp((1f / exp2(_e759.y)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e807 = (vec4<f32>(_e749.x, _e777.x, _e777.y, _e749.w).zxx + vec3<f32>(1f, -2f, -3f));
    let _e819 = select(select(_e106.xy, _e429.yz, (vec4<f32>(_e539, vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xx == vec2<f32>(0f, 0f))).xy, _e699.zw, (_e807.yy == vec2<f32>(0f, 0f)));
    let _e824 = select(_e819.xy, _e121.xy, (_e807.zz == vec2<f32>(0f, 0f)));
    let _e828 = (_e742.wyz * vec4<f32>(_e805.x, clamp((1f / (_e784 + _e784)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e805.z, _e805.w).xyy);
    let _e830 = clamp((1f / _e625), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e835 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e830);
    let _e837 = (_e828.yz + vec2<f32>(0.5f, 0.5f));
    let _e846 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[2i];
    let _e849 = ((_e807.x * _e846) + _e828.x);
    let _e850 = (_e849 * _e830);
    let _e858 = vs_VertexShaderConstants.g_LightScatteringColor;
    let _e860 = (vec4<f32>(_e835.x, _e835.y, _e850, _e835.w).zzw * _e858.xyz);
    let _e865 = vec4<f32>(_e860.x, _e860.y, _e860.z, _e819.y);
    let _e879 = vs_VertexShaderConstants.g_LightScatteringFarNearScale;
    let _e881 = ((vec4<f32>(_e865.x, _e865.y, _e865.z, (_e860.z * _e849)).xyw * vec4<f32>((1f - _e761), _e849, _e850, _e830).xxx).xyz * _e879.www);
    let _e886 = vec4<f32>(_e881.x, _e881.y, _e881.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e893 = vs_SharedConstants.g_HalfPixelOffset;
    let _e897 = (_e520.xy + (_e893 * _e520.w));
    vs_global = vec4<f32>(_e897.x, _e897.y, _e520.z, _e520.w);
    vs_out_u002e_var_u002e_TEXCOORD0_ = vec4<f32>(_e824.x, _e824.y, _e837.x, _e837.y);
    vs_out_u002e_var_u002e_TEXCOORD1_ = vec4<f32>(_e709.x, _e709.y, _e709.z, max(_e627, _e627));
    vs_out_u002e_var_u002e_TEXCOORD2_ = max(_e798, _e798);
    vs_out_u002e_var_u002e_TEXCOORD3_ = vec4<f32>(_e703.x, _e703.y, _e718.x, _e718.y);
    vs_out_u002e_var_u002e_TEXCOORD4_ = vec4<f32>(_e767.x, _e767.y, _e767.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD5_ = vec4<f32>(_e886.x, _e886.y, _e886.z, 0f);
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
@group(#{MATERIAL_BIND_GROUP}) @binding(24) 
var ps_sampEnv_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(27) 
var ps_sampEnv_s: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(36) 
var ps_sampPower_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(39) 
var ps_sampPower_s: sampler;
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
    var phi_413_: vec4<f32>;
    var phi_416_: vec4<f32>;
    var phi_418_: vec4<f32>;
    var phi_420_: i32;
    var local: vec4<f32>;
    var local_1: vec4<f32>;
    var local_2: vec4<f32>;
    var local_3: vec4<f32>;
    var local_4: vec4<f32>;
    var local_5: vec4<f32>;

    let _e89 = ps_in_u002e_var_u002e_TEXCOORD0_1;
    let _e90 = ps_in_u002e_var_u002e_TEXCOORD1_1;
    let _e91 = ps_in_u002e_var_u002e_TEXCOORD2_1;
    let _e92 = ps_in_u002e_var_u002e_TEXCOORD3_1;
    let _e93 = ps_in_u002e_var_u002e_TEXCOORD4_1;
    let _e94 = ps_in_u002e_var_u002e_TEXCOORD5_1;
    let _e95 = _e91.zxy;
    let _e99 = clamp((1f / _e92.w), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e104 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e99);
    let _e105 = (_e99 + _e99);
    let _e113 = clamp(inverseSqrt(abs(dot(_e95, _e95))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e116 = (vec4<f32>(_e105, _e104.y, _e104.z, _e104.w).xwwx * _e92.xxyy);
    let _e117 = _e116.yz;
    let _e118 = textureDimensions(ps_g_ShadowMapSampler_t2d, 0i);
    let _e119 = vec2<f32>(_e118);
    let _e122 = chr_shadow_tap(0i);
    let _e126 = chr_shadow_tap(1i);
    let _e130 = chr_shadow_tap(2i);
    let _e134 = chr_shadow_tap(3i);
    let _e137 = _e89.xy;
    let _e138 = textureSample(ps_sampPower_t2d, ps_sampPower_s, _e137);
    let _e144 = vec4<f32>(_e93.x, _e93.y, _e93.z, _e138.x);
    let _e151 = select(fract(((_e117 * _e119) - vec2<f32>(0.5f, 0.5f))), vec2<f32>(0f, 0f), ((bitcast<vec2<u32>>(_e117) & vec2<u32>(2147483647u, 2147483647u)) > vec2<u32>(2139095040u, 2139095040u)));
    let _e156 = vec4<f32>(_e151.x, vec4<f32>(0f, 0f, 0f, 0f).y, _e151.y, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e159 = ps_PixelShaderConstants.g_PowerGlossLevel[1i];
    let _e168 = ps_PixelShaderConstants.g_aLightField[3i];
    let _e172 = ps_PixelShaderConstants.g_aLightField[2i];
    let _e178 = ps_PixelShaderConstants.g_aLightField[1i];
    let _e182 = ps_PixelShaderConstants.g_aLightField[0i];
    let _e188 = ps_PixelShaderConstants.g_aLightField[5i];
    let _e192 = ps_PixelShaderConstants.g_aLightField[4i];
    let _e201 = ps_PixelShaderConstants.g_EyePosition;
    let _e202 = _e201.xyz;
    let _e204 = max(0f, 0f);
    let _e205 = (-(_e90.xyz) + _e202).zxy;
    let _e207 = max(max(_e92.z, _e92.z), _e204);
    let _e208 = vec4<f32>(_e105, _e113, _e207, _e99);
    let _e220 = _e91.zyx;
    let _e221 = (_e208.yyy * _e220);
    let _e229 = (vec4<f32>((0.5f * _e113), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx * _e220);
    let _e242 = vec4<f32>(_e90.x, _e90.y, _e90.z, clamp(inverseSqrt(abs(dot(_e205, _e205))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e244 = (_e205 * _e242.www);
    let _e247 = ps_PixelShaderConstants.mrgGlobalLight_Direction;
    let _e250 = (_e244.xzy + -(_e247.zyx));
    let _e253 = (vec4<f32>(vec4<f32>((_e159 * 10f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).x, _e229.x, _e229.y, _e229.z) + vec4<f32>(2f, 0.5f, 0.5f, 0.5f));
    let _e259 = vec4<f32>((-1f + max(_e116.w, _e116.w)), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e260 = _e250.xzy;
    let _e270 = abs(vec4<f32>(_e259.x, (-1f + _e116.x), _e259.z, _e259.w).yx);
    let _e279 = vec4<f32>(_e156.x, (1f - _e151.x), _e156.z, _e156.w);
    let _e282 = ps_PixelShaderConstants.g_ShadowMapParams;
    let _e286 = clamp((clamp(max(_e270, _e270), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f)).xy + -(_e282.xx)), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e288 = (1f - _e151.y);
    let _e293 = vec4<f32>(_e279.x, _e279.y, _e279.z, _e288);
    let _e296 = ((_e188.zyx + -(_e192.zyx)).xzy * _e253.yyy);
    let _e307 = ((_e178.zyx + -(_e182.zyx)).xzy * _e253.www);
    let _e309 = exp2(_e253.x);
    let _e312 = ((_e168.zyx + -(_e172.zyx)).xzy * _e253.zzz);
    let _e324 = dot((_e208.yyy * -(_e247.xyz)).zxy, _e95);
    let _e330 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, clamp(_e324, 0f, 1f));
    let _e340 = vec4<f32>((_e182.y + _e307.z), (_e182.x + _e307.y), (_e172.x + _e312.y), (_e192.x + _e296.y));
    let _e344 = _e221.zyx;
    let _e349 = vec4<f32>((_e182.z + _e307.x), (_e172.z + _e312.x), _e324, _e288);
    let _e352 = (_e250.xyz * vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, clamp(inverseSqrt(abs(dot(_e260, _e260))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)).www);
    let _e365 = vec4<f32>(_e349.x, _e349.y, _e349.z, (_e192.z + _e296.x));
    let _e366 = (_e344 * _e344).zxy;
    let _e372 = (_e138.x * max(_e309, _e309));
    let _e379 = max(vec4<f32>(_e365.x, _e365.y, _e365.z, _e372).zw, vec2<f32>(0f, 1f));
    let _e387 = vec4<f32>(_e340.x, (_e172.y + _e312.z), _e340.z, _e340.w);
    let _e389 = (vec4<f32>(_e352.x, _e352.y, _e352.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, ((-(_e207) * select(0f, 1f, (_e207 >= _e92.w))) + _e207)).w) * _e208.yyyw);
    let _e403 = ps_PixelShaderConstants.g_ShadowMapParams[1i];
    let _e412 = min(_e379.y, 512f);
    let _e416 = dot((_e293.ywyx * _e293.wxzz).wzxy, select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (vec4<f32>(_e122.x, _e126.x, _e130.x, _e134.x) >= _e389.wwww)).wzxy);
    let _e425 = ps_PixelShaderConstants.g_Ambient;
    let _e427 = (vec4<f32>(dot(_e366, _e365.wxy), dot(_e366, _e340.wyz), dot(_e366, vec4<f32>(_e387.x, _e387.y, (_e192.y + _e296.z), _e387.w).zxy), _e372).yzx * _e425.xyz);
    let _e432 = vec4<f32>(_e427.x, _e427.y, _e427.z, vec4<f32>().w);
    let _e435 = (((1f - _e416) * max((_e403 * _e286.x), (_e403 * _e286.y))) + _e416);
    let _e440 = vec4<f32>(_e91.x, _e91.y, _e435, _e91.w);
    let _e443 = ps_PixelShaderConstants.mrgGlobalLight_Specular;
    let _e447 = (_e440.zzz * _e443.xyz).xyz;
    let _e449 = (_e447 * _e379.xxx);
    let _e460 = vec4<f32>(_e440.x, exp2((clamp(log2(clamp(dot(_e389.xzy, _e95), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * max(_e412, _e412))), _e440.z, _e440.w);
    let _e475 = ((_e447 * _e460.yyy).xyz * _e330.www);
    let _e486 = vec4<f32>(_e460.x, (0.125f * _e412), _e460.z, _e460.w);
    phi_413_ = vec4<f32>(_e475.x, _e475.y, _e475.z, _e330.w);
    phi_416_ = vec4<f32>(_e432.x, _e432.y, _e432.z, select(0f, 1f, (-(abs(_e89.x)) > 0f)));
    phi_418_ = vec4<f32>(_e449.x, _e449.y, _e449.z, _e208.w);
    phi_420_ = 0i;
    loop {
        let _e488 = phi_413_;
        let _e490 = phi_416_;
        let _e492 = phi_418_;
        let _e494 = phi_420_;
        local = _e492;
        local_1 = _e492;
        local_2 = _e490;
        local_3 = _e490;
        local_4 = _e488;
        local_5 = _e488;
        if (_e494 < 4i) {
            continue;
        } else {
            break;
        }
        continuing {
            let _e496 = _e490.www;
            let _e497 = (_e496 == vec3<f32>(0f, 0f, 0f));
            let _e499 = ps_PixelShaderConstants.mrgLocalLight0_Position;
            let _e502 = ps_PixelShaderConstants.mrgLocalLight1_Position;
            let _e508 = ps_PixelShaderConstants.mrgLocalLight0_Range;
            let _e511 = ps_PixelShaderConstants.mrgLocalLight1_Range;
            let _e514 = (_e496 + vec3<f32>(-2f, -3f, -4f));
            let _e516 = ps_PixelShaderConstants.mrgLocalLight0_Color;
            let _e519 = ps_PixelShaderConstants.mrgLocalLight1_Color;
            let _e523 = (_e514.xxx == vec3<f32>(0f, 0f, 0f));
            let _e525 = ps_PixelShaderConstants.mrgLocalLight2_Color;
            let _e532 = ps_PixelShaderConstants.mrgLocalLight2_Range;
            let _e537 = ps_PixelShaderConstants.mrgLocalLight2_Position;
            let _e542 = (_e514.yyy == vec3<f32>(0f, 0f, 0f));
            let _e544 = ps_PixelShaderConstants.mrgLocalLight3_Position;
            let _e551 = ps_PixelShaderConstants.mrgLocalLight3_Range;
            let _e556 = ps_PixelShaderConstants.mrgLocalLight3_Color;
            let _e561 = (_e514.zzz == vec3<f32>(0f, 0f, 0f));
            let _e563 = ps_PixelShaderConstants.mrgLocalLight4_Color;
            let _e566 = select(select(select(select(_e519.xyz, _e516.xyz, _e497).xyz, _e525.xyz, _e523).xyz, _e556.xyz, _e542).xyz, _e563.xyz, _e561);
            let _e571 = vec4<f32>(_e566.x, _e566.y, _e566.z, _e514.z);
            let _e575 = ps_PixelShaderConstants.mrgLocalLight4_Range;
            let _e578 = select(select(select(select(_e511.zw, _e508.zw, (_e490.ww == vec2<f32>(0f, 0f))).xy, _e532.zw, (_e514.xx == vec2<f32>(0f, 0f))).xy, _e551.zw, (_e514.yy == vec2<f32>(0f, 0f))).xy, _e575.zw, (_e514.zz == vec2<f32>(0f, 0f)));
            let _e580 = ps_PixelShaderConstants.mrgLocalLight4_Position;
            let _e587 = (-(select(select(select(select(_e502.xyz, _e499.xyz, _e497).xyz, _e537.xyz, _e523).xyz, _e544.xyz, _e542).xyz, _e580.xyz, _e561).xyz) + _e242.xyz);
            let _e588 = _e587.zxy;
            let _e592 = -(_e578.x);
            let _e594 = abs(dot(_e588, _e588));
            let _e596 = (_e592 + sqrt(_e594));
            let _e603 = vec4<f32>(_e488.x, _e488.y, _e488.z, clamp(inverseSqrt(_e594), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
            let _e606 = (_e587.xyz * _e603.www);
            let _e612 = (-(_e606.xyz) + _e244.yzx);
            let _e613 = _e221.xzy;
            let _e616 = dot(_e613, -(_e606.zxy));
            let _e623 = vec4<f32>(_e492.x, _e492.y, _e492.z, max(_e616, 0f));
            let _e626 = _e612.zxy;
            let _e633 = vec4<f32>(_e571.x, _e571.y, _e571.z, (1f - clamp((clamp((1f / (_e578.y + _e592)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * max(_e596, _e596)), 0f, 1f)));
            let _e644 = vec4<f32>(_e603.x, _e603.y, _e603.z, clamp(inverseSqrt(abs(dot(_e626, _e626))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
            let _e645 = (_e633.www * _e633.xyz).xyz;
            let _e649 = ((_e645 * _e623.www) + _e623.xyz);
            let _e654 = vec4<f32>(_e649.x, _e649.y, _e649.z, _e623.w);
            let _e676 = vec4<f32>(_e644.x, _e644.y, _e644.z, clamp(_e616, 0f, 1f));
            let _e682 = vec4<f32>(_e654.x, _e654.y, _e654.z, exp2((_e412 * clamp(log2(clamp(dot((_e612.zyx * _e644.www).xzy, _e613), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f))));
            let _e689 = (((_e645 * _e682.www).xyz * _e676.www) + _e676.xyz);
            phi_413_ = vec4<f32>(_e689.x, _e689.y, _e689.z, _e676.w);
            phi_416_ = vec4<f32>(_e490.x, _e490.y, _e490.z, (_e490.w + 1f));
            phi_418_ = _e682;
            phi_420_ = (_e494 + 1i);
        }
    }
    let _e697 = textureSample(ps_sampEnv_t2d, ps_sampEnv_s, _e89.zw);
    let _e698 = textureSample(ps_sampDif_t2d, ps_sampDif_s, _e137);
    let _e701 = ps_PixelShaderConstants.g_Specular;
    let _e705 = ps_PixelShaderConstants.mrgEyeLight_Specular;
    let _e711 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[0i];
    let _e714 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[3i];
    let _e725 = (_e242.xyz + -(_e202));
    let _e728 = _e697.xyz;
    let _e730 = ps_PixelShaderConstants.mrgLuminanceRange;
    let _e736 = ps_PixelShaderConstants.g_Diffuse[3i];
    let _e742 = _e725.zxy;
    let _e743 = dot(_e742, _e742);
    let _e746 = ps_PixelShaderConstants.g_OpacityReflectionRefractionSpectype[0i];
    let _e748 = min((_e746 * (_e736 * max(_e698.w, _e698.w))), 4f);
    let _e756 = ps_PixelShaderConstants.mrgEyeLight_Attribute[0i];
    let _e758 = select(0f, 1f, (_e756 > 1f));
    let _e759 = abs(_e743);
    let _e762 = vec4<f32>(_e743, _e758, clamp(inverseSqrt(_e759), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), ((_e435 * 0.7f) + 0.3f));
    let _e765 = ((((_e728 * _e730.xxx).xyz * _e697.www) + _e728).xyz * _e144.www);
    let _e770 = vec4<f32>(_e697.x, _e765.x, _e765.y, _e765.z);
    let _e774 = (_e725.xyz * _e762.zzz);
    let _e779 = vec4<f32>(vec4<f32>((_e711 * _e714), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).x, _e774.x, _e774.y, _e774.z);
    let _e782 = ps_PixelShaderConstants.mrgEyeLight_Range[3i];
    let _e785 = ps_PixelShaderConstants.mrgEyeLight_Range[2i];
    let _e787 = _e221.xzy;
    let _e788 = _e774.zxy;
    let _e789 = -(_e788);
    let _e790 = dot(_e787, _e789);
    let _e794 = ps_PixelShaderConstants.mrgEyeLight_Attribute[1i];
    let _e797 = ps_PixelShaderConstants.mrgEyeLight_Attribute[2i];
    let _e800 = (_e789 + _e244.xyz);
    let _e805 = vec4<f32>(_e244.x, _e800.x, _e800.y, _e800.z);
    let _e813 = vec4<f32>(_e486.x, _e486.y, (-(_e785) - -(sqrt(_e759))), _e486.w);
    let _e817 = ps_PixelShaderConstants.g_EyeDirection;
    let _e822 = vec4<f32>(_e790, clamp((1f / (_e794 - _e797)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), clamp((1f / (_e782 - _e785)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), clamp(_e790, 0f, 1f));
    let _e823 = _e800.xyz;
    let _e832 = vec4<f32>(_e813.x, (-(_e797) - -(dot(_e788, _e817.zxy))), _e813.z, _e813.w);
    let _e836 = clamp((_e832.yz * _e822.yz), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e841 = vec4<f32>(_e822.x, _e836.x, _e836.y, _e822.w);
    let _e844 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[1i];
    let _e850 = vec4<f32>(_e779.x, (_e844 * _e714), _e779.z, _e779.w);
    let _e852 = (vec2<f32>(1f, 3f) - _e836.yx);
    let _e875 = vec4<f32>((_e852.y + -(_e836.x)), _e762.y, _e762.z, _e762.w);
    let _e883 = vec4<f32>(_e875.x, _e875.y, clamp(inverseSqrt(abs(dot(_e823, _e823))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e875.w);
    let _e886 = (vec4<f32>((_e836.x * _e836.x), _e805.y, _e805.z, _e805.w).xywz * _e883.xzzz);
    let _e889 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[2i];
    let _e905 = ps_PixelShaderConstants.mrgEyeLight_Attribute[3i];
    let _e920 = (vec4<f32>(select(1f, _e852.x, (_e756 > 0f)), _e770.y, _e770.z, _e770.w) * vec4<f32>(_e883.x, _e883.y, select(1f, clamp(exp2((clamp(log2(abs(_e886.x)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * _e905)), 0f, 1f), (_e758 > 0f)), _e883.w).zwww);
    let _e922 = _e920.xxx;
    let _e932 = vec4<f32>(_e841.x, _e841.y, exp2((_e412 * clamp(log2(clamp(dot(_e886.ywz, _e787), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f))), _e841.w);
    let _e941 = vec4<f32>(max(max(_e790, _e790), _e204), _e932.y, _e932.z, _e932.w);
    let _e946 = local;
    let _e952 = ps_PixelShaderConstants.g_Diffuse;
    let _e956 = local_2;
    let _e965 = local_4;
    let _e989 = ((min((((((((_e922 * vec4<f32>(_e850.x, _e850.y, (_e889 * _e714), _e850.w).xyz).xyz * _e941.xxx) + _e946.xyz).xyz * _e952.xyz) + _e956.xyz).xyz * _e698.xyz) + ((((_e486.yyy * _e701.xyz).xyz * ((((_e922 * (_e705.xyz * _e705.www).xyz).xyz * _e932.zzz).xyz * _e941.www) + _e965.xyz).xyz) + _e920.yzw).xyz * _e832.www).xyz).xyz, vec3<f32>(4f, 4f, 4f)).xyz * _e144.xyz) + _e94.xyz);
    let _e996 = ps_SharedConstants.g_SpecConstantsValue;
    if ((_e996 & 2u) != 0u) {
        let _e1000 = ps_SharedConstants.g_AlphaThreshold;
        if ((_e748 - _e1000) < 0f) {
            discard;
        }
    }
    ps_out_u002e_var_u002e_SV_Target0_ = vec4<f32>(_e989.x, _e989.y, _e989.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e748).w);
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

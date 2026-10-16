// Composed by re/xenos/compose.py from SonicEye_@bnu@.wgsl (vertex) and SonicEye_dpne@c@.wgsl (pixel): the game's own programs, resources in the material group; shadow-map taps answered by the host (host_shadow).

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
    mrgTexcoordOffset: vec4<f32>,
    mrgFresnelParam: vec4<f32>,
    g_LightScattering_Ray_Mie_Ray2_Mie2_: vec4<f32>,
    g_LightScattering_ConstG_FogDensity: vec4<f32>,
    g_LightScatteringFarNearScale: vec4<f32>,
    g_LightScatteringColor: vec4<f32>,
    g_SonicEyeHighLightPosition: vec4<f32>,
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
    @location(16) member_17: vec4<f32>,
    @location(17) member_18: vec4<f32>,
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
var<private> vs_in_u002e_var_u002e_COLOR0_1: vec4<f32>;
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
    var phi_135_: vec4<f32>;
    var phi_142_: vec4<f32>;
    var phi_149_: vec4<f32>;
    var phi_156_: vec4<f32>;
    var phi_192_: vec4<f32>;
    var local: bool;
    var phi_225_: vec4<f32>;
    var local_1: bool;
    var phi_258_: vec4<f32>;
    var phi_313_: vec4<f32>;
    var phi_316_: vec4<f32>;
    var phi_318_: vec4<f32>;
    var phi_320_: vec4<f32>;
    var phi_322_: vec4<f32>;
    var phi_324_: vec4<f32>;
    var phi_326_: i32;
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
    var phi_502_: vec4<f32>;
    var phi_503_: vec4<f32>;
    var phi_504_: vec4<f32>;
    var phi_505_: vec4<f32>;
    var phi_506_: vec4<f32>;

    let _e87 = vs_in_u002e_var_u002e_POSITION0_1;
    let _e88 = vs_in_u002e_var_u002e_TEXCOORD0_1;
    let _e89 = vs_in_u002e_var_u002e_TEXCOORD1_1;
    let _e90 = vs_in_u002e_var_u002e_TEXCOORD2_1;
    let _e91 = vs_in_u002e_var_u002e_TEXCOORD3_1;
    let _e92 = vs_in_u002e_var_u002e_COLOR0_1;
    let _e93 = vs_in_u002e_var_u002e_NORMAL0_1;
    let _e94 = vs_in_u002e_var_u002e_TANGENT0_1;
    let _e95 = vs_in_u002e_var_u002e_BINORMAL0_1;
    let _e96 = vs_in_u002e_var_u002e_BLENDWEIGHT0_1;
    let _e97 = vs_in_u002e_var_u002e_BLENDINDICES0_1;
    let _e99 = vs_SharedConstants.g_SwappedTexcoords;
    if ((_e99 & 1u) != 0u) {
        phi_135_ = _e88.yxwz;
    } else {
        phi_135_ = _e88;
    }
    let _e104 = phi_135_;
    if ((_e99 & 2u) != 0u) {
        phi_142_ = _e89.yxwz;
    } else {
        phi_142_ = _e89;
    }
    let _e109 = phi_142_;
    if ((_e99 & 4u) != 0u) {
        phi_149_ = _e90.yxwz;
    } else {
        phi_149_ = _e90;
    }
    let _e114 = phi_149_;
    if ((_e99 & 8u) != 0u) {
        phi_156_ = _e91.yxwz;
    } else {
        phi_156_ = _e91;
    }
    let _e119 = phi_156_;
    let _e121 = vs_SharedConstants.g_SpecConstantsValue;
    switch bitcast<i32>(0u) {
        default: {
            let _e124 = ((_e121 & 1u) != 0u);
            local = _e124;
            local_1 = _e124;
            if _e124 {
                phi_192_ = vec4<f32>((select(0f, -1f, ((_e93.x & 1024u) != 0u)) + (f32((_e93.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e93.x & 2097152u) != 0u)) + (f32(((_e93.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e93.x & 2147483648u) != 0u)) + (f32(((_e93.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_192_ = bitcast<vec4<f32>>(_e93);
                break;
            }
        }
    }
    let _e154 = phi_192_;
    let _e159 = vec4<f32>(_e154.x, _e154.y, _e154.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    switch bitcast<i32>(0u) {
        default: {
            let _e162 = local;
            if _e162 {
                phi_225_ = vec4<f32>((select(0f, -1f, ((_e94.x & 1024u) != 0u)) + (f32((_e94.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e94.x & 2097152u) != 0u)) + (f32(((_e94.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e94.x & 2147483648u) != 0u)) + (f32(((_e94.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_225_ = bitcast<vec4<f32>>(_e94);
                break;
            }
        }
    }
    let _e192 = phi_225_;
    let _e197 = vec4<f32>(vec4<f32>(0f, 0f, 0f, 0f).x, _e192.y, _e192.x, _e192.z);
    switch bitcast<i32>(0u) {
        default: {
            let _e200 = local_1;
            if _e200 {
                phi_258_ = vec4<f32>((select(0f, -1f, ((_e95.x & 1024u) != 0u)) + (f32((_e95.x & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e95.x & 2097152u) != 0u)) + (f32(((_e95.x >> bitcast<u32>(11u)) & 1023u)) * 0.0009765625f)), (select(0f, -1f, ((_e95.x & 2147483648u) != 0u)) + (f32(((_e95.x >> bitcast<u32>(22u)) & 511u)) * 0.001953125f)), 0f);
                break;
            } else {
                phi_258_ = bitcast<vec4<f32>>(_e95);
                break;
            }
        }
    }
    let _e230 = phi_258_;
    let _e235 = vec4<f32>(_e230.x, _e230.y, _e230.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e237 = vec4<f32>(_e97.wxyz);
    let _e240 = vs_VertexShaderConstants.g_SonicEyeHighLightPosition;
    let _e243 = (_e87.xyz + -(_e240.xyz));
    let _e244 = _e243.zxy;
    let _e247 = -(_e230.zx);
    let _e248 = max(_e247, _e247);
    let _e261 = vec4<f32>(clamp(inverseSqrt(abs(dot(_e244, _e244))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e197.y, _e197.z, _e197.w);
    let _e264 = (_e243.xyz * _e261.xxx);
    let _e271 = -(_e230.y);
    let _e272 = max(_e271, _e271);
    let _e277 = vec4<f32>(_e272, _e261.y, _e261.z, _e261.w);
    let _e279 = vs_SharedConstants.g_Booleans;
    phi_502_ = vec4<f32>(_e248.x, _e248.y, vec4<f32>(0f, 0f, 0f, 0f).z, vec4<f32>(0f, 0f, 0f, 0f).w);
    phi_503_ = vec4<f32>(_e264.x, _e264.y, _e264.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    phi_504_ = _e277;
    phi_505_ = _e159;
    phi_506_ = _e87;
    if ((_e279 & 1u) != 0u) {
        let _e286 = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(_e277.xxxx)) > vec4<f32>(0f, 0f, 0f, 0f)));
        let _e290 = select(0f, 1f, (-(abs(_e272)) > 0f));
        let _e295 = vec4<f32>(_e290, _e277.y, _e277.z, _e277.w);
        phi_313_ = _e159;
        phi_316_ = _e286;
        phi_318_ = _e286;
        phi_320_ = select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (-(abs(_e295.xxxx)) > vec4<f32>(0f, 0f, 0f, 0f)));
        phi_322_ = _e295;
        phi_324_ = vec4<f32>(_e235.x, _e235.y, _e235.z, select(0f, 1f, (-(abs(_e290)) > 0f)));
        phi_326_ = 0i;
        loop {
            let _e311 = phi_313_;
            let _e313 = phi_316_;
            let _e315 = phi_318_;
            let _e317 = phi_320_;
            let _e319 = phi_322_;
            let _e321 = phi_324_;
            let _e323 = phi_326_;
            local_2 = _e321;
            local_3 = _e317;
            local_4 = _e315;
            local_5 = _e313;
            local_6 = _e319;
            local_7 = _e319;
            local_8 = _e319;
            local_9 = _e321;
            local_10 = _e321;
            local_11 = _e311;
            local_12 = _e311;
            local_13 = _e311;
            if (_e323 < 3i) {
                continue;
            } else {
                break;
            }
            continuing {
                let _e326 = (_e319.x == 0f);
                let _e331 = (_e319.xx + vec2<f32>(-2f, -3f));
                let _e336 = (_e331.x == 0f);
                let _e342 = (_e331.y == 0f);
                let _e344 = select(select(select(_e96.y, _e96.x, _e326), _e96.z, _e336), _e96.w, _e342);
                let _e349 = vec4<f32>(_e311.x, _e311.y, _e311.z, _e344);
                let _e369 = i32(clamp(floor(((select(select(select(_e237.z, _e237.y, _e326), _e237.w, _e336), _e237.x, _e342) * 3f) + 0.5f)), -256f, 255f));
                let _e370 = _e349.wwww;
                let _e376 = vs_VertexShaderConstants.g_MtxPalette[min(_e369, 227i)];
                let _e381 = (1i + _e369);
                let _e387 = vs_VertexShaderConstants.g_MtxPalette[min(_e381, 227i)];
                let _e392 = (2i + _e369);
                let _e398 = vs_VertexShaderConstants.g_MtxPalette[min(_e392, 227i)];
                phi_313_ = _e349;
                phi_316_ = ((_e370 * select(vec4<f32>(0f, 0f, 0f, 0f), _e376, vec4((_e369 < 228i))).wzyx) + _e313);
                phi_318_ = ((_e370 * select(vec4<f32>(0f, 0f, 0f, 0f), _e387, vec4((_e381 < 228i))).wzyx) + _e315);
                phi_320_ = ((_e370 * select(vec4<f32>(0f, 0f, 0f, 0f), _e398, vec4((_e392 < 228i))).wzyx) + _e317);
                phi_322_ = vec4<f32>((1f + _e319.x), _e319.y, _e319.z, _e319.w);
                phi_324_ = vec4<f32>(_e321.x, _e321.y, _e321.z, (_e344 + _e321.w));
                phi_326_ = (_e323 + 1i);
            }
        }
        let _e407 = (3f * _e237.x);
        let _e409 = local_2;
        let _e420 = i32(clamp(floor((_e407 + 0.5f)), -256f, 255f));
        let _e421 = vec4<f32>((1f - _e409.w), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxxx;
        let _e422 = (2i + _e420);
        let _e428 = vs_VertexShaderConstants.g_MtxPalette[min(_e422, 227i)];
        let _e433 = local_3;
        let _e434 = ((_e421 * select(vec4<f32>(0f, 0f, 0f, 0f), _e428, vec4((_e422 < 228i))).wzyx) + _e433);
        let _e435 = (1i + _e420);
        let _e441 = vs_VertexShaderConstants.g_MtxPalette[min(_e435, 227i)];
        let _e446 = local_4;
        let _e447 = ((_e421 * select(vec4<f32>(0f, 0f, 0f, 0f), _e441, vec4((_e435 < 228i))).wzyx) + _e446);
        let _e453 = vs_VertexShaderConstants.g_MtxPalette[min(_e420, 227i)];
        let _e458 = local_5;
        let _e459 = ((_e421 * select(vec4<f32>(0f, 0f, 0f, 0f), _e453, vec4((_e420 < 228i))).wzyx) + _e458);
        let _e464 = vec4<f32>(_e407, max(_e87.y, _e87.y), max(_e87.z, _e87.z), max(_e87.w, _e87.w));
        let _e473 = vec4<f32>(max(_e87.x, _e87.x), _e464.y, _e464.z, _e464.w).wzxy;
        let _e479 = vec4<f32>(dot(_e459.xywz, _e473), _e87.y, _e87.z, _e87.w);
        let _e486 = vec4<f32>(_e479.x, dot(_e447.xywz, _e473), _e479.z, _e479.w);
        let _e500 = vec4<f32>(max(_e264.x, _e264.x), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e507 = vec4<f32>(_e500.x, _e500.y, max(_e264.z, _e264.z), _e500.w);
        let _e515 = vec4<f32>(_e507.x, max(_e264.y, _e264.y), _e507.z, _e507.w).zxy;
        let _e516 = _e459.ywz;
        let _e522 = vec4<f32>(dot(_e515, _e516), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e523 = _e447.ywz;
        let _e529 = vec4<f32>(_e522.x, dot(_e515, _e523), _e522.z, _e522.w);
        let _e530 = _e434.ywz;
        let _e538 = local_6;
        let _e545 = vec4<f32>(max(_e538.z, _e538.z), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e547 = local_7;
        let _e554 = vec4<f32>(_e545.x, _e545.y, max(_e547.w, _e547.w), _e545.w);
        let _e556 = local_8;
        let _e564 = vec4<f32>(_e554.x, max(_e556.y, _e556.y), _e554.z, _e554.w).zxy;
        let _e568 = local_9;
        let _e572 = -(_e568.zxy);
        let _e577 = local_11;
        let _e584 = vec4<f32>(vec4<f32>().x, max(_e577.y, _e577.y), vec4<f32>().z, vec4<f32>().w);
        let _e590 = vec4<f32>(dot(_e530, _e572), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e592 = local_12;
        let _e599 = vec4<f32>(max(_e592.x, _e592.x), _e584.y, _e584.z, _e584.w);
        let _e607 = local_13;
        let _e615 = vec4<f32>(_e599.x, _e599.y, max(_e607.z, _e607.z), _e599.w).zxy;
        let _e621 = vec4<f32>(dot(_e516, _e615), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
        let _e627 = vec4<f32>(_e621.x, dot(_e523, _e615), _e621.z, _e621.w);
        phi_502_ = vec4<f32>(_e590.x, dot(_e516, _e572), _e590.z, _e590.w);
        phi_503_ = vec4<f32>(_e529.x, _e529.y, dot(_e515, _e530), _e529.w);
        phi_504_ = vec4<f32>(dot(_e523, _e572), dot(_e523, _e564), dot(_e516, _e564), dot(_e530, _e564));
        phi_505_ = vec4<f32>(_e627.x, _e627.y, dot(_e530, _e615), _e627.w);
        phi_506_ = vec4<f32>(_e486.x, _e486.y, dot(_e434.xywz, _e473), _e486.w);
    }
    let _e635 = phi_502_;
    let _e637 = phi_503_;
    let _e639 = phi_504_;
    let _e641 = phi_505_;
    let _e643 = phi_506_;
    let _e647 = vs_VertexShaderConstants.g_MtxWorld[3i];
    let _e653 = vs_VertexShaderConstants.g_MtxWorld[2i];
    let _e661 = vs_VertexShaderConstants.g_MtxWorld[1i];
    let _e668 = vs_VertexShaderConstants.g_MtxWorld[0i];
    let _e671 = ((_e643.xxxx * _e668.xzyw) + ((_e643.yyyy * _e661.xzyw) + ((_e643.zzzz * _e653.xzyw) + (_e643.wwww * _e647.xyzw).xzyw)));
    let _e675 = vs_VertexShaderConstants.g_MtxView[3i];
    let _e678 = _e671.yyyy;
    let _e681 = vs_VertexShaderConstants.g_MtxView[2i];
    let _e686 = _e671.zzzz;
    let _e689 = vs_VertexShaderConstants.g_MtxView[1i];
    let _e694 = _e671.xxxx;
    let _e697 = vs_VertexShaderConstants.g_MtxView[0i];
    let _e700 = ((_e694 * _e697.xzyw) + ((_e686 * _e689.xzyw) + ((_e678 * _e681.xwyz) + (_e671.wwww * _e675.xwzy).xywz).xwzy));
    let _e704 = vs_VertexShaderConstants.g_MtxProjection[3i];
    let _e710 = vs_VertexShaderConstants.g_MtxProjection[2i];
    let _e717 = vs_VertexShaderConstants.g_MtxProjection[1i];
    let _e725 = vs_VertexShaderConstants.g_MtxProjection[0i];
    let _e729 = ((_e700.xxxx * _e725.xyzw) + ((_e700.zzzz * _e717.xzyw) + ((_e700.yyyy * _e710.xwzy) + (_e700.wwww * _e704.xwzy)).xzwy).xzyw);
    let _e732 = vs_VertexShaderConstants.mrgFresnelParam[0i];
    let _e736 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[3i];
    let _e739 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[0i];
    let _e748 = vs_VertexShaderConstants.mrgTexcoordIndex[0i];
    let _e754 = vec4<f32>(trunc(_e748), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e756 = _e653.xyz;
    let _e757 = (_e641.zzz * _e756);
    let _e762 = vec4<f32>(vec4<f32>((_e736 * _e739), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).x, _e757.x, _e757.y, _e757.z);
    let _e770 = vec4<f32>((_e661.y * _e639.x), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e779 = vec4<f32>(_e770.x, (_e661.z * _e639.x), _e770.z, _e770.w);
    let _e784 = select(_e109.xy, _e104.xy, (_e754.xx == vec2<f32>(0f, 0f)));
    let _e789 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, _e784.x, _e784.y);
    let _e791 = _e661.xyz;
    let _e799 = ((_e635.yy * _e668.yz) + _e779.xy);
    let _e804 = vec4<f32>(_e779.x, _e799.x, _e799.y, _e779.w);
    let _e811 = (_e639.xyyy * _e661.xxyz);
    let _e823 = _e668.xzy;
    let _e830 = ((_e641.xxx * _e823) + ((_e641.yyy * _e791) + _e757.xyz).xzy);
    let _e831 = _e830.yxz;
    let _e837 = ((_e639.www * _e653.xzy) + ((_e639.zzz * _e823) + _e811.ywz).xyz);
    let _e838 = _e837.yxz;
    let _e840 = _e671.xzy;
    let _e843 = vs_VertexShaderConstants.g_EyePosition;
    let _e845 = (-(_e840) + _e843.xyz);
    let _e848 = vs_VertexShaderConstants.g_MtxLightViewProjection[2i];
    let _e854 = ((_e635.xxx * _e756) + vec4<f32>(((_e635.y * _e668.x) + _e811.x), _e804.y, _e804.z, _e804.w).xyz);
    let _e855 = _e854.zxy;
    let _e867 = (_e830.xzy * vec4<f32>(clamp(inverseSqrt(abs(dot(_e831, _e831))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx);
    let _e872 = vec4<f32>(_e867.x, _e867.y, _e867.z, _e104.y);
    let _e873 = _e845.zxy;
    let _e877 = vs_VertexShaderConstants.g_MtxLightViewProjection[1i];
    let _e883 = vs_VertexShaderConstants.g_MtxLightViewProjection[0i];
    let _e886 = ((_e694 * _e883.xzwy) + ((_e686 * _e877.xzwy) + (_e678 * _e848.xzwy)));
    let _e891 = vs_VertexShaderConstants.g_MtxLightViewProjection[3i];
    let _e903 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[0i];
    let _e906 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[1i];
    let _e910 = (_e886.xzy + _e891.xwz);
    let _e929 = (_e845.xyz * vec4<f32>(clamp(inverseSqrt(abs(dot(_e873, _e873))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx).zxy;
    let _e931 = vs_VertexShaderConstants.mrgGlobalLight_Direction;
    let _e934 = dot(_e929, -(_e931.zxy));
    let _e939 = vec4<f32>(_e762.x, _e934, _e762.z, _e762.w);
    let _e944 = (vec4<f32>(_e910.x, vec4<f32>(vec4<f32>().x, (-(_e886.w) + -(_e891.y)), vec4<f32>().z, vec4<f32>().w).y, _e910.y, _e910.z).zxy * vec3<f32>(0.5f, 0.5f, 0.5f));
    let _e949 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[2i];
    let _e953 = vs_VertexShaderConstants.g_LightScattering_ConstG_FogDensity[1i];
    let _e954 = ((_e934 * _e949) + _e953);
    let _e959 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[0i];
    let _e962 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[1i];
    let _e963 = (_e959 + _e962);
    let _e966 = vs_VertexShaderConstants.g_LightScatteringFarNearScale[2i];
    let _e967 = (clamp(((-(_e906) + -(max(_e700.y, _e700.y))) * clamp((1f / (_e903 - _e906)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)), 0f, 1f) * _e966);
    let _e971 = vec4<f32>(_e967, _e954, _e963, clamp(inverseSqrt(abs(dot(_e838, _e838))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e978 = vec4<f32>((_e967 * -(_e963)), _e971.y, _e971.z, _e971.w);
    let _e986 = vec4<f32>(_e978.x, clamp(log2(abs(_e954)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e978.z, _e978.w);
    let _e989 = (_e944.yz + _e944.xx);
    let _e990 = max(_e840, _e840);
    let _e995 = vec4<f32>(_e990.x, _e990.y, _e990.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e996 = -(_e700.y);
    let _e1005 = (_e886.yz + _e891.zw);
    let _e1014 = (_e837.xzy * _e986.www);
    let _e1019 = vec4<f32>(_e1014.x, _e1014.y, _e1014.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1029 = ((_e637.xxx * _e668.xyz) + ((_e637.yyy * _e791) + (_e637.zzz * _e756).xyz).xyz);
    let _e1034 = vec4<f32>(_e1029.x, _e1029.y, _e1029.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1041 = (_e986.xy * vec2<f32>(1.442695f, 1.5f));
    let _e1043 = exp2(_e1041.x);
    let _e1044 = vec3(_e1043);
    let _e1049 = vec4<f32>(_e1044.x, _e1044.y, _e1044.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1062 = vec4<f32>(clamp(dot(_e867.zxy, _e929), 0f, 1f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1069 = (vec2<f32>(1f, 1f) - vec4<f32>(_e1062.x, _e1043, _e1062.z, _e1062.w).xy);
    let _e1078 = vec4<f32>(_e939.x, _e939.y, _e939.z, (_e1069.x * _e1069.x));
    let _e1085 = vec4<f32>(_e1078.x, _e1078.y, clamp((1f / exp2(_e1041.y)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e1078.w);
    let _e1088 = (_e1085.wxy * _e1085.wzy);
    let _e1097 = clamp((1f / _e963), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e1102 = vec4<f32>(_e789.x, _e1097, _e789.z, _e789.w);
    let _e1104 = (vec4<f32>(_e754.x, _e1088.x, _e1088.y, _e1088.z).xwx + vec3<f32>(-2f, 1f, -3f));
    let _e1112 = vec4<f32>(_e872.x, _e872.y, _e872.z, clamp(inverseSqrt(abs(dot(_e855, _e855))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e1115 = (_e854.xyz * _e1112.www);
    let _e1120 = vec4<f32>(_e1115.x, _e1115.y, _e1115.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1130 = select(_e1102.zw, _e114.xy, (_e1104.xx == vec2<f32>(0f, 0f)));
    let _e1135 = vec4<f32>(_e1102.x, _e1102.y, _e1130.x, _e1130.y);
    let _e1139 = vs_VertexShaderConstants.g_LightScattering_Ray_Mie_Ray2_Mie2_[2i];
    let _e1142 = ((_e1104.y * _e1139) + _e1088.y);
    let _e1150 = vec4<f32>((_e1142 * _e1097), _e1135.y, _e1135.z, _e1135.w);
    let _e1155 = select(_e1150.zw, _e119.xy, (_e1104.zz == vec2<f32>(0f, 0f)));
    let _e1158 = vs_VertexShaderConstants.mrgTexcoordOffset;
    let _e1160 = (_e1155.xy + _e1158.xy);
    let _e1165 = vec4<f32>(_e1160.x, _e1160.y, vec4<f32>(0f, 0f, 0f, 0f).z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1170 = vec4<f32>(_e1165.x, _e1165.y, 0f, _e1165.w);
    let _e1178 = vs_VertexShaderConstants.g_LightScatteringColor;
    let _e1180 = (_e1150.xxy * _e1178.xyz);
    let _e1185 = vec4<f32>(_e1180.x, _e1180.y, _e1180.z, _e1155.y);
    let _e1188 = vs_VertexShaderConstants.mrgFresnelParam[1i];
    let _e1194 = vec4<f32>(_e1112.x, _e1112.y, _e1112.z, (_e1188 * (((_e1088.x * _e1069.x) * (1f - _e732)) + _e732)));
    let _e1207 = vs_VertexShaderConstants.g_LightScatteringFarNearScale;
    let _e1209 = ((vec4<f32>(_e1185.x, _e1185.y, _e1185.z, (_e1180.z * _e1142)).xyw * _e1069.yyy).xyz * _e1207.www);
    let _e1214 = vec4<f32>(_e1209.x, _e1209.y, _e1209.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e1221 = vs_SharedConstants.g_HalfPixelOffset;
    let _e1225 = (_e729.xy + (_e1221 * _e729.w));
    vs_global = vec4<f32>(_e1225.x, _e1225.y, _e729.z, _e729.w);
    vs_out_u002e_var_u002e_TEXCOORD0_ = vec4<f32>(_e1170.x, _e1170.y, _e1170.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD1_ = vec4<f32>(_e1019.x, _e1019.y, _e1019.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD2_ = vec4<f32>(_e1120.x, _e1120.y, _e1120.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD3_ = max(_e1194, _e1194);
    vs_out_u002e_var_u002e_TEXCOORD4_ = vec4<f32>(_e1049.x, _e1049.y, _e1049.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD5_ = vec4<f32>(_e1214.x, _e1214.y, _e1214.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD6_ = vec4<f32>(_e1034.x, _e1034.y, _e1034.z, 0f);
    vs_out_u002e_var_u002e_TEXCOORD7_ = vec4<f32>(_e995.x, _e995.y, _e995.z, max(_e996, _e996));
    vs_out_u002e_var_u002e_TEXCOORD8_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD9_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD10_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD11_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD12_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD13_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD14_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_TEXCOORD15_ = vec4<f32>(0f, 0f, 0f, 0f);
    vs_out_u002e_var_u002e_COLOR0_ = max(_e92, _e92);
    vs_out_u002e_var_u002e_COLOR1_ = vec4<f32>(_e989.x, _e989.y, _e1005.x, _e1005.y);
    return;
}

@vertex 
fn vertex(@location(0) in_u002e_var_u002e_POSITION0_: vec4<f32>, @location(4) in_u002e_var_u002e_TEXCOORD0_: vec4<f32>, @location(5) in_u002e_var_u002e_TEXCOORD1_: vec4<f32>, @location(6) in_u002e_var_u002e_TEXCOORD2_: vec4<f32>, @location(7) in_u002e_var_u002e_TEXCOORD3_: vec4<f32>, @location(8) in_u002e_var_u002e_COLOR0_: vec4<f32>, @location(1) in_u002e_var_u002e_NORMAL0_: vec4<u32>, @location(2) in_u002e_var_u002e_TANGENT0_: vec4<u32>, @location(3) in_u002e_var_u002e_BINORMAL0_: vec4<u32>, @location(10) in_u002e_var_u002e_BLENDWEIGHT0_: vec4<f32>, @location(9) in_u002e_var_u002e_BLENDINDICES0_: vec4<u32>) -> vs_VertexOutput {
    vs_in_u002e_var_u002e_POSITION0_1 = in_u002e_var_u002e_POSITION0_;
    vs_in_u002e_var_u002e_TEXCOORD0_1 = in_u002e_var_u002e_TEXCOORD0_;
    vs_in_u002e_var_u002e_TEXCOORD1_1 = in_u002e_var_u002e_TEXCOORD1_;
    vs_in_u002e_var_u002e_TEXCOORD2_1 = in_u002e_var_u002e_TEXCOORD2_;
    vs_in_u002e_var_u002e_TEXCOORD3_1 = in_u002e_var_u002e_TEXCOORD3_;
    vs_in_u002e_var_u002e_COLOR0_1 = in_u002e_var_u002e_COLOR0_;
    vs_in_u002e_var_u002e_NORMAL0_1 = in_u002e_var_u002e_NORMAL0_;
    vs_in_u002e_var_u002e_TANGENT0_1 = in_u002e_var_u002e_TANGENT0_;
    vs_in_u002e_var_u002e_BINORMAL0_1 = in_u002e_var_u002e_BINORMAL0_;
    vs_in_u002e_var_u002e_BLENDWEIGHT0_1 = in_u002e_var_u002e_BLENDWEIGHT0_;
    vs_in_u002e_var_u002e_BLENDINDICES0_1 = in_u002e_var_u002e_BLENDINDICES0_;
    vs_main_1();
    vs_global.z = 0.1; // Bevy infinite reverse-Z: depth = near / w
    let _e44 = vs_global;
    let _e45 = vs_out_u002e_var_u002e_TEXCOORD0_;
    let _e46 = vs_out_u002e_var_u002e_TEXCOORD1_;
    let _e47 = vs_out_u002e_var_u002e_TEXCOORD2_;
    let _e48 = vs_out_u002e_var_u002e_TEXCOORD3_;
    let _e49 = vs_out_u002e_var_u002e_TEXCOORD4_;
    let _e50 = vs_out_u002e_var_u002e_TEXCOORD5_;
    let _e51 = vs_out_u002e_var_u002e_TEXCOORD6_;
    let _e52 = vs_out_u002e_var_u002e_TEXCOORD7_;
    let _e53 = vs_out_u002e_var_u002e_TEXCOORD8_;
    let _e54 = vs_out_u002e_var_u002e_TEXCOORD9_;
    let _e55 = vs_out_u002e_var_u002e_TEXCOORD10_;
    let _e56 = vs_out_u002e_var_u002e_TEXCOORD11_;
    let _e57 = vs_out_u002e_var_u002e_TEXCOORD12_;
    let _e58 = vs_out_u002e_var_u002e_TEXCOORD13_;
    let _e59 = vs_out_u002e_var_u002e_TEXCOORD14_;
    let _e60 = vs_out_u002e_var_u002e_TEXCOORD15_;
    let _e61 = vs_out_u002e_var_u002e_COLOR0_;
    let _e62 = vs_out_u002e_var_u002e_COLOR1_;
    return vs_VertexOutput(_e44, _e45, _e46, _e47, _e48, _e49, _e50, _e51, _e52, _e61, _e62);
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
    mrgFresnelParam: vec4<f32>,
    mrgLuminanceRange: vec4<f32>,
    g_ShadowMapParams: vec4<f32>,
    g_aLightField: array<vec4<f32>, 6>,
    g_SonicEyeHighLightColor: vec4<f32>,
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
@group(#{MATERIAL_BIND_GROUP}) @binding(32) 
var ps_sampNrm_t2d: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(35) 
var ps_sampNrm_s: sampler;
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
var<private> ps_in_u002e_var_u002e_TEXCOORD6_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_TEXCOORD7_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_COLOR0_1: vec4<f32>;
var<private> ps_in_u002e_var_u002e_COLOR1_1: vec4<f32>;
var<private> ps_out_u002e_var_u002e_SV_Target0_: vec4<f32>;

fn ps_main_1() {
    var local: array<vec3<f32>, 2>;
    var phi_563_: vec4<f32>;
    var phi_566_: vec4<f32>;
    var phi_568_: vec4<f32>;
    var phi_570_: vec4<f32>;
    var phi_572_: i32;
    var local_1: vec4<f32>;
    var local_2: vec4<f32>;
    var local_3: vec4<f32>;
    var local_4: vec4<f32>;
    var local_5: vec4<f32>;
    var local_6: vec4<f32>;

    let _e108 = ps_in_u002e_var_u002e_TEXCOORD0_1;
    let _e109 = ps_in_u002e_var_u002e_TEXCOORD1_1;
    let _e110 = ps_in_u002e_var_u002e_TEXCOORD2_1;
    let _e111 = ps_in_u002e_var_u002e_TEXCOORD3_1;
    let _e112 = ps_in_u002e_var_u002e_TEXCOORD4_1;
    let _e113 = ps_in_u002e_var_u002e_TEXCOORD5_1;
    let _e114 = ps_in_u002e_var_u002e_TEXCOORD6_1;
    let _e115 = ps_in_u002e_var_u002e_TEXCOORD7_1;
    let _e116 = ps_in_u002e_var_u002e_COLOR0_1;
    let _e117 = ps_in_u002e_var_u002e_COLOR1_1;
    local = array<vec3<f32>, 2>();
    let _e118 = _e108.xy;
    let _e119 = textureSample(ps_sampPower_t2d, ps_sampPower_s, _e118);
    let _e120 = textureSample(ps_sampNrm_t2d, ps_sampNrm_s, _e118);
    let _e123 = ps_PixelShaderConstants.g_PowerGlossLevel[1i];
    let _e130 = _e110.zxy;
    let _e132 = _e109.zxy;
    let _e134 = _e115.xyz;
    let _e137 = ps_PixelShaderConstants.g_EyePosition;
    let _e139 = (-(_e134) + _e137.xyz);
    let _e140 = _e111.zxy;
    let _e144 = ((_e120.wyxz * vec4<f32>(2f, 2f, 2f, 2f)) + vec4<f32>(-1f, -1f, -1f, -1f));
    let _e145 = _e139.zxy;
    let _e149 = clamp(inverseSqrt(abs(dot(_e140, _e140))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e154 = vec4<f32>(_e109.x, _e109.y, _e109.z, _e149);
    let _e155 = _e144.yxwz;
    let _e156 = (_e155 * _e155);
    let _e159 = (_e154.www * _e111.xyz);
    let _e167 = vec4<f32>(_e112.x, _e112.y, _e112.z, clamp(inverseSqrt(abs(dot(_e132, _e132))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e170 = (_e167.www * _e154.zxy);
    let _e175 = vec4<f32>(_e170.x, _e170.y, _e170.z, vec4<f32>(0f, 0f, 0f, 0f).w);
    let _e183 = vec4<f32>(_e108.x, _e108.y, _e108.z, clamp(inverseSqrt(abs(dot(_e130, _e130))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e186 = (_e183.www * _e110.xyz);
    let _e192 = vec4<f32>(_e183.x, _e183.y, _e183.z, (0.5f * _e149));
    let _e195 = (_e192.www * _e111.zyx);
    let _e208 = vec4<f32>(_e192.x, _e192.y, clamp(inverseSqrt(abs(dot(_e145, _e145))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e192.w);
    let _e211 = (_e139.zyx * _e208.zzz);
    let _e224 = ((_e170.yxzx * _e144.zzzw) + (_e186.xzyz * _e144.yyyx));
    let _e225 = _e211.zyx;
    let _e227 = ps_PixelShaderConstants.mrgGlobalLight_Direction;
    let _e230 = (_e225 + -(_e227.xyz));
    let _e233 = (vec4<f32>(_e195.x, _e195.y, _e195.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, (_e123 * 10f)).w).xzyw + vec4<f32>(0.5f, 0.5f, 0.5f, 2f));
    let _e235 = _e230.zxy;
    let _e243 = vec4<f32>(sqrt(abs((1f - (_e156.z + _e156.y)))), _e154.y, _e154.z, _e154.w);
    let _e246 = (_e159.xzy * _e243.xxx);
    let _e251 = vec4<f32>(_e246.x, _e246.y, _e246.z, _e243.w);
    let _e259 = vec4<f32>(_e208.x, _e208.y, clamp(inverseSqrt(abs(dot(_e235, _e235))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e208.w);
    let _e267 = vec4<f32>((_e224.w + _e246.y), _e175.y, _e175.z, _e175.w);
    let _e269 = exp2(_e233.w);
    let _e274 = ((_e186.yx * _e144.xx) + _e246.zx);
    let _e279 = vec4<f32>(_e251.x, _e274.x, _e274.y, _e251.w);
    let _e282 = (_e230.zyx * _e259.zzz);
    let _e287 = vec4<f32>(_e230.x, _e282.x, _e282.y, _e282.z);
    let _e288 = max(_e269, _e269);
    let _e290 = _e159.zxy;
    let _e299 = vec4<f32>((_e119.y * _e288), _e287.y, _e287.z, _e287.w);
    let _e304 = ((_e267.zy * _e144.ww) + _e274.xy);
    let _e311 = vec4<f32>(_e267.x, _e304.x, _e304.y, _e267.w).xzy;
    let _e316 = dot(_e311, -(_e227.zxy));
    let _e321 = vec4<f32>(_e299.x, _e299.y, _e316, _e299.w);
    let _e330 = max(vec4<f32>(_e321.x, (_e119.z * _e288), _e321.z, _e321.w).yxz, vec3<f32>(1f, 1f, 0f));
    let _e337 = vec4<f32>(clamp(log2(clamp(dot(_e282.xzy, _e290), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e279.y, _e279.z, _e279.w);
    let _e339 = min(_e330.yx, vec2<f32>(512f, 512f));
    let _e344 = vec4<f32>(_e339.x, _e339.y, vec4<f32>().z, vec4<f32>().w);
    let _e351 = vec4<f32>(_e337.x, clamp(log2(clamp(dot(_e299.ywz, _e311), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e337.z, _e337.w);
    let _e354 = (_e339.xy * _e351.xy);
    let _e357 = clamp((1f / _e117.w), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f);
    let _e366 = vec4<f32>(clamp(_e316, 0f, 1f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e369 = vec4<f32>(_e357, (_e357 + _e357), exp2(_e354.y), exp2(_e354.x));
    let _e372 = (_e369.xxyy * _e117.xyyx);
    let _e373 = _e372.xy;
    let _e374 = textureDimensions(ps_g_ShadowMapSampler_t2d, 0i);
    let _e375 = vec2<f32>(_e374);
    let _e378 = chr_shadow_tap(0i);
    let _e382 = chr_shadow_tap(1i);
    let _e386 = chr_shadow_tap(2i);
    let _e390 = chr_shadow_tap(3i);
    let _e399 = select(fract(((_e373 * _e375) - vec2<f32>(0.5f, 0.5f))), vec2<f32>(0f, 0f), ((bitcast<vec2<u32>>(_e373) & vec2<u32>(2147483647u, 2147483647u)) > vec2<u32>(2139095040u, 2139095040u)));
    let _e404 = vec4<f32>(_e399.x, _e144.y, _e399.y, _e144.w);
    let _e407 = ps_PixelShaderConstants.g_aLightField[5i];
    let _e411 = ps_PixelShaderConstants.g_aLightField[4i];
    let _e414 = (_e407.zyx + -(_e411.zyx));
    let _e419 = vec4<f32>(_e414.x, vec4<f32>(0f, 0f, 0f, 0f).y, _e414.y, _e414.z);
    let _e424 = ps_PixelShaderConstants.g_aLightField[1i];
    let _e428 = ps_PixelShaderConstants.g_aLightField[0i];
    let _e432 = max(0f, 0f);
    let _e435 = ps_PixelShaderConstants.g_aLightField[3i];
    let _e439 = ps_PixelShaderConstants.g_aLightField[2i];
    let _e443 = max(max(_e117.z, _e117.z), _e432);
    let _e452 = vec4<f32>(_e404.x, (1f - _e399.x), _e404.z, _e404.w);
    let _e460 = vec4<f32>(_e366.x, ((-(_e443) * select(0f, 1f, (_e443 >= _e117.w))) + _e443), _e366.z, _e366.w);
    let _e464 = (_e351.www * -(_e227.zyx));
    let _e469 = vec4<f32>(_e464.x, vec4<f32>(0f, 0f, 0f, 0f).y, _e464.y, _e464.z);
    let _e476 = vec4<f32>(_e259.x, _e259.y, sqrt(abs((1f - (_e156.w + _e156.x)))), _e259.w);
    let _e477 = _e159.xyz;
    let _e484 = ((_e435.zyx + -(_e439.zyx)).xzy * _e233.zzz);
    let _e491 = vec4<f32>(_e452.x, _e452.y, _e452.z, (1f - _e399.y));
    let _e494 = ((_e424.zyx + -(_e428.zyx)).xzy * _e233.yyy);
    let _e502 = vec4<f32>(_e419.x, (_e439.y + _e484.z), _e419.z, _e419.w);
    let _e505 = (_e502.zxw * _e233.xxx);
    let _e513 = vec4<f32>((_e428.y + _e494.z), _e502.y, _e502.z, _e502.w);
    let _e516 = (_e491.ywyx * _e491.wxzz);
    let _e524 = vec4<f32>(_e469.x, (_e439.x + _e484.y), _e469.z, _e469.w);
    let _e540 = vec4<f32>((_e428.x + _e494.y), _e524.y, _e524.z, _e524.w);
    let _e549 = vec4<f32>((_e428.z + _e494.x), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e557 = vec4<f32>(_e549.x, _e549.y, (_e505.y + _e411.z), _e549.w);
    let _e566 = (_e477 * _e477).zxy;
    let _e573 = vec4<f32>(dot(_e566, vec4<f32>(_e557.x, (_e439.z + _e484.x), _e557.z, _e557.w).zxy), _e111.y, _e111.z, _e111.w);
    let _e588 = vec4<f32>(_e573.x, dot(_e566, vec4<f32>(_e540.x, _e540.y, (_e411.x + _e505.z), _e540.w).zxy), _e573.z, _e573.w);
    let _e603 = vec4<f32>(_e588.x, _e588.y, dot(_e566, vec4<f32>(_e513.x, _e513.y, (_e411.y + _e505.x), _e513.w).zxy), _e588.w);
    let _e606 = (vec4<f32>(_e460.x, _e460.y, clamp(dot(_e524.xwz, _e140), 0f, 1f), _e460.w).xyz * _e369.zxw);
    let _e608 = (_e372.zw + vec2<f32>(-1f, -1f));
    let _e613 = vec4<f32>(_e476.x, _e476.y, _e608.x, _e608.y);
    let _e618 = abs(_e608.y);
    let _e625 = vec4<f32>(clamp(max(_e618, _e618), 0f, 1f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e628 = dot(_e516.wzxy, select(vec4<f32>(0f, 0f, 0f, 0f), vec4<f32>(1f, 1f, 1f, 1f), (vec4<f32>(_e378.x, _e382.x, _e386.x, _e390.x) >= _e606.yyyy)).wzxy);
    let _e630 = abs(_e608.x);
    let _e640 = ps_PixelShaderConstants.g_ShadowMapParams;
    let _e647 = (clamp((vec4<f32>(_e625.x, clamp(max(_e630, _e630), 0f, 1f), _e625.z, _e625.w).xy + -(_e640.xx)), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f)).yx * _e640.yy);
    let _e657 = vec4<f32>(_e613.x, _e613.y, select(0f, 1f, (-(abs(_e108.x)) > 0f)), _e613.w);
    let _e663 = (((1f - _e628) * max(_e647.y, _e647.x)) + _e628);
    let _e668 = vec4<f32>(_e167.x, _e167.y, _e167.z, _e663);
    let _e671 = ps_PixelShaderConstants.g_Ambient;
    let _e673 = (_e603.yzx * _e671.xyz);
    let _e676 = ps_PixelShaderConstants.mrgGlobalLight_Diffuse[0i];
    let _e682 = vec4<f32>((_e676 * _e663), _e603.y, _e603.z, _e603.w);
    let _e685 = ps_PixelShaderConstants.mrgGlobalLight_Specular;
    let _e687 = (_e668.www * _e685.xyz);
    let _e690 = ps_PixelShaderConstants.mrgGlobalLight_Diffuse[1i];
    let _e696 = vec4<f32>(_e682.x, (_e690 * _e663), _e682.z, _e682.w);
    let _e699 = (_e687.yz * _e606.xx);
    let _e707 = ps_PixelShaderConstants.mrgGlobalLight_Diffuse[2i];
    let _e713 = vec4<f32>(_e696.x, _e696.y, (_e707 * _e663), _e696.w);
    phi_563_ = (_e687.zyxx * _e606.zzzx);
    phi_566_ = vec4<f32>(_e699.x, _e699.y, _e516.z, _e516.w);
    phi_568_ = vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, (0.125f * _e339.x));
    phi_570_ = vec4<f32>(_e657.x, _e657.y, _e657.z, (0.125f * _e339.y));
    phi_572_ = 0i;
    loop {
        let _e735 = phi_563_;
        let _e737 = phi_566_;
        let _e739 = phi_568_;
        let _e741 = phi_570_;
        let _e743 = phi_572_;
        local_1 = _e741;
        local_2 = _e739;
        local_3 = _e737;
        local_4 = _e741;
        local_5 = _e735;
        local_6 = _e735;
        if (_e743 < 4i) {
            continue;
        } else {
            break;
        }
        continuing {
            let _e745 = _e741.zzz;
            let _e746 = (_e745 == vec3<f32>(0f, 0f, 0f));
            let _e748 = ps_PixelShaderConstants.mrgLocalLight0_Position;
            let _e751 = ps_PixelShaderConstants.mrgLocalLight1_Position;
            let _e754 = (_e745 + vec3<f32>(-2f, -3f, -4f));
            let _e758 = ps_PixelShaderConstants.mrgLocalLight0_Range;
            let _e761 = ps_PixelShaderConstants.mrgLocalLight1_Range;
            let _e765 = ps_PixelShaderConstants.mrgLocalLight0_Color;
            let _e768 = ps_PixelShaderConstants.mrgLocalLight1_Color;
            let _e772 = (_e754.xxx == vec3<f32>(0f, 0f, 0f));
            let _e774 = ps_PixelShaderConstants.mrgLocalLight2_Color;
            let _e781 = ps_PixelShaderConstants.mrgLocalLight2_Range;
            let _e786 = ps_PixelShaderConstants.mrgLocalLight2_Position;
            let _e791 = (_e754.yyy == vec3<f32>(0f, 0f, 0f));
            let _e793 = ps_PixelShaderConstants.mrgLocalLight3_Position;
            let _e800 = ps_PixelShaderConstants.mrgLocalLight3_Range;
            let _e805 = ps_PixelShaderConstants.mrgLocalLight3_Color;
            let _e810 = (_e754.zzz == vec3<f32>(0f, 0f, 0f));
            let _e812 = ps_PixelShaderConstants.mrgLocalLight4_Color;
            let _e819 = ps_PixelShaderConstants.mrgLocalLight4_Range;
            let _e822 = select(select(select(select(_e761.zw, _e758.zw, (_e741.zz == vec2<f32>(0f, 0f))).xy, _e781.zw, (_e754.xx == vec2<f32>(0f, 0f))).xy, _e800.zw, (_e754.yy == vec2<f32>(0f, 0f))).xy, _e819.zw, (_e754.zz == vec2<f32>(0f, 0f)));
            let _e827 = vec4<f32>(_e737.x, _e737.y, _e822.x, _e822.y);
            let _e829 = ps_PixelShaderConstants.mrgLocalLight4_Position;
            let _e832 = select(select(select(select(_e751.xyz, _e748.xyz, _e746).xyz, _e786.xyz, _e772).xyz, _e793.xyz, _e791).xyz, _e829.xyz, _e810);
            let _e837 = vec4<f32>(_e832.x, _e832.y, _e832.z, _e739.w);
            let _e840 = (-(_e832.xyz) + _e134);
            let _e841 = _e840.zxy;
            let _e845 = -(_e822.x);
            let _e847 = abs(dot(_e841, _e841));
            let _e856 = vec4<f32>(clamp(inverseSqrt(_e847), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e837.y, _e837.z, _e837.w);
            let _e859 = (_e840.xyz * _e856.xxx);
            let _e861 = -(_e859.zxy);
            let _e868 = vec4<f32>(_e827.x, _e827.y, clamp(dot(_e861, _e311), 0f, 1f), _e827.w);
            let _e871 = (-(_e859.xyz) + _e225);
            let _e872 = _e871.zxy;
            let _e880 = vec4<f32>(_e868.x, _e868.y, _e868.z, clamp(dot(_e290, _e861), 0f, 1f));
            let _e888 = vec4<f32>(clamp(inverseSqrt(abs(dot(_e872, _e872))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e856.y, _e856.z, _e856.w);
            let _e892 = (_e871.zyx * _e888.xxx).xzy;
            let _e905 = vec4<f32>(clamp(log2(clamp(dot(_e892, _e290), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e888.y, _e888.z, _e888.w);
            let _e914 = vec4<f32>(_e905.x, clamp(log2(clamp(dot(_e892, _e311), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e905.z, _e905.w);
            let _e917 = (_e339.yx * _e914.yx);
            let _e922 = vec4<f32>(_e917.x, _e917.y, _e914.z, _e914.w);
            let _e928 = vec4<f32>(_e922.x, _e922.y, (1f - clamp(((_e845 + sqrt(_e847)) * clamp((1f / (_e822.y + _e845)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f)), 0f, 1f)), _e922.w);
            let _e935 = vec4<f32>(exp2(_e917.x), _e928.y, _e928.z, _e928.w);
            let _e938 = (_e935.zzz * select(select(select(select(_e768.xyz, _e765.xyz, _e746).xyz, _e774.xyz, _e772).xyz, _e805.xyz, _e791).xyz, _e812.xyz, _e810).xyz);
            let _e945 = vec4<f32>(_e935.x, exp2(_e917.y), _e935.z, _e935.w);
            let _e948 = (_e880.zw * _e945.xy);
            let _e965 = ((_e938.zyx * _e948.yyy) + _e735.xyz);
            let _e970 = vec4<f32>(_e965.x, _e965.y, _e965.z, _e735.w);
            let _e975 = ((_e938.yz * _e948.xx) + _e880.xy);
            phi_563_ = vec4<f32>(_e970.x, _e970.y, _e970.z, ((_e938.x * _e948.x) + _e735.w));
            phi_566_ = vec4<f32>(_e975.x, _e975.y, _e880.z, _e880.w);
            phi_568_ = vec4<f32>(_e948.x, _e948.y, _e945.z, _e945.w);
            phi_570_ = vec4<f32>(_e741.x, _e741.y, (1f + _e741.z), _e741.w);
            phi_572_ = (_e743 + 1i);
        }
    }
    let _e992 = _e211.xzy;
    let _e994 = dot(-(_e992), _e290);
    let _e997 = local_1;
    let _e1002 = vec4<f32>(_e997.x, _e997.y, (_e994 + _e994), _e997.w);
    local[u32()] = ((-(_e477) * _e1002.zzz) + -(_e225)).xyz;
    let _e1022 = local[u32(_e432)];
    let _e1023 = textureSample(ps_sampEnv_tcube, ps_sampEnv_s, _e1022);
    let _e1025 = textureSample(ps_sampDif_t2d, ps_sampDif_s, _e1002.xy);
    let _e1028 = ps_PixelShaderConstants.mrgEyeLight_Attribute[1i];
    let _e1031 = ps_PixelShaderConstants.mrgEyeLight_Attribute[2i];
    let _e1032 = -(_e1031);
    let _e1036 = ps_PixelShaderConstants.mrgEyeLight_Range[3i];
    let _e1039 = ps_PixelShaderConstants.mrgEyeLight_Range[2i];
    let _e1047 = vec4<f32>(vec4<f32>().x, (_e269 * _e119.x), vec4<f32>().z, vec4<f32>().w);
    let _e1051 = ps_PixelShaderConstants.mrgFresnelParam[0i];
    let _e1052 = -(_e1051);
    let _e1055 = ps_PixelShaderConstants.g_SonicEyeHighLightColor;
    let _e1058 = ps_PixelShaderConstants.g_Specular;
    let _e1066 = vec4<f32>(_e344.x, _e344.y, (1f + max(_e1052, _e1052)), _e344.w);
    let _e1069 = ps_PixelShaderConstants.mrgEyeLight_Attribute[0i];
    let _e1074 = ps_PixelShaderConstants.g_Specular[0i];
    let _e1076 = local_2;
    let _e1083 = vec4<f32>((_e1074 * _e1076.w), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1088 = ps_PixelShaderConstants.g_Diffuse[3i];
    let _e1092 = ps_PixelShaderConstants.g_Specular[1i];
    let _e1098 = vec4<f32>(_e1083.x, (_e1092 * _e1076.w), _e1083.z, _e1083.w);
    let _e1107 = ps_PixelShaderConstants.g_EyePosition[0i];
    let _e1115 = vec4<f32>((-(_e1107) - -(max(_e115.x, _e115.x))), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1116 = _e114.zxy;
    let _e1120 = ps_PixelShaderConstants.g_EyePosition[1i];
    let _e1128 = vec4<f32>(_e1115.x, (-(_e1120) - -(max(_e115.y, _e115.y))), _e1115.z, _e1115.w);
    let _e1129 = _e1023.xyz;
    let _e1131 = ps_PixelShaderConstants.mrgLuminanceRange;
    let _e1136 = ps_PixelShaderConstants.g_EyePosition[2i];
    let _e1144 = vec4<f32>(_e1128.x, _e1128.y, (-(_e1136) - -(max(_e115.z, _e115.z))), _e1128.w);
    let _e1149 = (1f - clamp(dot(_e992, _e311), 0f, 1f));
    let _e1151 = local_3;
    let _e1156 = vec4<f32>(_e1151.x, _e1151.y, _e1149, _e1151.w);
    let _e1165 = _e1144.zxy;
    let _e1169 = ps_PixelShaderConstants.g_OpacityReflectionRefractionSpectype[0i];
    let _e1171 = min((_e1169 * (_e1025.w * _e1088)), 4f);
    let _e1182 = ((-(vec4<f32>(clamp(inverseSqrt(abs(dot(_e1116, _e1116))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w).xxx) * _e114.xzy) + _e211.zxy);
    let _e1187 = vec4<f32>(_e1182.x, _e1182.y, _e1182.z, vec4<f32>().w);
    let _e1191 = abs(dot(_e1165, _e1165));
    let _e1193 = _e1182.yxz;
    let _e1201 = vec4<f32>(_e668.x, _e668.y, _e668.z, clamp(inverseSqrt(_e1191), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e1204 = (_e1144.xyz * _e1201.www);
    let _e1207 = ps_PixelShaderConstants.g_Specular[2i];
    let _e1214 = _e1204.zxy;
    let _e1215 = -(_e1214);
    let _e1222 = vec4<f32>(_e1187.x, _e1187.y, _e1187.z, clamp(dot(_e290, _e1215), 0f, 1f));
    let _e1224 = local_4;
    let _e1226 = (_e1092 * _e1224.w);
    let _e1227 = dot(_e1215, _e311);
    let _e1229 = (_e1207 * _e1224.w);
    let _e1237 = ps_PixelShaderConstants.mrgEyeLight_Specular[0i];
    let _e1240 = ps_PixelShaderConstants.mrgEyeLight_Specular[3i];
    let _e1246 = vec4<f32>((_e1237 * _e1240), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1250 = (-(_e1204.zyx) + _e211.xyz);
    let _e1258 = vec4<f32>(_e1246.x, _e1246.y, (-(_e1039) - -(sqrt(_e1191))), _e1246.w);
    let _e1260 = ps_PixelShaderConstants.g_EyeDirection;
    let _e1280 = vec4<f32>(_e1258.x, (_e1032 - -(dot(_e1214, _e1260.zxy))), _e1258.z, _e1258.w);
    let _e1281 = _e1250.xzy;
    let _e1289 = vec4<f32>(clamp((1f / (_e1028 + _e1032)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e1222.y, _e1222.z, _e1222.w);
    let _e1291 = max(vec4<f32>(_e1047.x, _e1047.y, _e1227, _e1047.w).yz, vec2<f32>(1f, 0f));
    let _e1299 = vec4<f32>(_e1289.x, clamp(inverseSqrt(abs(dot(_e1281, _e1281))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e1289.z, _e1289.w);
    let _e1309 = vec4<f32>(_e1299.x, clamp((1f / (_e1036 - _e1039)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e1299.z, _e1299.w);
    let _e1313 = clamp((_e1280.yz * _e1309.xy), vec2<f32>(0f, 0f), vec2<f32>(1f, 1f));
    let _e1316 = ps_PixelShaderConstants.mrgEyeLight_Specular[2i];
    let _e1322 = vec4<f32>(_e1280.x, (_e1316 * _e1240), _e1280.z, _e1280.w);
    let _e1323 = (_e1250.xyz * _e1299.yyy).xzy;
    let _e1328 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[0i];
    let _e1331 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[3i];
    let _e1334 = (vec2<f32>(1f, 3f) - _e1313.yx);
    let _e1339 = vec4<f32>(_e1334.x, _e1309.y, _e1334.y, _e1309.w);
    let _e1340 = (_e1149 * _e1149);
    let _e1343 = select(1f, _e1334.x, (_e1069 > 0f));
    let _e1349 = min(_e1291.x, 512f);
    let _e1352 = vec4<f32>(_e1349, (_e1328 * _e1331), clamp(log2(clamp(dot(_e1323, _e290), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), clamp(log2(clamp(dot(_e1323, _e311), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f));
    let _e1355 = (_e1066.yx * _e1352.wz);
    let _e1360 = vec4<f32>(_e1355.x, _e1066.y, _e1066.z, _e1355.y);
    let _e1366 = vec4<f32>(_e1339.x, (_e1340 * _e1340), _e1339.z, _e1339.w);
    let _e1372 = vec4<f32>((_e1349 * 0.125f), _e1366.y, _e1366.z, _e1366.w);
    let _e1379 = vec4<f32>(_e1156.x, _e1156.y, _e1156.z, exp2(_e1355.y));
    let _e1383 = (_e1334.y + -(_e1313.x));
    let _e1400 = vec4<f32>((clamp(_e1227, 0f, 1f) * exp2(_e1355.x)), vec4<f32>().y, vec4<f32>().z, vec4<f32>().w);
    let _e1403 = (_e1372.wy * _e1379.wz);
    let _e1408 = vec4<f32>(_e1400.x, _e1403.x, _e1400.z, _e1403.y);
    let _e1414 = vec4<f32>(_e1408.x, _e1408.y, (_e1313.x * _e1313.x), _e1408.w);
    let _e1417 = (_e1414.wz * vec4<f32>(_e1360.x, _e1360.y, _e1360.z, _e1383).zw);
    let _e1422 = vec4<f32>(_e1372.x, _e1417.x, _e1372.z, _e1417.y);
    let _e1425 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[1i];
    let _e1431 = vec4<f32>(_e1352.x, _e1352.y, (_e1425 * _e1331), _e1352.w);
    let _e1434 = (((((_e1129 * _e1131.xxx).xyz * _e1023.www) + _e1129).xzy * _e116.www).xzy * (vec4<f32>(vec4<f32>().x, vec4<f32>().y, (0.7f * _e663), vec4<f32>().w).xyz + vec3<f32>(1.5f, 1.5f, 0.3f)).zzz);
    let _e1439 = vec4<f32>(_e1434.x, _e1434.y, _e1434.z, vec4<f32>().w);
    let _e1446 = ps_PixelShaderConstants.mrgEyeLight_Attribute[3i];
    let _e1450 = ps_PixelShaderConstants.mrgEyeLight_Diffuse[2i];
    let _e1458 = (_e1417.x + _e1051);
    let _e1463 = vec4<f32>(_e1422.x, _e1458, _e1422.z, _e1422.w);
    let _e1467 = select(1f, clamp(exp2((clamp(log2(abs(_e1417.y)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f) * _e1446)), 0f, 1f), (select(0f, 1f, (_e1069 > 1f)) > 0f));
    let _e1471 = vec4<f32>(_e1467, (_e1343 * _e1467), clamp(log2(clamp(dot((_e1222.xzy * vec4<f32>(vec4<f32>().x, clamp(inverseSqrt(abs(dot(_e1193, _e1193))), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), vec4<f32>().z, vec4<f32>().w).yyy).zxy, ((_e477 * _e476.zzz) + _e224.xzy).zxy), 0f, 1f)), -340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f), _e1383);
    let _e1473 = (vec4<f32>(_e1431.x, _e1431.y, _e1431.z, (_e1450 * _e1331)) * _e1471.zyyy);
    let _e1476 = ps_PixelShaderConstants.mrgEyeLight_Specular[1i];
    let _e1485 = (vec4<f32>(_e1322.x, _e1322.y, (_e1476 * _e1240), _e1322.w).xzy * _e1471.xyy);
    let _e1490 = vec4<f32>(_e1471.x, _e1485.x, _e1485.y, _e1485.z);
    let _e1498 = vec4<f32>((_e1485.x * _e1343), _e1490.y, _e1490.z, _e1490.w);
    let _e1505 = vec4<f32>(_e1439.x, _e1439.y, _e1439.z, exp2(_e1473.x));
    let _e1510 = ((_e1473.yzw * _e1291.yyy) + (_e713.xyz * _e330.zzz).xyz);
    let _e1515 = ((_e1498.zw * _e1414.xx) + _e1379.xy);
    let _e1519 = ps_PixelShaderConstants.g_Diffuse[2i];
    let _e1527 = vec4<f32>(vec4<f32>().x, vec4<f32>().y, ((_e1510.z * _e1519) + _e673.z), vec4<f32>().w);
    let _e1529 = ps_PixelShaderConstants.g_Diffuse;
    let _e1533 = ((_e1510.xy * _e1529.xy) + _e673.xy);
    let _e1538 = vec4<f32>(_e1463.x, _e1463.y, _e1533.x, _e1533.y);
    let _e1547 = local_5;
    let _e1551 = ((_e1498.xwzx * _e1414.yyyx) + _e1547.zxyw);
    let _e1568 = vec4<f32>(((_e1074 * _e1224.w) * _e1551.w), _e1538.y, _e1538.z, _e1538.w);
    let _e1575 = vec4<f32>((_e1515.x * max(_e1226, _e1226)), _e1527.y, _e1527.z, _e1527.w);
    let _e1578 = (_e1568.xzw * _e1025.xxy);
    let _e1586 = ps_PixelShaderConstants.mrgFresnelParam;
    let _e1588 = (vec4<f32>(_e1578.x, _e1568.y, _e1578.y, _e1578.z).xy * _e1586.yy);
    let _e1598 = (vec4<f32>(_e1575.x, (_e1515.y * max(_e1229, _e1229)), _e1575.z, _e1575.w).xyz * _e1025.yzz);
    let _e1603 = (_e1598.xy * _e1588.yy);
    let _e1608 = vec4<f32>(vec4<f32>().x, _e1603.x, _e1603.y, vec4<f32>().w);
    let _e1619 = ((((vec4<f32>(_e1098.x, _e1098.y, (_e1207 * _e1076.w), _e1098.w).xyz * _e1551.xzy) + ((((_e1055.xyz * _e1058.xyz).xzy * _e1505.www).xyz * _e1463.xxx) + _e1505.xzy).xzy).xyz * _e713.www) + vec4<f32>((_e1458 * max(_e1588.x, _e1588.x)), _e1608.y, _e1608.z, _e1608.w).xyz);
    let _e1624 = vec4<f32>(_e1619.x, _e1619.y, _e1619.z, _e1598.z);
    let _e1632 = vec4<f32>(_e1624.x, _e1624.y, (_e1619.z + _e1598.z), _e1624.w);
    let _e1635 = (_e1632.xy + _e1578.yz);
    let _e1650 = ((min((vec4<f32>(_e1635.x, _e1635.y, _e1632.z, _e1632.w).xyz * _e116.xyz).xyz, vec3<f32>(4f, 4f, 4f)).xyz * _e1201.xyz) + _e113.xyz);
    let _e1657 = ps_SharedConstants.g_SpecConstantsValue;
    if ((_e1657 & 2u) != 0u) {
        let _e1661 = ps_SharedConstants.g_AlphaThreshold;
        if ((_e1171 - _e1661) < 0f) {
            discard;
        }
    }
    ps_out_u002e_var_u002e_SV_Target0_ = vec4<f32>(_e1650.x, _e1650.y, _e1650.z, vec4<f32>(vec4<f32>().x, vec4<f32>().y, vec4<f32>().z, _e1171).w);
    return;
}

@fragment 
fn fragment(@builtin(position) chr_frag: vec4<f32>, @location(0) in_u002e_var_u002e_TEXCOORD0_: vec4<f32>, @location(1) in_u002e_var_u002e_TEXCOORD1_: vec4<f32>, @location(2) in_u002e_var_u002e_TEXCOORD2_: vec4<f32>, @location(3) in_u002e_var_u002e_TEXCOORD3_: vec4<f32>, @location(4) in_u002e_var_u002e_TEXCOORD4_: vec4<f32>, @location(5) in_u002e_var_u002e_TEXCOORD5_: vec4<f32>, @location(6) in_u002e_var_u002e_TEXCOORD6_: vec4<f32>, @location(7) in_u002e_var_u002e_TEXCOORD7_: vec4<f32>, @location(16) in_u002e_var_u002e_COLOR0_: vec4<f32>, @location(17) in_u002e_var_u002e_COLOR1_: vec4<f32>) -> @location(0) vec4<f32> {
    chr_shadow = chr_fetch_shadow(chr_frag);
    ps_in_u002e_var_u002e_TEXCOORD0_1 = in_u002e_var_u002e_TEXCOORD0_;
    ps_in_u002e_var_u002e_TEXCOORD1_1 = in_u002e_var_u002e_TEXCOORD1_;
    ps_in_u002e_var_u002e_TEXCOORD2_1 = in_u002e_var_u002e_TEXCOORD2_;
    ps_in_u002e_var_u002e_TEXCOORD3_1 = in_u002e_var_u002e_TEXCOORD3_;
    ps_in_u002e_var_u002e_TEXCOORD4_1 = in_u002e_var_u002e_TEXCOORD4_;
    ps_in_u002e_var_u002e_TEXCOORD5_1 = in_u002e_var_u002e_TEXCOORD5_;
    ps_in_u002e_var_u002e_TEXCOORD6_1 = in_u002e_var_u002e_TEXCOORD6_;
    ps_in_u002e_var_u002e_TEXCOORD7_1 = in_u002e_var_u002e_TEXCOORD7_;
    ps_in_u002e_var_u002e_COLOR0_1 = in_u002e_var_u002e_COLOR0_;
    ps_in_u002e_var_u002e_COLOR1_1 = in_u002e_var_u002e_COLOR1_;
    ps_main_1();
    let _e21 = ps_out_u002e_var_u002e_SV_Target0_;
    return _e21;
}

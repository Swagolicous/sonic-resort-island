//! Mirage (Hedgehog Engine) resources -> Bevy meshes, materials and images.

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    image::{
        CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType,
    },
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use unleashed_assets::{
    mirage::{
        material::{Material as MirageMaterial, parse_material, parse_texset, parse_texture},
        mesh::{MeshSlot, Submesh},
    },
    resources::ResourceDb,
};

/// How a material should be shaded beyond what the `.material` says.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shading {
    Lit,
    /// Sky domes: unlit, unfogged, double sided.
    Sky,
}

#[derive(Default)]
pub struct MirageCache {
    images: HashMap<(String, bool), Option<Handle<Image>>>,
    materials: HashMap<(String, Option<MeshSlot>, Shading), Handle<StandardMaterial>>,
    pub failed_images: Vec<String>,
}

/// UV set carrying a material's diffuse map (some terrain uses TEXCOORD2).
pub fn diffuse_channel(db: &ResourceDb, material: &str) -> usize {
    let Some(Ok(mat)) = db.get(&format!("{material}.material")).map(parse_material) else {
        return 0;
    };
    texture_set(db, &mat).diffuse_channel
}

pub fn submesh_mesh(sub: &Submesh, vertex_colors: bool) -> Mesh {
    submesh_mesh_uv(sub, vertex_colors, 0)
}

/// Whether a material's vertex colours are a tint (Common shaders) rather
/// than blend weights (Blend shaders).
pub fn tints_with_vertex_color(db: &ResourceDb, material: &str) -> bool {
    match db.get(&format!("{material}.material")).map(parse_material) {
        Some(Ok(mat)) => !mat.shader.starts_with("Blend"),
        _ => true,
    }
}

pub fn submesh_mesh_uv(sub: &Submesh, vertex_colors: bool, uv_channel: usize) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, sub.positions.clone());
    if sub.normals.len() == sub.positions.len() {
        let normals: Vec<[f32; 3]> = sub.normals.iter().map(|n| normalize(*n)).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    let channel = if sub.uvs[uv_channel.min(3)].len() == sub.positions.len() { uv_channel.min(3) } else { 0 };
    let uv0 = if sub.uvs[channel].len() == sub.positions.len() {
        sub.uvs[channel].clone()
    } else {
        vec![[0.0, 0.0]; sub.positions.len()]
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv0);
    if vertex_colors && sub.colors.len() == sub.positions.len() {
        let colors: Vec<[f32; 4]> = sub.colors.iter().map(|c| [c[0], c[1], c[2], 1.0]).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    // Source data is Direct3D clockwise-front; Bevy expects counter-clockwise.
    let indices: Vec<u32> = sub
        .triangles
        .iter()
        .flat_map(|[a, b, c]| [*a, *c, *b])
        .collect();
    mesh.insert_indices(Indices::U32(indices));
    if sub.normals.len() != sub.positions.len() {
        mesh.compute_smooth_normals();
    }
    mesh
}

fn normalize(n: [f32; 3]) -> [f32; 3] {
    let v = Vec3::from(n);
    let len = v.length();
    if len > 1e-6 { (v / len).into() } else { [0.0, 1.0, 0.0] }
}

pub fn load_dds(
    db: &ResourceDb,
    cache: &mut MirageCache,
    images: &mut Assets<Image>,
    name: &str,
    srgb: bool,
) -> Option<Handle<Image>> {
    let key = (name.to_owned(), srgb);
    if let Some(found) = cache.images.get(&key) {
        return found.clone();
    }
    let file = format!("{name}.dds");
    let handle = db.get(&file).and_then(|bytes| {
        let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            address_mode_w: ImageAddressMode::Repeat,
            anisotropy_clamp: 8,
            ..ImageSamplerDescriptor::linear()
        });
        match Image::from_buffer(
            bytes,
            ImageType::Extension("dds"),
            CompressedImageFormats::BC,
            srgb,
            sampler,
            RenderAssetUsages::RENDER_WORLD,
        ) {
            Ok(image) => Some(images.add(image)),
            Err(e) => {
                cache.failed_images.push(format!("{file}: {e}"));
                None
            }
        }
    });
    if handle.is_none() && !db.contains(&file) {
        cache.failed_images.push(format!("{file}: missing"));
    }
    cache.images.insert(key, handle.clone());
    handle
}

pub struct TextureSet {
    pub diffuse: Option<String>,
    pub diffuse_channel: usize,
    pub opacity: Option<String>,
    pub normal: Option<String>,
}

pub fn texture_set(db: &ResourceDb, material: &MirageMaterial) -> TextureSet {
    let mut set = TextureSet {
        diffuse: None,
        diffuse_channel: 0,
        opacity: None,
        normal: None,
    };
    let Some(Ok(names)) = db
        .get(&format!("{}.texset", material.texset))
        .map(parse_texset)
    else {
        return set;
    };
    let mut fallback_diffuse = None;
    for name in names {
        let Some(Ok(tex)) = db.get(&format!("{name}.texture")).map(parse_texture) else {
            continue;
        };
        if tex.kind == "diffuse" && tex.texcoord != 0 && fallback_diffuse.is_none() {
            fallback_diffuse = Some((tex.dds.clone(), tex.texcoord as usize));
        }
        let slot = match (tex.kind.as_str(), tex.texcoord) {
            ("diffuse", 0) => &mut set.diffuse,
            ("opacity", _) => &mut set.opacity,
            ("normal", 0) => &mut set.normal,
            _ => continue,
        };
        if slot.is_none() {
            *slot = Some(tex.dds);
        }
    }
    if set.diffuse.is_none() {
        if let Some((dds, channel)) = fallback_diffuse {
            set.diffuse = Some(dds);
            set.diffuse_channel = channel;
        }
    }
    set
}

pub fn material(
    db: &ResourceDb,
    cache: &mut MirageCache,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    slot: Option<&MeshSlot>,
    shading: Shading,
) -> Handle<StandardMaterial> {
    let key = (name.to_owned(), slot.cloned(), shading);
    if let Some(handle) = cache.materials.get(&key) {
        return handle.clone();
    }

    let parsed = db.get(&format!("{name}.material")).map(parse_material);
    let mut out = StandardMaterial {
        perceptual_roughness: 0.8,
        reflectance: 0.25,
        ..default()
    };
    match parsed {
        Some(Ok(mat)) => {
            let set = texture_set(db, &mat);
            if let Some(diffuse) = &set.diffuse {
                out.base_color_texture = load_dds(db, cache, images, diffuse, true);
            }
            if let Some(color) = mat.param("diffuse") {
                out.base_color = Color::srgb(
                    color[0].clamp(0.0, 1.0),
                    color[1].clamp(0.0, 1.0),
                    color[2].clamp(0.0, 1.0),
                );
            }
            if mat.double_sided {
                out.cull_mode = None;
                out.double_sided = true;
            }
            let threshold = (mat.alpha_threshold as f32 / 255.0).clamp(0.05, 0.95);
            out.alpha_mode = match slot {
                Some(MeshSlot::Transparent) | Some(MeshSlot::Special(_)) => AlphaMode::Blend,
                Some(MeshSlot::PunchThrough) => AlphaMode::Mask(threshold),
                _ => AlphaMode::Opaque,
            };
            if mat.additive {
                out.alpha_mode = AlphaMode::Add;
            }
            if mat.shader.starts_with("Water") {
                out.alpha_mode = AlphaMode::Blend;
                out.base_color = out.base_color.with_alpha(0.75);
                out.perceptual_roughness = 0.1;
                out.reflectance = 0.6;
            }
            if mat.shader.starts_with("Metal") {
                out.perceptual_roughness = 0.4;
            }
            if mat.shader.starts_with("Ring") {
                // Ring_dse gets its gold from specular + an environment map.
                out.base_color_texture = None;
                out.base_color = Color::srgb(1.0, 0.76, 0.18);
                out.emissive = LinearRgba::rgb(0.9, 0.55, 0.05);
                out.perceptual_roughness = 0.25;
                out.reflectance = 0.8;
            }
        }
        Some(Err(e)) => warn!("material {name}: {e:#}"),
        None => warn!("material {name}: missing"),
    }
    if shading == Shading::Sky {
        out.unlit = true;
        out.fog_enabled = false;
        out.cull_mode = None;
        out.double_sided = true;
        out.alpha_mode = AlphaMode::Opaque;
    }
    let handle = materials.add(out);
    cache.materials.insert(key, handle.clone());
    handle
}

/// Mirage instance matrices are row-major with translation in the 4th column.
pub fn row_major_to_mat4(m: &[f32; 16]) -> Mat4 {
    Mat4::from_cols_array(m).transpose()
}

/// Mesh with every attribute the ported Hedgehog shaders read (missing
/// streams are filled with neutral values so one vertex layout fits all).
pub fn hedgehog_mesh(sub: &Submesh, diffuse_channel: usize) -> Mesh {
    use crate::hedgehog::{ATTRIBUTE_BINORMAL, ATTRIBUTE_UV_2};
    let n = sub.positions.len();
    let mut mesh = submesh_mesh_uv(sub, false, diffuse_channel);
    let channel = |c: usize| if sub.uvs[c].len() == n { sub.uvs[c].clone() } else { vec![[0.0, 0.0]; n] };
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, channel(1));
    let uv2 = if sub.uvs[2].len() == n { sub.uvs[2].clone() } else { channel(diffuse_channel.min(3)) };
    mesh.insert_attribute(ATTRIBUTE_UV_2, uv2);
    let colors: Vec<[f32; 4]> = if sub.colors.len() == n { sub.colors.clone() } else { vec![[1.0; 4]; n] };
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    let tangents: Vec<[f32; 4]> = if sub.tangents.len() == n {
        sub.tangents.iter().map(|t| [t[0], t[1], t[2], 1.0]).collect()
    } else {
        vec![[1.0, 0.0, 0.0, 1.0]; n]
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    let binormals = if sub.binormals.len() == n { sub.binormals.clone() } else { vec![[0.0, 0.0, 1.0]; n] };
    mesh.insert_attribute(ATTRIBUTE_BINORMAL, binormals);
    mesh
}

/// Shaders ported to `hedgehog.wgsl`, with the feature bits they use.
fn hedgehog_shader_flags(shader: &str) -> Option<u32> {
    use crate::hedgehog::flags::*;
    let base = shader.split(['[', '@']).next().unwrap_or(shader);
    Some(match base {
        "Common_d" => 0,
        "Common_dn" => HAS_NORMAL,
        "Common_dp" | "Metal_dsp" => HAS_GLOSS,
        "Common_dpn" | "Common_dpne" => HAS_NORMAL | HAS_GLOSS,
        "Blend_dndn" | "Blend_dndn_dn" => HAS_NORMAL | BLEND,
        // Sky_d: raw diffuse, clamped; DummySky: flat material colour.
        "Sky_d" | "DummySky" => UNLIT | NO_SCATTERING,
        _ => return None,
    })
}

/// The material for a ported shader, without per-instance GI (callers set
/// `gi` / `uniform.gi_atlas`). `None` for shaders not ported yet.
pub fn hedgehog_material(
    db: &ResourceDb,
    cache: &mut MirageCache,
    images: &mut Assets<Image>,
    scene: &crate::hedgehog::SceneLighting,
    name: &str,
    slot: Option<&MeshSlot>,
) -> Option<crate::hedgehog::HedgehogMaterial> {
    use crate::hedgehog::{HedgehogMaterial, HedgehogUniform, flags::*};
    let mat = db.get(&format!("{name}.material")).map(parse_material)?.ok()?;
    let mut bits = hedgehog_shader_flags(&mat.shader)?;
    let names = db.get(&format!("{}.texset", mat.texset)).map(parse_texset).and_then(Result::ok).unwrap_or_default();
    let textures: Vec<_> = names
        .iter()
        .filter_map(|n| db.get(&format!("{n}.texture")).map(parse_texture).and_then(Result::ok))
        .collect();
    // Raw (non-sRGB) sampling: the original shaders work on stored values.
    let mut load = |kind: &str, nth: usize| {
        textures
            .iter()
            .filter(|t| t.kind == kind)
            .nth(nth)
            .and_then(|t| load_dds(db, cache, images, &t.dds, false))
    };
    let diffuse = load("diffuse", 0);
    let diffuse2 = load("diffuse", 1);
    let normal = load("normal", 0);
    let normal2 = load("normal", 1);
    let gloss = load("gloss", 0);
    if normal.is_none() {
        bits &= !HAS_NORMAL;
    }
    if gloss.is_none() {
        bits &= !HAS_GLOSS;
    }
    if diffuse2.is_none() {
        bits &= !BLEND;
    }
    let p = |n: &str, d: [f32; 4]| Vec4::from(mat.param(n).unwrap_or(d));
    let mut uniform = HedgehogUniform {
        diffuse: p("diffuse", [1.0; 4]),
        ambient: p("ambient", [1.0; 4]),
        specular: p("specular", [0.0; 4]),
        power_gloss_level: p("power_gloss_level", [50.0, 0.1, 0.1, 0.0]),
        opacity: p("opacity_reflection_refraction_spectype", [1.0, 0.0, 1.0, 0.0]),
        gi_atlas: Vec4::new(1.0, 1.0, 0.0, 0.0),
        ..default()
    };
    scene.apply(&mut uniform);
    if mat.shader.starts_with("Sky_d") {
        // Sky_d ignores g_Diffuse.
        uniform.diffuse = Vec4::ONE;
    }
    let threshold = mat.alpha_threshold as f32 / 255.0;
    let alpha_mode = match slot {
        Some(MeshSlot::Transparent) | Some(MeshSlot::Special(_)) => AlphaMode::Blend,
        Some(MeshSlot::PunchThrough) => AlphaMode::Mask(threshold),
        _ => AlphaMode::Opaque,
    };
    let alpha_test = if matches!(alpha_mode, AlphaMode::Mask(_)) { threshold.max(0.01) } else { 0.0 };
    uniform.flags = Vec4::new(bits as f32, alpha_test, 0.0, 0.0);
    Some(HedgehogMaterial {
        uniform,
        diffuse,
        normal,
        gloss,
        gi: None,
        diffuse2,
        normal2,
        gi_tiles: Handle::default(),
        alpha_mode: if mat.additive { AlphaMode::Add } else { alpha_mode },
        double_sided: mat.double_sided,
    })
}

/// A GI atlas texture (DXT5, raw values, clamped).
pub fn gi_image(images: &mut Assets<Image>, dds: &[u8]) -> Option<Handle<Image>> {
    let sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..ImageSamplerDescriptor::linear()
    });
    Image::from_buffer(dds, ImageType::Extension("dds"), CompressedImageFormats::BC, false, sampler, RenderAssetUsages::RENDER_WORLD)
        .ok()
        .map(|i| images.add(i))
}

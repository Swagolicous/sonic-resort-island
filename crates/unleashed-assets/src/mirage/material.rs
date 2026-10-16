//! Mirage material chain: `.material` -> `.texset` -> `.texture` -> DDS name,
//! plus `.terrain-instanceinfo` placement records.
//!
//! ```text
//! material v1:  shader*, sub_shader*, texset_name*, inline_textures*,
//!               u8 alpha_threshold, u8 no_backface_culling, u8 additive, u8,
//!               u8 param_count, u8 x3, params**
//!   param:      u8 flag, u8, u8 value_count, u8, name*, values* (float4 each)
//! texset:       count, names**  (each a `.texture` resource stem)
//! texture:      dds_name*, u8 texcoord_index, u8 x3, kind*  ("diffuse", "normal", ...)
//! instanceinfo: model_name*, matrix* (row-major 4x4), instance_name*
//! ```

use anyhow::{Result, bail};

use super::reader::MirageReader;

#[derive(Debug, Clone)]
pub struct MaterialParam {
    pub name: String,
    pub values: Vec<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub shader: String,
    pub texset: String,
    pub alpha_threshold: u8,
    pub double_sided: bool,
    pub additive: bool,
    pub params: Vec<MaterialParam>,
}

impl Material {
    pub fn param(&self, name: &str) -> Option<[f32; 4]> {
        self.params
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.values.first().copied())
    }
}

#[derive(Debug, Clone)]
pub struct TextureRef {
    pub dds: String,
    pub kind: String,
    pub texcoord: u8,
}

#[derive(Debug, Clone)]
pub struct TerrainInstance {
    pub model: String,
    pub name: String,
    /// Row-major; translation in the fourth column.
    pub matrix: [f32; 16],
}

pub fn parse_material(bytes: &[u8]) -> Result<Material> {
    let r = MirageReader::new(bytes)?;
    if r.version != 1 && r.version != 3 {
        bail!("unsupported material version {}", r.version);
    }
    let param_count = r.u8(0x14)? as usize;
    let table = r.ptr(0x18)?;
    let mut params = Vec::with_capacity(param_count);
    for i in 0..param_count {
        let p = r.ptr(table + i * 4)?;
        let count = r.u8(p + 2)? as usize;
        let values_at = r.ptr(p + 8)?;
        let values = (0..count)
            .map(|v| {
                let at = values_at + v * 16;
                Ok([r.f32(at)?, r.f32(at + 4)?, r.f32(at + 8)?, r.f32(at + 12)?])
            })
            .collect::<Result<_>>()?;
        params.push(MaterialParam {
            name: r.str_ptr(p + 4)?.to_owned(),
            values,
        });
    }
    Ok(Material {
        shader: r.str_ptr(0)?.to_owned(),
        texset: r.str_ptr(8)?.to_owned(),
        alpha_threshold: r.u8(0x10)?,
        double_sided: r.u8(0x11)? != 0,
        additive: r.u8(0x12)? != 0,
        params,
    })
}

pub fn parse_texset(bytes: &[u8]) -> Result<Vec<String>> {
    let r = MirageReader::new(bytes)?;
    let count = r.u32(0)? as usize;
    let table = r.ptr(4)?;
    (0..count)
        .map(|i| Ok(r.str_ptr(table + i * 4)?.to_owned()))
        .collect()
}

pub fn parse_texture(bytes: &[u8]) -> Result<TextureRef> {
    let r = MirageReader::new(bytes)?;
    Ok(TextureRef {
        dds: r.str_ptr(0)?.to_owned(),
        texcoord: r.u8(4)?,
        kind: r.str_ptr(8)?.to_owned(),
    })
}

pub fn parse_terrain_instance(bytes: &[u8]) -> Result<TerrainInstance> {
    let r = MirageReader::new(bytes)?;
    Ok(TerrainInstance {
        model: r.str_ptr(0)?.to_owned(),
        matrix: r.matrix(r.ptr(4)?)?,
        name: r.str_ptr(8)?.to_owned(),
    })
}

#[derive(Debug, Clone)]
pub struct Light {
    /// 0 = directional (vector is the travel direction), 1 = point.
    pub kind: u32,
    pub vector: [f32; 3],
    pub color: [f32; 3],
}

/// `.light` v1: kind, float3 vector, float3 color.
pub fn parse_light(bytes: &[u8]) -> Result<Light> {
    let r = MirageReader::new(bytes)?;
    Ok(Light {
        kind: r.u32(0)?,
        vector: [r.f32(4)?, r.f32(8)?, r.f32(12)?],
        color: [r.f32(16)?, r.f32(20)?, r.f32(24)?],
    })
}

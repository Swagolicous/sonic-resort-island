//! Mirage mesh resources: `.terrain-model` and `.model` (versions 4 and 5).
//!
//! Layouts (all offsets data-relative, big-endian):
//!
//! ```text
//! terrain-model v5 root: group_count, group_table*, name*
//! model v5 root:         group_count, group_table*, unk_count, unk*,
//!                        bone_count, bone_table*, bone_matrices*, aabb*
//! model v4 root:         one mesh group inline (3 slots + special count/ptr),
//!                        then at +0x20: bone_count, bone_table*, bone_matrices*, aabb*
//! model v2 root:         3 slots inline, then at +0x18 the same bone header
//!
//! mesh group (v5): 3 x (count, table*) for opaque / transparent / punch-through,
//!                  special_count, special_types**, special_counts**, special_tables**
//!
//! submesh: material_name*, index_count, indices*, vertex_count, vertex_size,
//!          vertices*, vertex_decl*, bone_count, bone_palette*, texunit_count, texunits*
//! ```
//!
//! Indices are 16-bit triangle strips restarted by 0xFFFF. Vertex declarations
//! are Xbox 360 `D3DVERTEXELEMENT9` arrays terminated by stream 0xFF.

use anyhow::{Context, Result, bail};

use super::reader::{MirageReader, be_u16, be_u32, half_to_f32};

const DECL_FLOAT1: u32 = 0x002C_83A4;
const DECL_FLOAT2: u32 = 0x002C_23A5;
const DECL_FLOAT3: u32 = 0x002A_23B9;
const DECL_FLOAT4: u32 = 0x001A_23A6;
const DECL_FLOAT16_2: u32 = 0x002C_235F;
const DECL_FLOAT16_4: u32 = 0x001A_2360;
const DECL_HEND3N: u32 = 0x002A_2190;
const DECL_DEC3N: u32 = 0x002A_2187;
const DECL_UBYTE4N: u32 = 0x001A_2086;
const DECL_UBYTE4: u32 = 0x001A_2286;
const DECL_UBYTE4_ALT: u32 = 0x001A_2386;
const DECL_D3DCOLOR: u32 = 0x0018_2886;

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

const USAGE_POSITION: u8 = 0;
const USAGE_BLENDWEIGHT: u8 = 1;
const USAGE_BLENDINDICES: u8 = 2;
const USAGE_NORMAL: u8 = 3;
const USAGE_TEXCOORD: u8 = 5;
const USAGE_TANGENT: u8 = 6;
const USAGE_BINORMAL: u8 = 7;
const USAGE_COLOR: u8 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MeshSlot {
    Opaque,
    Transparent,
    PunchThrough,
    Special(String),
}

#[derive(Debug, Clone, Default)]
pub struct Submesh {
    pub material: String,
    pub slot: Option<MeshSlot>,
    /// Triangle list in the source (Direct3D, clockwise-front) winding.
    pub triangles: Vec<[u32; 3]>,
    /// Length of the source triangle strip (restarts included) and the size of one
    /// source vertex: what the game passes to its draw call for this submesh.
    pub strip_len: usize,
    pub vertex_size: usize,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 3]>,
    pub binormals: Vec<[f32; 3]>,
    /// `uvs[n]` is TEXCOORDn; missing channels are empty.
    pub uvs: [Vec<[f32; 2]>; 4],
    pub colors: Vec<[f32; 4]>,
    /// Indices into `bone_palette`.
    pub joints: Vec<[u8; 4]>,
    pub weights: Vec<[f32; 4]>,
    /// Maps palette slots to model bone indices.
    pub bone_palette: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Bone {
    pub name: String,
    pub parent: i32,
    /// 4x4 matrix exactly as serialized (16 big-endian floats).
    pub matrix: [f32; 16],
}

#[derive(Debug, Clone, Default)]
pub struct MirageModel {
    pub name: Option<String>,
    pub submeshes: Vec<Submesh>,
    pub bones: Vec<Bone>,
}

/// Parses a `.terrain-model` (version 5).
pub fn parse_terrain_model(bytes: &[u8]) -> Result<MirageModel> {
    let r = MirageReader::new(bytes)?;
    if r.version != 5 {
        bail!("unsupported terrain-model version {}", r.version);
    }
    let group_count = r.u32(0)? as usize;
    let table = r.ptr(4)?;
    let name = r.str_ptr(8).ok().map(str::to_owned);
    let mut submeshes = Vec::new();
    for g in 0..group_count {
        let group = r.ptr(table + g * 4)?;
        parse_mesh_group(&r, group, true, &mut submeshes)
            .with_context(|| format!("mesh group {g}"))?;
    }
    Ok(MirageModel {
        name,
        submeshes,
        bones: Vec::new(),
    })
}

/// Parses a `.model` (versions 4 and 5), including its bone list.
pub fn parse_model(bytes: &[u8]) -> Result<MirageModel> {
    let r = MirageReader::new(bytes)?;
    let mut submeshes = Vec::new();
    let bone_header = match r.version {
        5 => {
            let group_count = r.u32(0)? as usize;
            let table = r.ptr(4)?;
            for g in 0..group_count {
                let group = r.ptr(table + g * 4)?;
                parse_mesh_group(&r, group, true, &mut submeshes)
                    .with_context(|| format!("mesh group {g}"))?;
            }
            0x10
        }
        // v4: inline group + special (count, ptr); v2: inline group only.
        4 | 2 => {
            parse_mesh_group(&r, 0, false, &mut submeshes).context("inline mesh group")?;
            if r.version == 4 { 0x20 } else { 0x18 }
        }
        v => bail!("unsupported model version {v}"),
    };

    let bone_count = r.u32(bone_header)? as usize;
    let mut bones = Vec::with_capacity(bone_count);
    if bone_count > 0 {
        let defs = r.ptr(bone_header + 4)?;
        let matrices = r.ptr(bone_header + 8)?;
        for i in 0..bone_count {
            let def = r.ptr(defs + i * 4)?;
            bones.push(Bone {
                parent: r.i32(def)?,
                name: r.string_lossy_at(r.ptr(def + 4)?)?,
                // Some v4 object models carry fewer matrices than bones.
                matrix: r.matrix(matrices + i * 64).unwrap_or(IDENTITY),
            });
        }
    }
    Ok(MirageModel {
        name: None,
        submeshes,
        bones,
    })
}

fn parse_mesh_group(
    r: &MirageReader<'_>,
    group: usize,
    has_special_tables: bool,
    out: &mut Vec<Submesh>,
) -> Result<()> {
    let slots = [MeshSlot::Opaque, MeshSlot::Transparent, MeshSlot::PunchThrough];
    for (i, slot) in slots.into_iter().enumerate() {
        let count = r.u32(group + i * 8)? as usize;
        if count == 0 {
            continue;
        }
        let table = r.ptr(group + i * 8 + 4)?;
        for m in 0..count {
            let mut mesh = parse_submesh(r, r.ptr(table + m * 4)?)
                .with_context(|| format!("{slot:?} submesh {m}"))?;
            mesh.slot = Some(slot.clone());
            out.push(mesh);
        }
    }

    if !has_special_tables {
        return Ok(());
    }
    let special_count = r.u32(group + 24)? as usize;
    if special_count == 0 {
        return Ok(());
    }
    let types = r.ptr(group + 28)?;
    let counts = r.ptr(group + 32)?;
    let tables = r.ptr(group + 36)?;
    for s in 0..special_count {
        let kind = r.str_ptr(types + s * 4)?.to_owned();
        let count = r.u32(r.ptr(counts + s * 4)?)? as usize;
        let table = r.ptr(tables + s * 4)?;
        for m in 0..count {
            let mut mesh = parse_submesh(r, r.ptr(table + m * 4)?)
                .with_context(|| format!("special {kind} submesh {m}"))?;
            mesh.slot = Some(MeshSlot::Special(kind.clone()));
            out.push(mesh);
        }
    }
    Ok(())
}

struct DeclElement {
    offset: usize,
    kind: u32,
    usage: u8,
    usage_index: u8,
}

fn parse_submesh(r: &MirageReader<'_>, at: usize) -> Result<Submesh> {
    let material = r.string_lossy_at(r.ptr(at)?)?;
    let index_count = r.u32(at + 4)? as usize;
    let indices_at = r.ptr(at + 8)?;
    let vertex_count = r.u32(at + 12)? as usize;
    let vertex_size = r.u32(at + 16)? as usize;
    let vertices_at = r.ptr(at + 20)?;
    let decl_at = r.ptr(at + 24)?;
    let bone_count = r.u32(at + 28)? as usize;
    let bones_at = r.ptr(at + 32)?;

    let index_bytes = r.bytes(indices_at, index_count * 2)?;
    let strip: Vec<u16> = (0..index_count)
        .map(|i| be_u16(index_bytes, i * 2))
        .collect::<Result<_>>()?;
    let triangles = strip_to_triangles(&strip, vertex_count)?;

    let mut decl = Vec::new();
    for i in 0..64 {
        let e = decl_at + i * 12;
        let stream = (r.u32(e)? >> 16) as u16;
        if stream == 0xFF {
            break;
        }
        decl.push(DeclElement {
            offset: (r.u32(e)? & 0xFFFF) as usize,
            kind: r.u32(e + 4)?,
            usage: r.u8(e + 9)?,
            usage_index: r.u8(e + 10)?,
        });
    }

    let vertex_bytes = r.bytes(vertices_at, vertex_count * vertex_size)?;
    let mut mesh = Submesh {
        material,
        triangles,
        strip_len: index_count,
        vertex_size,
        bone_palette: r.bytes(bones_at, bone_count)?.to_vec(),
        ..Default::default()
    };

    for element in &decl {
        let read = |v: usize| -> Result<[f32; 4]> {
            decode_element(vertex_bytes, v * vertex_size + element.offset, element.kind)
        };
        let all = |f: &dyn Fn([f32; 4]) -> [f32; 3]| -> Result<Vec<[f32; 3]>> {
            (0..vertex_count).map(|v| read(v).map(f)).collect()
        };
        let xyz = |v: [f32; 4]| [v[0], v[1], v[2]];
        match (element.usage, element.usage_index) {
            (USAGE_POSITION, 0) => mesh.positions = all(&xyz)?,
            (USAGE_NORMAL, 0) => mesh.normals = all(&xyz)?,
            (USAGE_TANGENT, 0) => mesh.tangents = all(&xyz)?,
            (USAGE_BINORMAL, 0) => mesh.binormals = all(&xyz)?,
            (USAGE_TEXCOORD, n @ 0..=3) => {
                mesh.uvs[n as usize] = (0..vertex_count)
                    .map(|v| read(v).map(|t| [t[0], t[1]]))
                    .collect::<Result<_>>()?;
            }
            (USAGE_COLOR, 0) => {
                mesh.colors = (0..vertex_count).map(read).collect::<Result<_>>()?;
            }
            (USAGE_BLENDWEIGHT, 0) => {
                mesh.weights = (0..vertex_count).map(read).collect::<Result<_>>()?;
            }
            (USAGE_BLENDINDICES, 0) => {
                mesh.joints = (0..vertex_count)
                    .map(|v| {
                        let at = v * vertex_size + element.offset;
                        vertex_bytes
                            .get(at..at + 4)
                            .map(|b| [b[0], b[1], b[2], b[3]])
                            .context("blend indices out of bounds")
                    })
                    .collect::<Result<_>>()?;
            }
            _ => {}
        }
    }

    if mesh.positions.len() != vertex_count {
        bail!("submesh {} has no POSITION element", mesh.material);
    }
    Ok(mesh)
}

fn decode_element(bytes: &[u8], at: usize, kind: u32) -> Result<[f32; 4]> {
    let f = |i: usize| -> Result<f32> { Ok(f32::from_bits(be_u32(bytes, at + i * 4)?)) };
    let h = |i: usize| -> Result<f32> { Ok(half_to_f32(be_u16(bytes, at + i * 2)?)) };
    let b = |i: usize| -> Result<f32> {
        bytes
            .get(at + i)
            .map(|v| *v as f32)
            .context("vertex byte out of bounds")
    };
    Ok(match kind {
        DECL_FLOAT1 => [f(0)?, 0.0, 0.0, 1.0],
        DECL_FLOAT2 => [f(0)?, f(1)?, 0.0, 1.0],
        DECL_FLOAT3 => [f(0)?, f(1)?, f(2)?, 1.0],
        DECL_FLOAT4 => [f(0)?, f(1)?, f(2)?, f(3)?],
        DECL_FLOAT16_2 => [h(0)?, h(1)?, 0.0, 1.0],
        DECL_FLOAT16_4 => [h(0)?, h(1)?, h(2)?, h(3)?],
        DECL_HEND3N => {
            let v = be_u32(bytes, at)?;
            [
                signed_bits(v, 0, 11) / 1023.0,
                signed_bits(v, 11, 11) / 1023.0,
                signed_bits(v, 22, 10) / 511.0,
                1.0,
            ]
        }
        DECL_DEC3N => {
            let v = be_u32(bytes, at)?;
            [
                signed_bits(v, 0, 10) / 511.0,
                signed_bits(v, 10, 10) / 511.0,
                signed_bits(v, 20, 10) / 511.0,
                1.0,
            ]
        }
        DECL_UBYTE4N => [b(0)? / 255.0, b(1)? / 255.0, b(2)? / 255.0, b(3)? / 255.0],
        DECL_UBYTE4 | DECL_UBYTE4_ALT => [b(0)?, b(1)?, b(2)?, b(3)?],
        // ARGB packed into a big-endian u32.
        DECL_D3DCOLOR => [b(1)? / 255.0, b(2)? / 255.0, b(3)? / 255.0, b(0)? / 255.0],
        other => bail!("unsupported vertex element type {other:#x}"),
    })
}

fn signed_bits(value: u32, shift: u32, bits: u32) -> f32 {
    let raw = (value >> shift) & ((1 << bits) - 1);
    let sign_bit = 1 << (bits - 1);
    let v = if raw & sign_bit != 0 {
        raw as i32 - (1 << bits)
    } else {
        raw as i32
    };
    (v as f32).max(-((sign_bit - 1) as f32))
}

/// Converts a 0xFFFF-restarted triangle strip into a list, preserving the
/// first triangle's winding and dropping degenerates.
fn strip_to_triangles(strip: &[u16], vertex_count: usize) -> Result<Vec<[u32; 3]>> {
    let mut out = Vec::with_capacity(strip.len());
    let mut run_start = 0;
    for i in 0..strip.len() {
        if strip[i] == 0xFFFF {
            run_start = i + 1;
            continue;
        }
        let k = i - run_start;
        if k < 2 {
            continue;
        }
        let (a, b, c) = (strip[i - 2], strip[i - 1], strip[i]);
        if a == b || b == c || a == c {
            continue;
        }
        for v in [a, b, c] {
            if v as usize >= vertex_count {
                bail!("strip index {v} exceeds vertex count {vertex_count}");
            }
        }
        let tri = if k % 2 == 0 { [a, b, c] } else { [b, a, c] };
        out.push(tri.map(u32::from));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_alternate_winding_and_restart() {
        let tris = strip_to_triangles(&[0, 1, 2, 3, 0xFFFF, 4, 5, 6], 7).unwrap();
        assert_eq!(tris, vec![[0, 1, 2], [2, 1, 3], [4, 5, 6]]);
    }

    #[test]
    fn hend3n_decodes_unit_normal() {
        // First vertex normal of myk01_ter_sk_rock02c_51.
        let v = decode_element(&0x181a_4227u32.to_be_bytes(), 0, DECL_HEND3N).unwrap();
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        assert!((len - 1.0).abs() < 0.01, "{v:?}");
    }
}

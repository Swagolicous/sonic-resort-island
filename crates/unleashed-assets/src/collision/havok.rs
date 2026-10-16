//! Havok 5.5 packfile object graph and static collision extraction.
//!
//! Sonic Unleashed stage collision (`*.phy.hkx`) is a big-endian, 32-bit
//! pointer packfile whose `__data__` section holds:
//!
//! ```text
//! hkpRigidBody -> hkpMoppBvTreeShape -> hkpStorageExtendedMeshShape
//!                                     -> hkpStorageExtendedMeshShapeMeshSubpartStorage[]
//! hkpRigidBody -> hkpBoxShape                          (kill volumes)
//! ```
//!
//! Field offsets below were validated against every rigid body in
//! `myk_col_act01.phy.hkx` (29,195 triangles, all indices in range).

use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::mirage::reader::be_u32;

const MAGIC: [u32; 2] = [0x57E0_E057, 0x10C0_C010];
const SECTION_HEADER: usize = 0x30;

/// hkpProperty key carrying the per-body surface type (matches `_matN` names).
pub const PROPERTY_SURFACE_TYPE: u32 = 0x4B20_2B5A;

pub struct Packfile<'a> {
    bytes: &'a [u8],
    data_start: usize,
    local: HashMap<u32, u32>,
    global: HashMap<u32, (u32, u32)>,
    /// (object offset, class name), sorted by offset.
    objects: Vec<(u32, String)>,
}

impl<'a> Packfile<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if be_u32(bytes, 0)? != MAGIC[0] || be_u32(bytes, 4)? != MAGIC[1] {
            bail!("not a Havok packfile");
        }
        if bytes.get(0x10) != Some(&4) || bytes.get(0x11) != Some(&0) {
            bail!("only 32-bit big-endian Havok packfiles are supported");
        }
        let section_count = be_u32(bytes, 0x14)? as usize;
        let mut sections = Vec::with_capacity(section_count);
        for i in 0..section_count {
            let h = 0x40 + i * SECTION_HEADER;
            let tag = bytes.get(h..h + 19).context("section header out of bounds")?;
            let tag = String::from_utf8_lossy(tag.split(|b| *b == 0).next().unwrap()).into_owned();
            let mut offsets = [0usize; 7];
            for (k, value) in offsets.iter_mut().enumerate() {
                *value = be_u32(bytes, h + 20 + k * 4)? as usize;
            }
            sections.push((tag, offsets));
        }
        let find = |name: &str| {
            sections
                .iter()
                .find(|(tag, _)| tag == name)
                .map(|(_, o)| *o)
                .with_context(|| format!("missing Havok section {name}"))
        };
        let classes = find("__classnames__")?;
        let data = find("__data__")?;
        let [start, local_at, global_at, virtual_at, exports_at, ..] = data;

        let mut class_names = HashMap::new();
        let mut at = classes[0];
        let class_end = classes[0] + classes[1];
        while at + 5 < class_end && bytes[at + 4] == 0x09 {
            let name_at = at + 5;
            let len = bytes[name_at..]
                .iter()
                .position(|b| *b == 0)
                .context("unterminated class name")?;
            let name = String::from_utf8_lossy(&bytes[name_at..name_at + len]).into_owned();
            class_names.insert((name_at - classes[0]) as u32, name);
            at = name_at + len + 1;
        }

        let mut local = HashMap::new();
        for o in (start + local_at..start + global_at).step_by(8) {
            if o + 8 > start + global_at {
                break;
            }
            let src = be_u32(bytes, o)?;
            if src != u32::MAX {
                local.insert(src, be_u32(bytes, o + 4)?);
            }
        }
        let mut global = HashMap::new();
        for o in (start + global_at..start + virtual_at).step_by(12) {
            if o + 12 > start + virtual_at {
                break;
            }
            let src = be_u32(bytes, o)?;
            if src != u32::MAX {
                global.insert(src, (be_u32(bytes, o + 4)?, be_u32(bytes, o + 8)?));
            }
        }
        let mut objects = Vec::new();
        for o in (start + virtual_at..start + exports_at).step_by(12) {
            if o + 12 > start + exports_at {
                break;
            }
            let src = be_u32(bytes, o)?;
            if src == u32::MAX {
                continue;
            }
            let class = be_u32(bytes, o + 8)?;
            let name = class_names
                .get(&class)
                .cloned()
                .unwrap_or_else(|| format!("<class@{class:#x}>"));
            objects.push((src, name));
        }
        objects.sort();

        Ok(Self {
            bytes,
            data_start: start,
            local,
            global,
            objects,
        })
    }

    pub fn objects(&self) -> &[(u32, String)] {
        &self.objects
    }

    pub fn class_of(&self, offset: u32) -> Option<&str> {
        self.objects
            .binary_search_by_key(&offset, |(o, _)| *o)
            .ok()
            .map(|i| self.objects[i].1.as_str())
    }

    pub fn u32(&self, offset: u32) -> Result<u32> {
        be_u32(self.bytes, self.data_start + offset as usize)
    }

    pub fn f32(&self, offset: u32) -> Result<f32> {
        Ok(f32::from_bits(self.u32(offset)?))
    }

    pub fn vec4(&self, offset: u32) -> Result<[f32; 4]> {
        Ok([
            self.f32(offset)?,
            self.f32(offset + 4)?,
            self.f32(offset + 8)?,
            self.f32(offset + 12)?,
        ])
    }

    /// Follows a pointer field, whether it was serialized as a local or a
    /// global fixup.
    pub fn ptr(&self, field: u32) -> Option<u32> {
        self.local
            .get(&field)
            .copied()
            .or_else(|| self.global.get(&field).map(|(_, dst)| *dst))
    }

    pub fn raw(&self, offset: u32, len: usize) -> Result<&'a [u8]> {
        let at = self.data_start + offset as usize;
        self.bytes.get(at..at + len).context("raw range out of bounds")
    }

    pub fn c_str(&self, offset: u32) -> Result<String> {
        let at = self.data_start + offset as usize;
        let tail = self.bytes.get(at..).context("string out of bounds")?;
        let len = tail.iter().position(|b| *b == 0).context("unterminated")?;
        Ok(String::from_utf8_lossy(&tail[..len]).into_owned())
    }
}

#[derive(Debug, Clone)]
pub enum CollisionShape {
    /// World-space triangles in source winding.
    Mesh {
        vertices: Vec<[f32; 3]>,
        triangles: Vec<[u32; 3]>,
    },
    /// Oriented box in world space.
    Box {
        center: [f32; 3],
        half_extents: [f32; 3],
    },
}

#[derive(Debug, Clone)]
pub struct CollisionBody {
    pub name: String,
    pub surface_type: u32,
    pub properties: Vec<(u32, u32)>,
    pub shape: CollisionShape,
}

impl CollisionBody {
    /// `_mat12` bodies (sea plane, death box) kill the player.
    pub fn is_kill_volume(&self) -> bool {
        self.surface_type == 12
    }
}

/// Extracts every rigid body's world-space collision from a stage `.phy.hkx`.
pub fn parse_collision(bytes: &[u8]) -> Result<Vec<CollisionBody>> {
    let pf = Packfile::parse(bytes)?;
    let mut bodies = Vec::new();
    for (body, class) in pf.objects() {
        if class != "hkpRigidBody" {
            continue;
        }
        let body = *body;
        let name = match pf.ptr(body + 0x70) {
            Some(at) => pf.c_str(at)?,
            None => String::new(),
        };
        let mut properties = Vec::new();
        if let Some(at) = pf.ptr(body + 0x74) {
            for i in 0..pf.u32(body + 0x78)? {
                let p = at + i * 16;
                properties.push((pf.u32(p)?, pf.u32(p + 12)?));
            }
        }
        let surface_type = properties
            .iter()
            .find(|(k, _)| *k == PROPERTY_SURFACE_TYPE)
            .map(|(_, v)| *v)
            .unwrap_or(0);

        // hkMotionState transform: rotation columns then translation.
        let c0 = pf.vec4(body + 0xE0)?;
        let c1 = pf.vec4(body + 0xF0)?;
        let c2 = pf.vec4(body + 0x100)?;
        let t = pf.vec4(body + 0x110)?;
        let xf = |p: [f32; 4]| -> [f32; 3] {
            [
                c0[0] * p[0] + c1[0] * p[1] + c2[0] * p[2] + t[0],
                c0[1] * p[0] + c1[1] * p[1] + c2[1] * p[2] + t[1],
                c0[2] * p[0] + c1[2] * p[1] + c2[2] * p[2] + t[2],
            ]
        };

        let shape_at = pf
            .ptr(body + 0x10)
            .with_context(|| format!("rigid body {name} has no shape"))?;
        let shape = match pf.class_of(shape_at) {
            Some("hkpBoxShape") => {
                let h = pf.vec4(shape_at + 0x20)?;
                CollisionShape::Box {
                    center: [t[0], t[1], t[2]],
                    half_extents: [h[0], h[1], h[2]],
                }
            }
            Some("hkpMoppBvTreeShape") => {
                let mesh = pf
                    .ptr(shape_at + 0x34)
                    .context("MOPP shape without child")?;
                if pf.class_of(mesh) != Some("hkpStorageExtendedMeshShape") {
                    bail!("{name}: unsupported MOPP child {:?}", pf.class_of(mesh));
                }
                let (vertices, triangles) = storage_mesh(&pf, mesh, &xf)
                    .with_context(|| format!("rigid body {name}"))?;
                CollisionShape::Mesh {
                    vertices,
                    triangles,
                }
            }
            other => bail!("{name}: unsupported shape {other:?}"),
        };
        bodies.push(CollisionBody {
            name,
            surface_type,
            properties,
            shape,
        });
    }
    Ok(bodies)
}

type MeshData = (Vec<[f32; 3]>, Vec<[u32; 3]>);

fn storage_mesh(pf: &Packfile<'_>, mesh: u32, xf: &dyn Fn([f32; 4]) -> [f32; 3]) -> Result<MeshData> {
    let subparts = pf.ptr(mesh + 0x50).context("no triangle subparts")?;
    let subpart_count = pf.u32(mesh + 0x54)?;
    let storages = pf.ptr(mesh + 0xC0).context("no mesh storage")?;
    let storage_count = pf.u32(mesh + 0xC4)?;
    if storage_count != subpart_count {
        bail!("{subpart_count} subparts but {storage_count} storages");
    }
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for i in 0..storage_count {
        let sub = subparts + i * 0x40;
        let tri_count = pf.u32(sub + 0x10)?;
        let index_stride = pf.u32(sub + 0x34)?;
        let storage = pf
            .ptr(storages + i * 4)
            .context("dangling mesh storage pointer")?;
        let base = vertices.len() as u32;
        let vertex_count = pf.u32(storage + 0x0C)?;
        let vertex_at = pf.ptr(storage + 0x08).context("no vertices")?;
        for v in 0..vertex_count {
            vertices.push(xf(pf.vec4(vertex_at + v * 16)?));
        }
        let (index_at, width) = if pf.u32(storage + 0x24)? > 0 {
            (pf.ptr(storage + 0x20).context("no indices32")?, 4)
        } else {
            (pf.ptr(storage + 0x14).context("no indices16")?, 2)
        };
        for t in 0..tri_count {
            let mut tri = [0u32; 3];
            for (k, slot) in tri.iter_mut().enumerate() {
                let at = index_at + t * index_stride + k as u32 * width;
                let raw = if width == 4 {
                    pf.u32(at)?
                } else {
                    let word = pf.u32(at & !3)?;
                    let shift = if at & 2 == 0 { 16 } else { 0 };
                    (word >> shift) & 0xFFFF
                };
                if raw >= vertex_count {
                    bail!("triangle index {raw} >= vertex count {vertex_count}");
                }
                *slot = base + raw;
            }
            triangles.push(tri);
        }
    }
    Ok((vertices, triangles))
}

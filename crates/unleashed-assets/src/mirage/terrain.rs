use std::fmt;

const MIRAGE_VERSION: u32 = 5;
const D3DDECLTYPE_FLOAT3: u32 = 0x002A_23B9;
const D3DDECLTYPE_UNUSED: u32 = 0xFFFF_FFFF;
const D3DDECLUSAGE_POSITION: u8 = 0;
const DECL_ELEMENT_SIZE: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexFormat {
    U16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshTopology {
    TriangleStrip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainMesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u16>,
    pub index_format: IndexFormat,
    pub topology: MeshTopology,
    pub primitive_restart: Option<u16>,
    pub vertex_stride: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceTransform {
    /// Row-major 4x4 transform as stored in the terrain-instanceinfo resource.
    pub matrix: [[f32; 4]; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainParseError {
    Truncated,
    UnsupportedVersion(u32),
    InvalidHeader,
    InvalidFixupTable,
    MeshNotFound,
    InvalidMesh,
    InstanceTransformNotFound,
}

impl fmt::Display for TerrainParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "resource is truncated"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported Mirage resource version {version}")
            }
            Self::InvalidHeader => write!(f, "invalid Mirage resource header"),
            Self::InvalidFixupTable => write!(f, "invalid Mirage fixup table"),
            Self::MeshNotFound => write!(f, "terrain mesh payload was not found"),
            Self::InvalidMesh => write!(f, "terrain mesh payload is inconsistent"),
            Self::InstanceTransformNotFound => {
                write!(f, "terrain instance transform was not found")
            }
        }
    }
}

impl std::error::Error for TerrainParseError {}

#[derive(Debug)]
struct MirageHeader {
    fixup_table: usize,
    fixups: Vec<usize>,
}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16, TerrainParseError> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or(TerrainParseError::Truncated)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, TerrainParseError> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(TerrainParseError::Truncated)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_f32_be(data: &[u8], offset: usize) -> Result<f32, TerrainParseError> {
    Ok(f32::from_bits(read_u32_be(data, offset)?))
}

fn parse_header(data: &[u8]) -> Result<MirageHeader, TerrainParseError> {
    if data.len() < 0x18 {
        return Err(TerrainParseError::Truncated);
    }

    let declared_size = read_u32_be(data, 0)? as usize;
    if declared_size != data.len() {
        return Err(TerrainParseError::InvalidHeader);
    }

    let version = read_u32_be(data, 4)?;
    if version != MIRAGE_VERSION {
        return Err(TerrainParseError::UnsupportedVersion(version));
    }

    let fixup_table = read_u32_be(data, 0x10)? as usize;
    let fixup_end = read_u32_be(data, 0x14)? as usize;
    if fixup_table >= fixup_end || fixup_end > data.len() || fixup_table + 4 > data.len() {
        return Err(TerrainParseError::InvalidFixupTable);
    }

    let count = read_u32_be(data, fixup_table)? as usize;
    let entries_end = fixup_table
        .checked_add(4)
        .and_then(|v| v.checked_add(count.checked_mul(4)?))
        .ok_or(TerrainParseError::InvalidFixupTable)?;
    if entries_end > fixup_end || entries_end > data.len() {
        return Err(TerrainParseError::InvalidFixupTable);
    }

    let mut fixups = Vec::with_capacity(count);
    for i in 0..count {
        let location = read_u32_be(data, fixup_table + 4 + i * 4)? as usize;
        if location + 4 > data.len() {
            return Err(TerrainParseError::InvalidFixupTable);
        }
        fixups.push(location);
    }

    Ok(MirageHeader {
        fixup_table,
        fixups,
    })
}

fn is_position_decl(data: &[u8], offset: usize) -> bool {
    let Ok(stream) = read_u16_be(data, offset) else {
        return false;
    };
    let Ok(element_offset) = read_u16_be(data, offset + 2) else {
        return false;
    };
    let Ok(element_type) = read_u32_be(data, offset + 4) else {
        return false;
    };
    let Some(tail) = data.get(offset + 8..offset + 12) else {
        return false;
    };

    stream == 0
        && element_offset == 0
        && element_type == D3DDECLTYPE_FLOAT3
        && tail[0] == 0
        && tail[1] == D3DDECLUSAGE_POSITION
        && tail[2] == 0
}

fn validate_decl(data: &[u8], start: usize) -> bool {
    if !is_position_decl(data, start) {
        return false;
    }

    // tg-0068 declarations are ordinary 12-byte Xbox 360 vertex elements,
    // terminated by stream=0xFF/type=UNUSED.  Bound the walk so corrupt files
    // cannot scan arbitrarily far.
    for i in 1..=32 {
        let offset = start + i * DECL_ELEMENT_SIZE;
        let Ok(stream) = read_u16_be(data, offset) else {
            return false;
        };
        let Ok(element_type) = read_u32_be(data, offset + 4) else {
            return false;
        };
        if stream == 0x00FF && element_type == D3DDECLTYPE_UNUSED {
            return true;
        }
    }

    false
}

fn align_down_4(value: usize) -> usize {
    value & !3
}

pub fn parse_terrain_mesh(data: &[u8]) -> Result<TerrainMesh, TerrainParseError> {
    let header = parse_header(data)?;

    let declaration_start = (0..data.len().saturating_sub(DECL_ELEMENT_SIZE))
        .find(|&offset| offset % 4 == 0 && validate_decl(data, offset))
        .ok_or(TerrainParseError::MeshNotFound)?;

    // In the proven v5 terrain-model resource, the serialized mesh metadata is
    // discovered structurally rather than by assuming the observed 0x58 object
    // boundary.  Three pointer-bearing words around the metadata are covered by
    // the Mirage fixup table.  Counts/stride must derive index and vertex spans
    // that terminate exactly at the vertex declaration.
    let fixup = |offset: usize| header.fixups.contains(&offset);
    let mut found = None;

    for meta in (4..declaration_start.saturating_sub(0x28)).step_by(4) {
        if meta < 4 || !fixup(meta - 4) || !fixup(meta + 4) || !fixup(meta + 12) {
            continue;
        }

        let index_count = read_u32_be(data, meta)? as usize;
        let vertex_count = read_u32_be(data, meta + 8)? as usize;
        let vertex_stride = read_u32_be(data, meta + 12)? as usize;

        if index_count == 0
            || vertex_count == 0
            || vertex_stride < 12
            || vertex_stride > 256
            || vertex_stride % 4 != 0
        {
            continue;
        }

        let Some(vertex_bytes) = vertex_count.checked_mul(vertex_stride) else {
            continue;
        };
        let Some(vertex_start) = declaration_start.checked_sub(vertex_bytes) else {
            continue;
        };
        let Some(index_bytes) = index_count.checked_mul(2) else {
            continue;
        };
        let Some(index_unaligned) = vertex_start.checked_sub(index_bytes) else {
            continue;
        };
        let index_start = align_down_4(index_unaligned);

        // The metadata block is immediately followed by the raw index payload in
        // all three tg-0068 terrain models.  This relation is offset-independent
        // and prevents a false match elsewhere in the resource.
        if meta + 0x28 != index_start {
            continue;
        }
        if vertex_start > declaration_start
            || index_start + index_bytes > vertex_start
            || declaration_start > data.len()
        {
            continue;
        }

        found = Some((
            index_count,
            vertex_count,
            vertex_stride,
            index_start,
            vertex_start,
        ));
        break;
    }

    let Some((index_count, vertex_count, vertex_stride, index_start, vertex_start)) = found else {
        return Err(TerrainParseError::MeshNotFound);
    };

    let mut indices = Vec::with_capacity(index_count);
    for i in 0..index_count {
        indices.push(read_u16_be(data, index_start + i * 2)?);
    }

    let mut positions = Vec::with_capacity(vertex_count);
    for i in 0..vertex_count {
        let base = vertex_start + i * vertex_stride;
        let x = read_f32_be(data, base)?;
        let y = read_f32_be(data, base + 4)?;
        let z = read_f32_be(data, base + 8)?;
        if !(x.is_finite() && y.is_finite() && z.is_finite()) {
            return Err(TerrainParseError::InvalidMesh);
        }
        positions.push([x, y, z]);
    }

    // The runtime workaround in UnleashedRecomp operates on these resources as
    // u16 triangle strips and explicitly treats 0xFFFF as primitive restart.
    Ok(TerrainMesh {
        positions,
        indices,
        index_format: IndexFormat::U16,
        topology: MeshTopology::TriangleStrip,
        primitive_restart: Some(0xFFFF),
        vertex_stride: vertex_stride as u32,
    })
}

pub fn parse_terrain_instance_transform(
    data: &[u8],
) -> Result<InstanceTransform, TerrainParseError> {
    let header = parse_header(data)?;

    // A v5 terrain-instanceinfo starts its serialized object immediately after
    // the 0x18-byte Mirage header.  The first three object words are
    // body-relative offsets:
    //
    //   +0x00 -> referenced terrain-model stem string
    //   +0x04 -> row-major 4x4 transform
    //   +0x08 -> byte immediately after that 64-byte transform
    //
    // This relation was verified across all 2,568 Windmill Isle Act 1 instance
    // resources.  Matrix addresses vary with the model-stem length, so scanning
    // for an affine-looking float window is both unnecessary and unsafe.
    const BODY_START: usize = 0x18;
    const MATRIX_BYTES: usize = 16 * 4;

    let model_rel = read_u32_be(data, BODY_START)? as usize;
    let matrix_rel = read_u32_be(data, BODY_START + 4)? as usize;
    let matrix_end_rel = read_u32_be(data, BODY_START + 8)? as usize;

    let model_start = BODY_START
        .checked_add(model_rel)
        .ok_or(TerrainParseError::InstanceTransformNotFound)?;
    let matrix_start = BODY_START
        .checked_add(matrix_rel)
        .ok_or(TerrainParseError::InstanceTransformNotFound)?;
    let matrix_end = matrix_start
        .checked_add(MATRIX_BYTES)
        .ok_or(TerrainParseError::InstanceTransformNotFound)?;

    if matrix_rel % 4 != 0
        || matrix_end_rel != matrix_rel + MATRIX_BYTES
        || model_start >= matrix_start
        || matrix_end > header.fixup_table
    {
        return Err(TerrainParseError::InstanceTransformNotFound);
    }

    // The first relative offset identifies the referenced model stem.  Require
    // its NUL terminator to occur before the transform so corrupt relative
    // offsets cannot accidentally reinterpret unrelated bytes as a matrix.
    let model_region = data
        .get(model_start..matrix_start)
        .ok_or(TerrainParseError::InstanceTransformNotFound)?;
    let Some(model_end) = model_region.iter().position(|byte| *byte == 0) else {
        return Err(TerrainParseError::InstanceTransformNotFound);
    };
    if model_end == 0 {
        return Err(TerrainParseError::InstanceTransformNotFound);
    }

    let mut values = [0.0f32; 16];
    for (i, value) in values.iter_mut().enumerate() {
        let parsed = read_f32_be(data, matrix_start + i * 4)?;
        if !parsed.is_finite() {
            return Err(TerrainParseError::InstanceTransformNotFound);
        }
        *value = parsed;
    }

    // Supplementary integrity check for the serialized affine transform.  The
    // location itself comes exclusively from the resource's relative offsets.
    if values[12].to_bits() != 0.0f32.to_bits()
        || values[13].to_bits() != 0.0f32.to_bits()
        || values[14].to_bits() != 0.0f32.to_bits()
        || values[15].to_bits() != 1.0f32.to_bits()
    {
        return Err(TerrainParseError::InstanceTransformNotFound);
    }

    Ok(InstanceTransform {
        matrix: [
            [values[0], values[1], values[2], values[3]],
            [values[4], values[5], values[6], values[7]],
            [values[8], values[9], values[10], values[11]],
            [values[12], values[13], values[14], values[15]],
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_f32(data: &mut [u8], offset: usize, value: f32) {
        put_u32(data, offset, value.to_bits());
    }

    fn synthetic_model() -> Vec<u8> {
        // Deliberately place the mesh metadata at 0x70 rather than 0x5c/0x58 so
        // the test proves discovery is structural rather than hardcoded.
        let meta = 0x70usize;
        let index_start = meta + 0x28;
        let indices = [0u16, 1, 2, 0xFFFF, 2, 1, 0];
        let index_bytes = indices.len() * 2;
        let vertex_start = (index_start + index_bytes + 3) & !3;
        let stride = 16usize;
        let vertices = [[1.0f32, 2.0, 3.0], [-4.0, 5.5, 6.0], [7.0, 8.0, -9.0]];
        let decl = vertex_start + vertices.len() * stride;
        let fixup_table = decl + 24;
        let fixups = [meta - 4, meta + 4, meta + 12];
        let fixup_end = fixup_table + 4 + fixups.len() * 4;
        let size = fixup_end + 4;

        let mut data = vec![0u8; size];
        put_u32(&mut data, 0, size as u32);
        put_u32(&mut data, 4, MIRAGE_VERSION);
        put_u32(&mut data, 0x10, fixup_table as u32);
        put_u32(&mut data, 0x14, fixup_end as u32);

        put_u32(&mut data, meta, indices.len() as u32);
        put_u32(&mut data, meta + 4, vertex_start as u32);
        put_u32(&mut data, meta + 8, vertices.len() as u32);
        put_u32(&mut data, meta + 12, stride as u32);

        for (i, index) in indices.iter().copied().enumerate() {
            put_u16(&mut data, index_start + i * 2, index);
        }
        for (i, vertex) in vertices.iter().enumerate() {
            let base = vertex_start + i * stride;
            put_f32(&mut data, base, vertex[0]);
            put_f32(&mut data, base + 4, vertex[1]);
            put_f32(&mut data, base + 8, vertex[2]);
        }

        put_u16(&mut data, decl, 0);
        put_u16(&mut data, decl + 2, 0);
        put_u32(&mut data, decl + 4, D3DDECLTYPE_FLOAT3);
        data[decl + 8] = 0;
        data[decl + 9] = D3DDECLUSAGE_POSITION;
        data[decl + 10] = 0;
        data[decl + 11] = 0;

        put_u16(&mut data, decl + 12, 0x00FF);
        put_u16(&mut data, decl + 14, 0);
        put_u32(&mut data, decl + 16, D3DDECLTYPE_UNUSED);

        put_u32(&mut data, fixup_table, fixups.len() as u32);
        for (i, fixup) in fixups.iter().copied().enumerate() {
            put_u32(&mut data, fixup_table + 4 + i * 4, fixup as u32);
        }

        data
    }

    fn synthetic_instance() -> Vec<u8> {
        const BODY_START: usize = 0x18;
        let model_start = 0x34usize;
        let transform = 0xa0usize;
        let fixup_table = 0xe0usize;
        let fixups = [0x00usize, 0x04, 0x08, 0x10];
        let fixup_end = fixup_table + 4 + fixups.len() * 4;
        let size = fixup_end + 4;

        let mut data = vec![0u8; size];
        put_u32(&mut data, 0, size as u32);
        put_u32(&mut data, 4, MIRAGE_VERSION);
        put_u32(&mut data, 0x10, fixup_table as u32);
        put_u32(&mut data, 0x14, fixup_end as u32);

        put_u32(&mut data, BODY_START, (model_start - BODY_START) as u32);
        put_u32(&mut data, BODY_START + 4, (transform - BODY_START) as u32);
        put_u32(
            &mut data,
            BODY_START + 8,
            (transform + 64 - BODY_START) as u32,
        );
        data[model_start..model_start + 11].copy_from_slice(b"test_model\0");

        // An earlier affine-looking window proves the parser follows the
        // serialized relative transform offset instead of scanning for floats.
        let decoy = [
            [9.0f32, 0.0, 0.0, 900.0],
            [0.0, 9.0, 0.0, 900.0],
            [0.0, 0.0, 9.0, 900.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let decoy_start = 0x50usize;
        for row in 0..4 {
            for col in 0..4 {
                put_f32(
                    &mut data,
                    decoy_start + (row * 4 + col) * 4,
                    decoy[row][col],
                );
            }
        }

        let matrix = [
            [1.0f32, 0.0, 0.0, 10.0],
            [0.0, 2.0, 0.0, 20.0],
            [0.0, 0.0, 3.0, 30.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        for row in 0..4 {
            for col in 0..4 {
                put_f32(&mut data, transform + (row * 4 + col) * 4, matrix[row][col]);
            }
        }

        put_u32(&mut data, fixup_table, fixups.len() as u32);
        for (i, fixup) in fixups.iter().copied().enumerate() {
            put_u32(&mut data, fixup_table + 4 + i * 4, fixup as u32);
        }

        data
    }

    #[test]
    fn parses_mesh_without_hardcoded_observed_offset() {
        let mesh = parse_terrain_mesh(&synthetic_model()).unwrap();
        assert_eq!(mesh.vertex_stride, 16);
        assert_eq!(
            mesh.positions,
            vec![[1.0, 2.0, 3.0], [-4.0, 5.5, 6.0], [7.0, 8.0, -9.0]]
        );
        assert_eq!(mesh.indices, vec![0, 1, 2, 0xFFFF, 2, 1, 0]);
        assert_eq!(mesh.index_format, IndexFormat::U16);
        assert_eq!(mesh.topology, MeshTopology::TriangleStrip);
        assert_eq!(mesh.primitive_restart, Some(0xFFFF));
    }

    #[test]
    fn parses_instance_transform_from_serialized_relative_offset() {
        let parsed = parse_terrain_instance_transform(&synthetic_instance()).unwrap();
        assert_eq!(parsed.matrix[0], [1.0, 0.0, 0.0, 10.0]);
        assert_eq!(parsed.matrix[1], [0.0, 2.0, 0.0, 20.0]);
        assert_eq!(parsed.matrix[2], [0.0, 0.0, 3.0, 30.0]);
        assert_eq!(parsed.matrix[3], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn rejects_inconsistent_instance_transform_span() {
        let mut data = synthetic_instance();
        let matrix_rel = read_u32_be(&data, 0x1c).unwrap();
        put_u32(&mut data, 0x20, matrix_rel + 60);
        assert_eq!(
            parse_terrain_instance_transform(&data),
            Err(TerrainParseError::InstanceTransformNotFound)
        );
    }

    #[test]
    fn rejects_bad_declared_size() {
        let mut data = synthetic_model();
        put_u32(&mut data, 0, 123);
        assert_eq!(
            parse_terrain_mesh(&data),
            Err(TerrainParseError::InvalidHeader)
        );
    }
}

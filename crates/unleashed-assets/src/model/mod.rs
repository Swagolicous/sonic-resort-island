//! Minimal Sonic Unleashed Mirage character-model parsing.
//!
//! This parser intentionally targets the serialized layout evidenced by
//! `SonicRoot.model`. It does not parse the Havok/model skeleton. Unknown
//! resource fields remain exposed as validated raw slices instead of being
//! assigned speculative semantics.

use std::{fmt, str};

const MIRAGE_VERSION: u32 = 5;
const MIRAGE_DATA_BASE: usize = 0x18;
const VERTEX_ELEMENT_SIZE: usize = 12;
const VERTEX_DECL_END_STREAM: u16 = 0x00ff;
const VERTEX_DECL_END_TYPE: u32 = 0xffff_ffff;
const D3DDECLTYPE_FLOAT3: u32 = 0x002a_23b9;
const MAX_VERTEX_ELEMENTS: usize = 32;
const MAX_VERTEX_STRIDE: usize = 256;
const MATERIAL_TRAILER_LIMIT: usize = 0x200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexUsage {
    Position,
    BlendWeight,
    BlendIndices,
    Normal,
    PointSize,
    TexCoord,
    Tangent,
    Binormal,
    TessFactor,
    PositionTransformed,
    Color,
    Fog,
    Depth,
    Sample,
    Unknown(u8),
}

impl VertexUsage {
    fn from_raw(value: u8) -> Self {
        match value {
            0 => Self::Position,
            1 => Self::BlendWeight,
            2 => Self::BlendIndices,
            3 => Self::Normal,
            4 => Self::PointSize,
            5 => Self::TexCoord,
            6 => Self::Tangent,
            7 => Self::Binormal,
            8 => Self::TessFactor,
            9 => Self::PositionTransformed,
            10 => Self::Color,
            11 => Self::Fog,
            12 => Self::Depth,
            13 => Self::Sample,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexElement {
    pub stream: u16,
    pub offset: u16,
    /// Raw Xbox 360 D3D declaration type. The parser does not guess formats
    /// which have not yet been independently decoded.
    pub element_type: u32,
    pub method: u8,
    pub usage: VertexUsage,
    pub usage_index: u8,
    pub padding: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticSpan {
    pub stream: u16,
    pub offset: u16,
    /// Number of bytes occupied by this semantic inside one vertex, derived
    /// from the following element offset (or the vertex stride).
    pub width: u16,
    pub element_type: u32,
    pub usage_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkinningSemantics {
    pub blend_indices: Option<SemanticSpan>,
    pub blend_weights: Option<SemanticSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VertexStream<'a> {
    pub stream: u16,
    pub stride: u32,
    pub vertex_count: u32,
    pub data: &'a [u8],
}

impl<'a> VertexStream<'a> {
    pub fn vertex(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.vertex_count as usize {
            return None;
        }
        let stride = self.stride as usize;
        let start = index.checked_mul(stride)?;
        self.data.get(start..start.checked_add(stride)?)
    }

    pub fn semantic_bytes(&self, index: usize, semantic: SemanticSpan) -> Option<&'a [u8]> {
        if semantic.stream != self.stream {
            return None;
        }
        let vertex = self.vertex(index)?;
        let start = semantic.offset as usize;
        let end = start.checked_add(semantic.width as usize)?;
        vertex.get(start..end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMesh<'a> {
    /// Offset of the evidenced 0x28-byte serialized mesh record.
    pub resource_offset: usize,
    pub index_count: u32,
    /// Raw big-endian u16 index payload.
    pub index_bytes: &'a [u8],
    /// Decoded host-endian indices. `0xffff` is preserved verbatim.
    pub indices: Vec<u16>,
    pub vertex_count: u32,
    pub vertex_stride: u32,
    /// SonicRoot stores the evidenced character meshes in stream zero.
    pub vertex_stream: VertexStream<'a>,
    pub vertex_declaration_bytes: &'a [u8],
    pub vertex_elements: Vec<VertexElement>,
    pub skinning: SkinningSemantics,
    /// Per-mesh model node palette. Blend indices refer into this palette;
    /// mapping those entries through a Havok/model skeleton is intentionally
    /// outside this parser.
    pub node_palette: &'a [u8],
    /// Unknown pointer-array metadata after the node palette. The count and
    /// bounds are validated, but no semantic meaning is assigned yet.
    pub auxiliary_bytes: &'a [u8],
    /// Material/database reference string evidenced in the post-declaration
    /// trailer (for example `sonic_gm_body`).
    pub material_reference: Option<&'a str>,
}

impl ModelMesh<'_> {
    pub fn blend_indices_bytes(&self, vertex: usize) -> Option<&[u8]> {
        self.vertex_stream
            .semantic_bytes(vertex, self.skinning.blend_indices?)
    }

    pub fn blend_weights_bytes(&self, vertex: usize) -> Option<&[u8]> {
        self.vertex_stream
            .semantic_bytes(vertex, self.skinning.blend_weights?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model<'a> {
    pub meshes: Vec<ModelMesh<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelParseError {
    Truncated,
    UnsupportedVersion(u32),
    InvalidHeader,
    MeshNotFound,
    InvalidMesh,
    AmbiguousMesh,
}

impl fmt::Display for ModelParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "model resource is truncated"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported Mirage resource version {version}")
            }
            Self::InvalidHeader => write!(f, "invalid Mirage model header"),
            Self::MeshNotFound => write!(f, "no validated character mesh was found"),
            Self::InvalidMesh => write!(f, "character mesh payload is inconsistent"),
            Self::AmbiguousMesh => write!(f, "vertex declaration maps to multiple mesh records"),
        }
    }
}

impl std::error::Error for ModelParseError {}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16, ModelParseError> {
    let bytes = data
        .get(offset..offset.checked_add(2).ok_or(ModelParseError::Truncated)?)
        .ok_or(ModelParseError::Truncated)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, ModelParseError> {
    let bytes = data
        .get(offset..offset.checked_add(4).ok_or(ModelParseError::Truncated)?)
        .ok_or(ModelParseError::Truncated)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn resolve_data_pointer(value: u32, len: usize) -> Option<usize> {
    let offset = MIRAGE_DATA_BASE.checked_add(value as usize)?;
    (offset <= len).then_some(offset)
}

fn parse_header(data: &[u8]) -> Result<usize, ModelParseError> {
    if data.len() < MIRAGE_DATA_BASE {
        return Err(ModelParseError::Truncated);
    }
    if read_u32_be(data, 0)? as usize != data.len() {
        return Err(ModelParseError::InvalidHeader);
    }
    let version = read_u32_be(data, 4)?;
    if version != MIRAGE_VERSION {
        return Err(ModelParseError::UnsupportedVersion(version));
    }

    let data_end = read_u32_be(data, 8)? as usize;
    if data_end < MIRAGE_DATA_BASE || data_end > data.len() {
        return Err(ModelParseError::InvalidHeader);
    }

    Ok(data_end)
}

fn declaration_len(data: &[u8], start: usize) -> Option<usize> {
    if read_u16_be(data, start).ok()? != 0
        || read_u16_be(data, start + 2).ok()? != 0
        || read_u32_be(data, start + 4).ok()? != D3DDECLTYPE_FLOAT3
        || *data.get(start + 9)? != 0
    {
        return None;
    }

    for index in 1..=MAX_VERTEX_ELEMENTS {
        let offset = start.checked_add(index.checked_mul(VERTEX_ELEMENT_SIZE)?)?;
        if read_u16_be(data, offset).ok()? == VERTEX_DECL_END_STREAM
            && read_u32_be(data, offset + 4).ok()? == VERTEX_DECL_END_TYPE
        {
            return (index + 1).checked_mul(VERTEX_ELEMENT_SIZE);
        }
    }

    None
}

fn parse_vertex_elements(
    data: &[u8],
    declaration_start: usize,
    declaration_len: usize,
    vertex_stride: usize,
) -> Result<Vec<VertexElement>, ModelParseError> {
    if declaration_len < VERTEX_ELEMENT_SIZE * 2 || declaration_len % VERTEX_ELEMENT_SIZE != 0 {
        return Err(ModelParseError::InvalidMesh);
    }

    let count = declaration_len / VERTEX_ELEMENT_SIZE - 1;
    let mut elements = Vec::with_capacity(count);

    for index in 0..count {
        let offset = declaration_start + index * VERTEX_ELEMENT_SIZE;
        let element = VertexElement {
            stream: read_u16_be(data, offset)?,
            offset: read_u16_be(data, offset + 2)?,
            element_type: read_u32_be(data, offset + 4)?,
            method: data[offset + 8],
            usage: VertexUsage::from_raw(data[offset + 9]),
            usage_index: data[offset + 10],
            padding: data[offset + 11],
        };

        // SonicRoot's serialized mesh record exposes one vertex payload and all
        // evidenced declarations bind it as stream zero. Rejecting other streams
        // avoids pretending we know how additional serialized streams are stored.
        if element.stream != 0 || element.offset as usize >= vertex_stride {
            return Err(ModelParseError::InvalidMesh);
        }
        elements.push(element);
    }

    Ok(elements)
}

fn semantic_span(
    elements: &[VertexElement],
    element_index: usize,
    stride: usize,
) -> Result<SemanticSpan, ModelParseError> {
    let element = elements
        .get(element_index)
        .copied()
        .ok_or(ModelParseError::InvalidMesh)?;
    let start = element.offset as usize;

    let end = elements
        .iter()
        .skip(element_index + 1)
        .find(|next| next.stream == element.stream && next.offset as usize > start)
        .map(|next| next.offset as usize)
        .unwrap_or(stride);

    if end <= start || end > stride || end - start > u16::MAX as usize {
        return Err(ModelParseError::InvalidMesh);
    }

    Ok(SemanticSpan {
        stream: element.stream,
        offset: element.offset,
        width: (end - start) as u16,
        element_type: element.element_type,
        usage_index: element.usage_index,
    })
}

fn skinning_semantics(
    elements: &[VertexElement],
    stride: usize,
) -> Result<SkinningSemantics, ModelParseError> {
    let blend_indices = elements
        .iter()
        .position(|element| element.usage == VertexUsage::BlendIndices)
        .map(|index| semantic_span(elements, index, stride))
        .transpose()?;
    let blend_weights = elements
        .iter()
        .position(|element| element.usage == VertexUsage::BlendWeight)
        .map(|index| semantic_span(elements, index, stride))
        .transpose()?;

    Ok(SkinningSemantics {
        blend_indices,
        blend_weights,
    })
}

#[derive(Debug, Clone, Copy)]
struct MeshCandidate {
    resource_offset: usize,
    declaration_start: usize,
    declaration_len: usize,
    index_count: usize,
    index_start: usize,
    vertex_count: usize,
    vertex_stride: usize,
    vertex_start: usize,
    node_count: usize,
    node_start: usize,
    auxiliary_count: usize,
    auxiliary_start: usize,
}

fn find_candidate_for_declaration(
    data: &[u8],
    declaration_start: usize,
    declaration_len: usize,
) -> Result<MeshCandidate, ModelParseError> {
    let encoded_declaration = declaration_start
        .checked_sub(MIRAGE_DATA_BASE)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(ModelParseError::InvalidMesh)?;

    let mut found = None;

    for pointer_field in (MIRAGE_DATA_BASE..declaration_start).step_by(4) {
        if read_u32_be(data, pointer_field)? != encoded_declaration || pointer_field < 0x14 {
            continue;
        }

        // In the evidenced resource the declaration pointer is field +0x14 of
        // the 0x28-byte serialized mesh record.
        let resource_offset = pointer_field - 0x14;
        let Some(resource_end) = resource_offset.checked_add(0x28) else {
            continue;
        };
        if resource_end > data.len() {
            continue;
        }

        let index_count = read_u32_be(data, resource_offset)? as usize;
        let vertex_count = read_u32_be(data, resource_offset + 8)? as usize;
        let vertex_stride = read_u32_be(data, resource_offset + 12)? as usize;
        let node_count = read_u32_be(data, resource_offset + 24)? as usize;
        let auxiliary_count = read_u32_be(data, resource_offset + 32)? as usize;

        if index_count == 0
            || vertex_count == 0
            || vertex_stride < 12
            || vertex_stride > MAX_VERTEX_STRIDE
            || vertex_stride % 4 != 0
        {
            continue;
        }

        let Some(index_start) =
            resolve_data_pointer(read_u32_be(data, resource_offset + 4)?, data.len())
        else {
            continue;
        };
        let Some(vertex_start) =
            resolve_data_pointer(read_u32_be(data, resource_offset + 16)?, data.len())
        else {
            continue;
        };
        let Some(declaration_pointer) =
            resolve_data_pointer(read_u32_be(data, resource_offset + 20)?, data.len())
        else {
            continue;
        };
        let Some(node_start) =
            resolve_data_pointer(read_u32_be(data, resource_offset + 28)?, data.len())
        else {
            continue;
        };
        let Some(auxiliary_start) =
            resolve_data_pointer(read_u32_be(data, resource_offset + 36)?, data.len())
        else {
            continue;
        };

        if declaration_pointer != declaration_start {
            continue;
        }

        let Some(index_bytes) = index_count.checked_mul(2) else {
            continue;
        };
        let Some(index_end) = index_start.checked_add(index_bytes) else {
            continue;
        };
        let Some(vertex_bytes) = vertex_count.checked_mul(vertex_stride) else {
            continue;
        };
        let Some(vertex_end) = vertex_start.checked_add(vertex_bytes) else {
            continue;
        };
        let Some(node_end) = node_start.checked_add(node_count) else {
            continue;
        };
        let Some(auxiliary_bytes) = auxiliary_count.checked_mul(4) else {
            continue;
        };
        let Some(auxiliary_end) = auxiliary_start.checked_add(auxiliary_bytes) else {
            continue;
        };
        let Some(declaration_end) = declaration_start.checked_add(declaration_len) else {
            continue;
        };

        if index_end > vertex_start
            || vertex_end != declaration_start
            || declaration_end > data.len()
            || node_end > data.len()
            || auxiliary_end > data.len()
        {
            continue;
        }

        // Every non-restart index in SonicRoot is local to this mesh's vertex
        // payload. This catches false declaration references very effectively.
        let mut indices_valid = true;
        for index in 0..index_count {
            let value = read_u16_be(data, index_start + index * 2)?;
            if value != 0xffff && value as usize >= vertex_count {
                indices_valid = false;
                break;
            }
        }
        if !indices_valid {
            continue;
        }

        let candidate = MeshCandidate {
            resource_offset,
            declaration_start,
            declaration_len,
            index_count,
            index_start,
            vertex_count,
            vertex_stride,
            vertex_start,
            node_count,
            node_start,
            auxiliary_count,
            auxiliary_start,
        };

        if found.replace(candidate).is_some() {
            return Err(ModelParseError::AmbiguousMesh);
        }
    }

    found.ok_or(ModelParseError::MeshNotFound)
}

fn last_ascii_c_string(data: &[u8], start: usize, end: usize) -> Option<&str> {
    if start >= end || end > data.len() {
        return None;
    }

    let mut last = None;
    let mut cursor = start;
    while cursor < end {
        if !data[cursor].is_ascii_graphic() {
            cursor += 1;
            continue;
        }

        let string_start = cursor;
        while cursor < end && data[cursor].is_ascii_graphic() {
            cursor += 1;
        }

        if cursor < end && data[cursor] == 0 && cursor - string_start >= 4 {
            if let Ok(value) = str::from_utf8(&data[string_start..cursor]) {
                last = Some(value);
            }
        }
        cursor += 1;
    }
    last
}

/// Parses the serialized Sonic character-model mesh payload evidenced by
/// `SonicRoot.model`.
///
/// The parser deliberately does not parse model/Havok skeleton structures.
/// `node_palette` plus BLENDINDICES/BLENDWEIGHT declaration spans are enough
/// for a later animation layer to connect skinning once skeleton semantics are
/// separately proven.
pub fn parse_model(data: &[u8]) -> Result<Model<'_>, ModelParseError> {
    let data_end = parse_header(data)?;
    let search_end = data_end.min(data.len());

    let mut candidates = Vec::new();
    let mut offset = MIRAGE_DATA_BASE;
    while offset + VERTEX_ELEMENT_SIZE <= search_end {
        if offset % 4 == 0 {
            if let Some(len) = declaration_len(data, offset) {
                if offset + len <= search_end {
                    if let Ok(candidate) = find_candidate_for_declaration(data, offset, len) {
                        candidates.push(candidate);
                    }
                }
            }
        }
        offset += 4;
    }

    if candidates.is_empty() {
        return Err(ModelParseError::MeshNotFound);
    }

    candidates.sort_by_key(|candidate| candidate.resource_offset);
    candidates.dedup_by_key(|candidate| candidate.resource_offset);

    let mut meshes = Vec::with_capacity(candidates.len());

    for (index, candidate) in candidates.iter().copied().enumerate() {
        let index_end = candidate.index_start + candidate.index_count * 2;
        let vertex_end = candidate.vertex_start + candidate.vertex_count * candidate.vertex_stride;
        let declaration_end = candidate.declaration_start + candidate.declaration_len;
        let node_end = candidate.node_start + candidate.node_count;
        let auxiliary_end =
            candidate.auxiliary_start + candidate.auxiliary_count * std::mem::size_of::<u32>();

        let index_bytes = &data[candidate.index_start..index_end];
        let mut indices = Vec::with_capacity(candidate.index_count);
        for element in index_bytes.chunks_exact(2) {
            indices.push(u16::from_be_bytes([element[0], element[1]]));
        }

        let vertex_elements = parse_vertex_elements(
            data,
            candidate.declaration_start,
            candidate.declaration_len,
            candidate.vertex_stride,
        )?;
        let skinning = skinning_semantics(&vertex_elements, candidate.vertex_stride)?;

        let next_record = candidates
            .get(index + 1)
            .map(|next| next.resource_offset)
            .unwrap_or(search_end);
        let trailer_limit = declaration_end
            .checked_add(MATERIAL_TRAILER_LIMIT)
            .unwrap_or(search_end)
            .min(next_record)
            .min(search_end);
        let material_reference = last_ascii_c_string(data, declaration_end, trailer_limit);

        meshes.push(ModelMesh {
            resource_offset: candidate.resource_offset,
            index_count: candidate.index_count as u32,
            index_bytes,
            indices,
            vertex_count: candidate.vertex_count as u32,
            vertex_stride: candidate.vertex_stride as u32,
            vertex_stream: VertexStream {
                stream: 0,
                stride: candidate.vertex_stride as u32,
                vertex_count: candidate.vertex_count as u32,
                data: &data[candidate.vertex_start..vertex_end],
            },
            vertex_declaration_bytes: &data[candidate.declaration_start..declaration_end],
            vertex_elements,
            skinning,
            node_palette: &data[candidate.node_start..node_end],
            auxiliary_bytes: &data[candidate.auxiliary_start..auxiliary_end],
            material_reference,
        });
    }

    Ok(Model { meshes })
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

    fn encode_pointer(offset: usize) -> u32 {
        u32::try_from(offset - MIRAGE_DATA_BASE).unwrap()
    }

    fn synthetic_model() -> Vec<u8> {
        let resource = 0x40usize;
        let index_start = 0x80usize;
        let vertex_start = 0x88usize;
        let stride = 24usize;
        let vertex_count = 3usize;
        let declaration = vertex_start + stride * vertex_count;
        let declaration_len = VERTEX_ELEMENT_SIZE * 4;
        let node_palette = declaration + declaration_len;
        let auxiliary = node_palette + 3;
        let material = auxiliary + 8;
        let data_end = material + b"test_material\0".len();
        let len = (data_end + 3) & !3;

        let mut data = vec![0u8; len];
        put_u32(&mut data, 0, len as u32);
        put_u32(&mut data, 4, MIRAGE_VERSION);
        put_u32(&mut data, 8, data_end as u32);
        put_u32(&mut data, 0x0c, MIRAGE_DATA_BASE as u32);

        // 3 indices: 0, 1, 2.
        put_u16(&mut data, index_start, 0);
        put_u16(&mut data, index_start + 2, 1);
        put_u16(&mut data, index_start + 4, 2);

        // Three opaque raw vertices.
        for vertex in 0..vertex_count {
            let base = vertex_start + vertex * stride;
            for byte in 0..stride {
                data[base + byte] = (vertex * stride + byte) as u8;
            }
        }

        // POSITION float3, BLENDINDICES raw 4 bytes, BLENDWEIGHT raw 8 bytes.
        put_u16(&mut data, declaration, 0);
        put_u16(&mut data, declaration + 2, 0);
        put_u32(&mut data, declaration + 4, D3DDECLTYPE_FLOAT3);
        data[declaration + 9] = 0;

        let blend_indices = declaration + 12;
        put_u16(&mut data, blend_indices, 0);
        put_u16(&mut data, blend_indices + 2, 12);
        put_u32(&mut data, blend_indices + 4, 0x1111_1111);
        data[blend_indices + 9] = 2;

        let blend_weights = declaration + 24;
        put_u16(&mut data, blend_weights, 0);
        put_u16(&mut data, blend_weights + 2, 16);
        put_u32(&mut data, blend_weights + 4, 0x2222_2222);
        data[blend_weights + 9] = 1;

        let terminator = declaration + 36;
        put_u16(&mut data, terminator, VERTEX_DECL_END_STREAM);
        put_u32(&mut data, terminator + 4, VERTEX_DECL_END_TYPE);

        data[node_palette..node_palette + 3].copy_from_slice(&[7, 9, 11]);
        put_u32(&mut data, auxiliary, 0x1234_5678);
        put_u32(&mut data, auxiliary + 4, 0x9abc_def0);
        data[material..material + b"test_material\0".len()].copy_from_slice(b"test_material\0");

        put_u32(&mut data, resource, 3);
        put_u32(&mut data, resource + 4, encode_pointer(index_start));
        put_u32(&mut data, resource + 8, vertex_count as u32);
        put_u32(&mut data, resource + 12, stride as u32);
        put_u32(&mut data, resource + 16, encode_pointer(vertex_start));
        put_u32(&mut data, resource + 20, encode_pointer(declaration));
        put_u32(&mut data, resource + 24, 3);
        put_u32(&mut data, resource + 28, encode_pointer(node_palette));
        put_u32(&mut data, resource + 32, 2);
        put_u32(&mut data, resource + 36, encode_pointer(auxiliary));

        data
    }

    #[test]
    fn parses_mesh_stream_indices_material_and_skinning_semantics() {
        let data = synthetic_model();
        let model = parse_model(&data).unwrap();

        assert_eq!(model.meshes.len(), 1);
        let mesh = &model.meshes[0];

        assert_eq!(mesh.resource_offset, 0x40);
        assert_eq!(mesh.indices, [0, 1, 2]);
        assert_eq!(mesh.index_bytes, [0, 0, 0, 1, 0, 2]);
        assert_eq!(mesh.vertex_count, 3);
        assert_eq!(mesh.vertex_stride, 24);
        assert_eq!(mesh.vertex_stream.data.len(), 72);
        assert_eq!(mesh.node_palette, [7, 9, 11]);
        assert_eq!(mesh.auxiliary_bytes.len(), 8);
        assert_eq!(mesh.material_reference, Some("test_material"));

        let skinning = mesh.skinning;
        assert_eq!(
            skinning.blend_indices,
            Some(SemanticSpan {
                stream: 0,
                offset: 12,
                width: 4,
                element_type: 0x1111_1111,
                usage_index: 0,
            })
        );
        assert_eq!(
            skinning.blend_weights,
            Some(SemanticSpan {
                stream: 0,
                offset: 16,
                width: 8,
                element_type: 0x2222_2222,
                usage_index: 0,
            })
        );
        assert_eq!(
            mesh.blend_indices_bytes(1),
            Some(&mesh.vertex_stream.data[24 + 12..24 + 16])
        );
        assert_eq!(
            mesh.blend_weights_bytes(1),
            Some(&mesh.vertex_stream.data[24 + 16..24 + 24])
        );
    }

    #[test]
    fn rejects_index_outside_local_vertex_stream() {
        let mut data = synthetic_model();
        put_u16(&mut data, 0x80, 3);
        assert_eq!(parse_model(&data), Err(ModelParseError::MeshNotFound));
    }

    #[test]
    fn rejects_nonzero_vertex_stream_in_evidenced_layout() {
        let mut data = synthetic_model();
        let declaration = 0x88 + 24 * 3;
        put_u16(&mut data, declaration + 12, 1);
        assert_eq!(parse_model(&data), Err(ModelParseError::InvalidMesh));
    }

    #[test]
    fn rejects_wrong_mirage_version() {
        let mut data = synthetic_model();
        put_u32(&mut data, 4, 4);
        assert_eq!(
            parse_model(&data),
            Err(ModelParseError::UnsupportedVersion(4))
        );
    }
}

//! Minimal diffuse-material helpers for future textured terrain/model passes.
//!
//! Integration is intentionally opt-in: this module does not alter the existing terrain renderer.
//! The expected pipeline binding convention is:
//! - group 0: camera/world uniforms owned by the caller
//! - group 1: diffuse texture + filtering sampler from this module
//!
//! Vertex semantic locations follow the renderer's existing Mirage convention:
//! - location 0: position (float3)
//! - location 1: normal (float3), when present
//! - location 2: UV (float2), when present

pub const DIFFUSE_SHADER: &str = include_str!("diffuse.wgsl");

pub const POSITION_LOCATION: u32 = 0;
pub const NORMAL_LOCATION: u32 = 1;
pub const UV_LOCATION: u32 = 2;

const POSITION_STRIDE: wgpu::BufferAddress = 12;
const POSITION_NORMAL_STRIDE: wgpu::BufferAddress = 24;
const POSITION_UV_STRIDE: wgpu::BufferAddress = 20;
const POSITION_NORMAL_UV_STRIDE: wgpu::BufferAddress = 32;

const POSITION_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x3,
    offset: 0,
    shader_location: POSITION_LOCATION,
}];

const POSITION_NORMAL_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: POSITION_LOCATION,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 12,
        shader_location: NORMAL_LOCATION,
    },
];

const POSITION_UV_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: POSITION_LOCATION,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 12,
        shader_location: UV_LOCATION,
    },
];

const POSITION_NORMAL_UV_ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: POSITION_LOCATION,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 12,
        shader_location: NORMAL_LOCATION,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 24,
        shader_location: UV_LOCATION,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialVertexLayout {
    Position,
    PositionNormal,
    PositionUv,
    PositionNormalUv,
}

impl MaterialVertexLayout {
    pub const fn has_normal(self) -> bool {
        matches!(self, Self::PositionNormal | Self::PositionNormalUv)
    }

    pub const fn has_uv(self) -> bool {
        matches!(self, Self::PositionUv | Self::PositionNormalUv)
    }

    pub fn buffer_layout(self) -> wgpu::VertexBufferLayout<'static> {
        match self {
            Self::Position => wgpu::VertexBufferLayout {
                array_stride: POSITION_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &POSITION_ATTRIBUTES,
            },
            Self::PositionNormal => wgpu::VertexBufferLayout {
                array_stride: POSITION_NORMAL_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &POSITION_NORMAL_ATTRIBUTES,
            },
            Self::PositionUv => wgpu::VertexBufferLayout {
                array_stride: POSITION_UV_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &POSITION_UV_ATTRIBUTES,
            },
            Self::PositionNormalUv => wgpu::VertexBufferLayout {
                array_stride: POSITION_NORMAL_UV_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &POSITION_NORMAL_UV_ATTRIBUTES,
            },
        }
    }

    /// Vertex entry point in [`DIFFUSE_SHADER`] matching this layout.
    pub const fn vertex_entry_point(self) -> &'static str {
        match self {
            Self::Position => "vs_position",
            Self::PositionNormal => "vs_position_normal",
            Self::PositionUv => "vs_position_uv",
            Self::PositionNormalUv => "vs_position_normal_uv",
        }
    }
}

/// Creates the group-1 layout consumed by [`DIFFUSE_SHADER`]:
/// binding 0 = filterable 2D float texture, binding 1 = filtering sampler.
pub fn create_diffuse_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("diffuse-material-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// GPU material state for one diffuse texture.
///
/// The caller owns the texture/view. The sampler is retained here alongside the bind group so the
/// material is a self-contained binding object for a future terrain/model draw path.
pub struct DiffuseMaterial {
    bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
}

impl DiffuseMaterial {
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        diffuse_view: &wgpu::TextureView,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("diffuse-material-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self::with_sampler(device, layout, diffuse_view, sampler)
    }

    pub fn with_sampler(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        diffuse_view: &wgpu::TextureView,
        sampler: wgpu::Sampler,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse-material-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(diffuse_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            bind_group,
            sampler,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_layouts_match_semantic_contract() {
        let position = MaterialVertexLayout::Position.buffer_layout();
        assert_eq!(position.array_stride, 12);
        assert_eq!(position.attributes.len(), 1);
        assert_eq!(position.attributes[0].shader_location, POSITION_LOCATION);

        let position_normal = MaterialVertexLayout::PositionNormal.buffer_layout();
        assert_eq!(position_normal.array_stride, 24);
        assert_eq!(position_normal.attributes.len(), 2);
        assert_eq!(
            position_normal.attributes[1].shader_location,
            NORMAL_LOCATION
        );

        let position_uv = MaterialVertexLayout::PositionUv.buffer_layout();
        assert_eq!(position_uv.array_stride, 20);
        assert_eq!(position_uv.attributes.len(), 2);
        assert_eq!(position_uv.attributes[1].shader_location, UV_LOCATION);

        let position_normal_uv = MaterialVertexLayout::PositionNormalUv.buffer_layout();
        assert_eq!(position_normal_uv.array_stride, 32);
        assert_eq!(position_normal_uv.attributes.len(), 3);
        assert_eq!(
            position_normal_uv.attributes[2].shader_location,
            UV_LOCATION
        );
    }

    #[test]
    fn diffuse_wgsl_parses_and_validates_with_naga() {
        let module =
            wgpu::naga::front::wgsl::parse_str(DIFFUSE_SHADER).expect("diffuse WGSL should parse");

        let mut validator = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        );

        validator
            .validate(&module)
            .expect("diffuse WGSL should validate");
    }

    #[test]
    fn every_layout_has_a_shader_entry_point() {
        let module =
            wgpu::naga::front::wgsl::parse_str(DIFFUSE_SHADER).expect("diffuse WGSL should parse");

        for layout in [
            MaterialVertexLayout::Position,
            MaterialVertexLayout::PositionNormal,
            MaterialVertexLayout::PositionUv,
            MaterialVertexLayout::PositionNormalUv,
        ] {
            let entry = layout.vertex_entry_point();
            assert!(
                module.entry_points.iter().any(|candidate| {
                    candidate.stage == wgpu::naga::ShaderStage::Vertex && candidate.name == entry
                }),
                "missing vertex entry point {entry}"
            );
        }

        assert!(
            module.entry_points.iter().any(|candidate| {
                candidate.stage == wgpu::naga::ShaderStage::Fragment
                    && candidate.name == "fs_diffuse"
            }),
            "missing fragment entry point fs_diffuse"
        );
    }
}

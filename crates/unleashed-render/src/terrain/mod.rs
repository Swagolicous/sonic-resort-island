use std::error::Error;
use std::fmt;

use wgpu::util::DeviceExt;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const POSITION_STRIDE: wgpu::BufferAddress = 12;
const POSITION_NORMAL_STRIDE: wgpu::BufferAddress = 24;

const POSITION_ATTRIBUTES: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x3,
    offset: 0,
    shader_location: 0,
}];

const POSITION_NORMAL_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 12,
        shader_location: 1,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainVertexLayout {
    Position,
    PositionNormal,
}

impl TerrainVertexLayout {
    fn buffer_layout(self) -> wgpu::VertexBufferLayout<'static> {
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
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TerrainIndices<'a> {
    U16(&'a [u16]),
    U32(&'a [u32]),
}

impl TerrainIndices<'_> {
    fn len(self) -> usize {
        match self {
            Self::U16(indices) => indices.len(),
            Self::U32(indices) => indices.len(),
        }
    }

    fn format(self) -> wgpu::IndexFormat {
        match self {
            Self::U16(_) => wgpu::IndexFormat::Uint16,
            Self::U32(_) => wgpu::IndexFormat::Uint32,
        }
    }

    fn max_index(self) -> Option<u32> {
        match self {
            Self::U16(indices) => indices.iter().copied().max().map(u32::from),
            Self::U32(indices) => indices.iter().copied().max(),
        }
    }

    fn to_bytes(self) -> Vec<u8> {
        match self {
            Self::U16(indices) => {
                let mut bytes = Vec::with_capacity(indices.len() * 2);
                for index in indices {
                    bytes.extend_from_slice(&index.to_le_bytes());
                }
                bytes
            }
            Self::U32(indices) => {
                let mut bytes = Vec::with_capacity(indices.len() * 4);
                for index in indices {
                    bytes.extend_from_slice(&index.to_le_bytes());
                }
                bytes
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainMeshError {
    EmptyVertices,
    EmptyIndices,
    NormalCountMismatch { vertices: usize, normals: usize },
    IndexOutOfRange { max_index: u32, vertex_count: usize },
    TooManyVertices(usize),
    TooManyIndices(usize),
}

impl fmt::Display for TerrainMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVertices => write!(f, "terrain mesh has no vertices"),
            Self::EmptyIndices => write!(f, "terrain mesh has no indices"),
            Self::NormalCountMismatch { vertices, normals } => write!(
                f,
                "terrain normal count ({normals}) does not match vertex count ({vertices})"
            ),
            Self::IndexOutOfRange {
                max_index,
                vertex_count,
            } => write!(
                f,
                "terrain index {max_index} is outside vertex count {vertex_count}"
            ),
            Self::TooManyVertices(count) => {
                write!(f, "terrain vertex count {count} does not fit in u32")
            }
            Self::TooManyIndices(count) => {
                write!(f, "terrain index count {count} does not fit in u32")
            }
        }
    }
}

impl Error for TerrainMeshError {}

pub struct TerrainMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    index_count: u32,
    vertex_count: u32,
    layout: TerrainVertexLayout,
}

impl TerrainMesh {
    /// Uploads decoded host-endian XYZ positions using the authentic Mirage POSITION0
    /// shader location (location 0). The source renderer maps D3DDECLTYPE_FLOAT3 to
    /// three 32-bit components, i.e. a 12-byte position.
    pub fn from_positions(
        device: &wgpu::Device,
        positions: &[[f32; 3]],
        indices: TerrainIndices<'_>,
    ) -> Result<Self, TerrainMeshError> {
        validate_mesh(positions.len(), indices)?;

        let mut vertex_bytes = Vec::with_capacity(positions.len() * POSITION_STRIDE as usize);
        for position in positions {
            append_vec3(&mut vertex_bytes, *position);
        }

        Self::from_bytes(
            device,
            &vertex_bytes,
            positions.len(),
            TerrainVertexLayout::Position,
            indices,
        )
    }

    /// Uploads decoded host-endian XYZ position + XYZ normal vertices.
    ///
    /// This mirrors the source renderer's semantic mapping:
    /// POSITION0 -> shader location 0, NORMAL0 -> shader location 1.
    /// Both are exposed here as float3 values for the first-visible-geometry path.
    pub fn from_positions_normals(
        device: &wgpu::Device,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: TerrainIndices<'_>,
    ) -> Result<Self, TerrainMeshError> {
        if positions.len() != normals.len() {
            return Err(TerrainMeshError::NormalCountMismatch {
                vertices: positions.len(),
                normals: normals.len(),
            });
        }

        validate_mesh(positions.len(), indices)?;

        let mut vertex_bytes =
            Vec::with_capacity(positions.len() * POSITION_NORMAL_STRIDE as usize);
        for (position, normal) in positions.iter().zip(normals) {
            append_vec3(&mut vertex_bytes, *position);
            append_vec3(&mut vertex_bytes, *normal);
        }

        Self::from_bytes(
            device,
            &vertex_bytes,
            positions.len(),
            TerrainVertexLayout::PositionNormal,
            indices,
        )
    }

    fn from_bytes(
        device: &wgpu::Device,
        vertex_bytes: &[u8],
        vertex_count: usize,
        layout: TerrainVertexLayout,
        indices: TerrainIndices<'_>,
    ) -> Result<Self, TerrainMeshError> {
        let index_count = u32::try_from(indices.len())
            .map_err(|_| TerrainMeshError::TooManyIndices(indices.len()))?;
        let vertex_count = u32::try_from(vertex_count)
            .map_err(|_| TerrainMeshError::TooManyVertices(vertex_count))?;
        let index_format = indices.format();
        let index_bytes = indices.to_bytes();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain-vertices"),
            contents: vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain-indices"),
            contents: &index_bytes,
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_format,
            index_count,
            vertex_count,
            layout,
        })
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn vertex_layout(&self) -> TerrainVertexLayout {
        self.layout
    }

    pub fn index_format(&self) -> wgpu::IndexFormat {
        self.index_format
    }
}

fn validate_mesh(vertex_count: usize, indices: TerrainIndices<'_>) -> Result<(), TerrainMeshError> {
    if vertex_count == 0 {
        return Err(TerrainMeshError::EmptyVertices);
    }
    if indices.len() == 0 {
        return Err(TerrainMeshError::EmptyIndices);
    }

    if let Some(max_index) = indices.max_index() {
        if max_index as usize >= vertex_count {
            return Err(TerrainMeshError::IndexOutOfRange {
                max_index,
                vertex_count,
            });
        }
    }

    Ok(())
}

fn append_vec3(bytes: &mut Vec<u8>, value: [f32; 3]) {
    for component in value {
        bytes.extend_from_slice(&component.to_le_bytes());
    }
}

fn matrix_bytes(matrix: &[[f32; 4]; 4]) -> [u8; 64] {
    let mut bytes = [0_u8; 64];
    let mut offset = 0;

    for column in matrix {
        for component in column {
            bytes[offset..offset + 4].copy_from_slice(&component.to_le_bytes());
            offset += 4;
        }
    }

    bytes
}

fn identity_matrix() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

struct CameraUniform {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl CameraUniform {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let initial_matrix = matrix_bytes(&identity_matrix());
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain-camera"),
            contents: &initial_matrix,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain-camera-bind-group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self { buffer, bind_group }
    }

    fn update(&self, queue: &wgpu::Queue, view_projection_columns: [[f32; 4]; 4]) {
        let bytes = matrix_bytes(&view_projection_columns);
        queue.write_buffer(&self.buffer, 0, &bytes);
    }
}

pub struct TerrainDepthBuffer {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl TerrainDepthBuffer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain-depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _texture: texture,
            view,
            width,
            height,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.width == width && self.height == height {
            return;
        }

        *self = Self::new(device, width, height);
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

pub struct TerrainRenderer {
    position_pipeline: wgpu::RenderPipeline,
    position_normal_pipeline: wgpu::RenderPipeline,
    camera: CameraUniform,
    depth: TerrainDepthBuffer,
}

impl TerrainRenderer {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain-camera-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain-pipeline-layout"),
            bind_group_layouts: &[&camera_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("terrain.wgsl").into()),
        });

        let position_pipeline = create_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            TerrainVertexLayout::Position,
            "vs_position",
            "terrain-position-pipeline",
        );

        let position_normal_pipeline = create_pipeline(
            device,
            &pipeline_layout,
            &shader,
            color_format,
            TerrainVertexLayout::PositionNormal,
            "vs_position_normal",
            "terrain-position-normal-pipeline",
        );

        let camera = CameraUniform::new(device, &camera_layout);
        let depth = TerrainDepthBuffer::new(device, width, height);

        Self {
            position_pipeline,
            position_normal_pipeline,
            camera,
            depth,
        }
    }

    /// Updates the world-to-clip matrix consumed by WGSL.
    ///
    /// The four outer arrays are matrix columns, matching WGSL mat4x4<f32>
    /// construction/storage convention.
    pub fn update_camera(&self, queue: &wgpu::Queue, view_projection_columns: [[f32; 4]; 4]) {
        self.camera.update(queue, view_projection_columns);
    }

    pub fn resize_depth(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth.resize(device, width, height);
    }

    pub fn depth(&self) -> &TerrainDepthBuffer {
        &self.depth
    }

    pub fn depth_attachment(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.depth.view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    /// Binds the appropriate terrain pipeline and issues one indexed triangle-list draw.
    ///
    /// The caller owns the render pass so terrain can be integrated into Renderer without
    /// imposing frame ownership or color-target policy.
    pub fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>, mesh: &'pass TerrainMesh) {
        let pipeline = match mesh.layout {
            TerrainVertexLayout::Position => &self.position_pipeline,
            TerrainVertexLayout::PositionNormal => &self.position_normal_pipeline,
        };

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.camera.bind_group, &[]);
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_format);
        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    vertex_layout: TerrainVertexLayout,
    vertex_entry: &'static str,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let buffer_layout = vertex_layout.buffer_layout();

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[buffer_layout],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

pub mod terrain;

use anyhow::{Context, Result};
use std::sync::Arc;
use terrain::{TerrainIndices, TerrainMesh, TerrainMeshError, TerrainRenderer};
use unleashed_platform::FrameDriver;
use winit::window::Window;

#[derive(Debug, Clone, Copy)]
pub enum TerrainCpuMesh<'a> {
    Positions {
        positions: &'a [[f32; 3]],
        indices: TerrainIndices<'a>,
    },
    PositionsNormals {
        positions: &'a [[f32; 3]],
        normals: &'a [[f32; 3]],
        indices: TerrainIndices<'a>,
    },
}

pub struct Renderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    terrain_renderer: Option<TerrainRenderer>,
    terrain_meshes: Vec<TerrainMesh>,
    terrain_draw_call_count: u32,
}

impl Renderer {
    pub fn new(window: Window) -> Result<Self> {
        pollster::block_on(Self::new_async(window))
    }

    async fn new_async(window: Window) -> Result<Self> {
        let window = Arc::new(window);
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .context("create wgpu surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .context("request GPU adapter")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("unleashed-device"),
                ..Default::default()
            })
            .await
            .context("request GPU device")?;

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        window.request_redraw();

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            terrain_renderer: None,
            terrain_meshes: Vec::new(),
            terrain_draw_call_count: 0,
        })
    }

    fn ensure_terrain_renderer(&mut self) {
        if self.terrain_renderer.is_none() {
            self.terrain_renderer = Some(TerrainRenderer::new(
                &self.device,
                self.config.format,
                self.config.width,
                self.config.height,
            ));
        }
    }

    /// Replaces the entire terrain stage from decoded host-endian CPU meshes.
    ///
    /// Positions may already have their per-instance transform baked in. All GPU uploads
    /// must succeed before the existing stage is replaced; passing an empty slice clears it.
    pub fn replace_terrain_stage(
        &mut self,
        meshes: &[TerrainCpuMesh<'_>],
    ) -> Result<(), TerrainMeshError> {
        let mut uploaded = Vec::with_capacity(meshes.len());

        for mesh in meshes {
            uploaded.push(match *mesh {
                TerrainCpuMesh::Positions { positions, indices } => {
                    TerrainMesh::from_positions(&self.device, positions, indices)?
                }
                TerrainCpuMesh::PositionsNormals {
                    positions,
                    normals,
                    indices,
                } => {
                    TerrainMesh::from_positions_normals(&self.device, positions, normals, indices)?
                }
            });
        }

        if !uploaded.is_empty() {
            self.ensure_terrain_renderer();
        }

        self.terrain_meshes = uploaded;
        self.terrain_draw_call_count = 0;
        self.window.request_redraw();
        Ok(())
    }

    /// Uploads already-decoded host-endian terrain positions and indices.
    pub fn set_terrain_positions(
        &mut self,
        positions: &[[f32; 3]],
        indices: TerrainIndices<'_>,
    ) -> Result<(), TerrainMeshError> {
        self.replace_terrain_stage(&[TerrainCpuMesh::Positions { positions, indices }])
    }

    /// Uploads already-decoded host-endian terrain positions, normals, and indices.
    pub fn set_terrain_positions_normals(
        &mut self,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: TerrainIndices<'_>,
    ) -> Result<(), TerrainMeshError> {
        self.replace_terrain_stage(&[TerrainCpuMesh::PositionsNormals {
            positions,
            normals,
            indices,
        }])
    }

    pub fn clear_terrain(&mut self) {
        self.terrain_meshes.clear();
        self.terrain_draw_call_count = 0;
        self.window.request_redraw();
    }

    pub fn terrain_mesh_count(&self) -> usize {
        self.terrain_meshes.len()
    }

    /// Number of terrain draw calls issued by the last successfully presented frame.
    pub fn terrain_draw_call_count(&self) -> u32 {
        self.terrain_draw_call_count
    }

    /// Updates the terrain world-to-clip matrix. Outer arrays are matrix columns.
    pub fn update_terrain_camera(&mut self, view_projection_columns: [[f32; 4]; 4]) {
        self.ensure_terrain_renderer();
        self.terrain_renderer
            .as_ref()
            .expect("terrain renderer was initialized above")
            .update_camera(&self.queue, view_projection_columns);
        self.window.request_redraw();
    }
}

impl FrameDriver for Renderer {
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        if let Some(terrain_renderer) = &mut self.terrain_renderer {
            terrain_renderer.resize_depth(&self.device, width, height);
        }
        self.window.request_redraw();
    }

    fn render(&mut self) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                self.window.request_redraw();
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("unleashed-frame"),
            });

        {
            let depth_stencil_attachment = self
                .terrain_renderer
                .as_ref()
                .filter(|_| !self.terrain_meshes.is_empty())
                .map(TerrainRenderer::depth_attachment);

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("unleashed-main-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.025,
                            g: 0.04,
                            b: 0.07,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(terrain_renderer) = &self.terrain_renderer {
                for terrain_mesh in &self.terrain_meshes {
                    terrain_renderer.draw(&mut pass, terrain_mesh);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.terrain_draw_call_count = u32::try_from(self.terrain_meshes.len()).unwrap_or(u32::MAX);
        self.window.request_redraw();
        Ok(())
    }
}

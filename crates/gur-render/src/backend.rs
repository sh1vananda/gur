//! Full wgpu backend — surface, device, sprite batcher, canvas upscale.
//!
//! ## Frame flow
//! ```text
//! begin_frame()  → clear draw queue, get surface texture
//! draw_sprite()  → push to draw queue
//! end_frame()    → sort queue → build vertex buffer → render pass → present
//! ```

use std::sync::Arc;

use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::window::Window;

use gur_assets::HandleId;

use crate::{
    CANVAS_WIDTH, CANVAS_HEIGHT,
    sprite::SpriteDrawCall,
    pipeline::{
        self, CameraUniform, SpriteVertex,
        camera_bgl, texture_bgl, upscale_bgl,
        create_sprite_pipeline, create_upscale_pipeline,
    },
    gpu_texture::GpuTextureCache,
};

/// Maximum sprites per frame. Pre-allocates vertex/index buffer.
const MAX_SPRITES: usize = 4096;

/// The wgpu renderer backend. Constructed once, owns all GPU state.
pub struct WgpuBackend {
    // ── wgpu core ─────────────────────────────────────────────────────────
    pub device:  wgpu::Device,
    pub queue:   wgpu::Queue,
    surface:     wgpu::Surface<'static>,
    surface_cfg: wgpu::SurfaceConfiguration,
    surface_fmt: wgpu::TextureFormat,

    // ── Canvas (320×180 pixel-art render target) ──────────────────────────
    canvas_texture:     wgpu::Texture,
    canvas_view:        wgpu::TextureView,
    canvas_format:      wgpu::TextureFormat,

    // ── Pipelines & layouts ───────────────────────────────────────────────
    sprite_pipeline:   wgpu::RenderPipeline,
    upscale_pipeline:  wgpu::RenderPipeline,
    camera_bgl:        wgpu::BindGroupLayout,
    texture_bgl:       wgpu::BindGroupLayout,
    upscale_bgl:       wgpu::BindGroupLayout,

    // ── Per-frame GPU buffers ─────────────────────────────────────────────
    vertex_buffer:  wgpu::Buffer,
    index_buffer:   wgpu::Buffer,
    camera_buffer:  wgpu::Buffer,
    camera_bg:      wgpu::BindGroup,
    upscale_bg:     wgpu::BindGroup,

    // ── Texture cache & draw queue ────────────────────────────────────────
    textures:    GpuTextureCache,
    draw_queue:  Vec<SpriteDrawCall>,

    // ── Window size ───────────────────────────────────────────────────────
    window_size: (u32, u32),
}

impl WgpuBackend {
    /// Initialise the entire wgpu stack from a winit window.
    ///
    /// Blocks the calling thread via `pollster::block_on` — call from the
    /// main thread during `ApplicationHandler::resumed()`.
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let (w, h) = (size.width.max(1), size.height.max(1));

        // ── Instance & surface ────────────────────────────────────────────
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone())?;

        // ── Adapter ───────────────────────────────────────────────────────
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).ok_or_else(|| anyhow::anyhow!("No suitable GPU adapter found"))?;

        log::info!("GPU adapter: {}", adapter.get_info().name);

        // ── Device & queue ────────────────────────────────────────────────
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("gur_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        ))?;

        // ── Surface configuration ─────────────────────────────────────────
        let caps        = surface.get_capabilities(&adapter);
        let surface_fmt = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_cfg = wgpu::SurfaceConfiguration {
            usage:        wgpu::TextureUsages::RENDER_ATTACHMENT,
            format:       surface_fmt,
            width:        w,
            height:       h,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode:   caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_cfg);

        // ── Pixel-art canvas texture (320×180) ────────────────────────────
        let canvas_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let (canvas_texture, canvas_view) =
            Self::create_canvas_texture(&device, canvas_format);

        // ── Bind group layouts ────────────────────────────────────────────
        let camera_bgl  = camera_bgl(&device);
        let texture_bgl = texture_bgl(&device);
        let upscale_bgl = upscale_bgl(&device);

        // ── Pipelines ─────────────────────────────────────────────────────
        let sprite_pipeline  = create_sprite_pipeline(&device, &camera_bgl, &texture_bgl, canvas_format);
        let upscale_pipeline = create_upscale_pipeline(&device, &upscale_bgl, surface_fmt);

        // ── Camera uniform buffer ─────────────────────────────────────────
        let cam_uniform = CameraUniform::ortho_canvas(CANVAS_WIDTH as f32, CANVAS_HEIGHT as f32);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::bytes_of(&cam_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // ── Vertex & index buffers ────────────────────────────────────────
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_vb"),
            size: (MAX_SPRITES * 4 * std::mem::size_of::<SpriteVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Build a static index buffer: [0,1,2, 1,3,2] repeated MAX_SPRITES times
        let indices: Vec<u16> = (0..MAX_SPRITES as u16).flat_map(|s| {
            let b = s * 4;
            [b, b+1, b+2, b+1, b+3, b+2]
        }).collect();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_ib"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // ── Texture cache (includes 1×1 white fallback) ───────────────────
        let mut textures = GpuTextureCache::new(&device, &queue, &texture_bgl);

        // ── Upscale bind group (canvas → surface) ─────────────────────────
        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("canvas_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let upscale_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upscale_bg"),
            layout: &upscale_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&nearest_sampler),
                },
            ],
        });

        log::info!("WgpuBackend ready — {}×{} window, canvas {}×{}, surface format {:?}",
            w, h, CANVAS_WIDTH, CANVAS_HEIGHT, surface_fmt);

        Ok(Self {
            device, queue, surface, surface_cfg, surface_fmt,
            canvas_texture, canvas_view, canvas_format,
            sprite_pipeline, upscale_pipeline,
            camera_bgl, texture_bgl, upscale_bgl,
            vertex_buffer, index_buffer,
            camera_buffer, camera_bg, upscale_bg,
            textures,
            draw_queue: Vec::with_capacity(512),
            window_size: (w, h),
        })
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Clear the draw queue for the new frame.
    pub fn begin_frame(&mut self) {
        self.draw_queue.clear();
    }

    /// Queue a sprite draw call.
    pub fn draw_sprite(&mut self, call: SpriteDrawCall) {
        if self.draw_queue.len() < MAX_SPRITES {
            self.draw_queue.push(call);
        }
    }

    /// Upload a texture (RGBA8) and associate it with a handle ID.
    pub fn upload_texture(&mut self, id: HandleId, data: &[u8], w: u32, h: u32) {
        if !self.textures.contains(id) {
            self.textures.upload_rgba8(
                &self.device, &self.queue, &self.texture_bgl,
                data, w, h, id,
            );
        }
    }

    /// Update the camera matrix (call when the game camera moves).
    pub fn update_camera(&self, view_proj: Mat4) {
        let uniform = CameraUniform::from_mat4(view_proj);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Flush draw queue, render canvas, upscale to window surface, present.
    pub fn end_frame(&mut self) {
        // Sort by z then texture to minimise state changes
        self.draw_queue.sort_by(|a, b| {
            a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.texture.cmp(&b.texture))
        });

        // Build vertex data
        let mut vertices: Vec<SpriteVertex> = Vec::with_capacity(self.draw_queue.len() * 4);
        for call in &self.draw_queue {
            let tex   = self.textures.get(call.texture);
            let tw    = tex.width  as f32;
            let th    = tex.height as f32;

            let (sx, sy, sw, sh) = if let Some(r) = call.source {
                (r.x / tw, r.y / th, r.w / tw, r.h / th)
            } else {
                (0.0_f32, 0.0_f32, 1.0_f32, 1.0_f32)
            };

            let (u0, u1, v0, v1) = match call.flip {
                crate::sprite::SpriteFlip::None       => (sx,       sx+sw, sy,      sy+sh),
                crate::sprite::SpriteFlip::Horizontal => (sx+sw,    sx,    sy,      sy+sh),
                crate::sprite::SpriteFlip::Vertical   => (sx,       sx+sw, sy+sh,   sy),
                crate::sprite::SpriteFlip::Both       => (sx+sw,    sx,    sy+sh,   sy),
            };

            let half_w = call.size.x * 0.5;
            let half_h = call.size.y * 0.5;
            let (px, py) = (call.position.x, call.position.y);
            let c = call.color.to_array();

            // TL, TR, BL, BR
            vertices.push(SpriteVertex { position: [px - half_w, py - half_h], uv: [u0, v0], color: c });
            vertices.push(SpriteVertex { position: [px + half_w, py - half_h], uv: [u1, v0], color: c });
            vertices.push(SpriteVertex { position: [px - half_w, py + half_h], uv: [u0, v1], color: c });
            vertices.push(SpriteVertex { position: [px + half_w, py + half_h], uv: [u1, v1], color: c });
        }

        if !vertices.is_empty() {
            self.queue.write_buffer(
                &self.vertex_buffer, 0,
                bytemuck::cast_slice(&vertices),
            );
        }

        // Get surface frame
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_cfg);
                return;
            }
            Err(e) => { log::error!("Surface error: {e}"); return; }
        };

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder  = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("frame_encoder") }
        );

        // ── Pass 1: Sprite → Canvas ───────────────────────────────────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.canvas_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.05, b: 0.1, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_bind_group(0, &self.camera_bg, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // Group draws by texture — bind group changes are the expensive ops
            let mut current_tex: Option<HandleId> = None;
            for (i, call) in self.draw_queue.iter().enumerate() {
                if current_tex != Some(call.texture) {
                    let bg = &self.textures.get(call.texture).bind_group;
                    pass.set_bind_group(1, bg, &[]);
                    current_tex = Some(call.texture);
                }
                let idx_start = (i as u32) * 6;
                pass.draw_indexed(idx_start..idx_start + 6, 0, 0..1);
            }
        }

        // ── Pass 2: Canvas → Window surface (upscale) ─────────────────────
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("upscale_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.upscale_pipeline);
            pass.set_bind_group(0, &self.upscale_bg, &[]);
            pass.draw(0..3, 0..1); // full-screen triangle — no vertex buffer
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    /// Resize the surface after a window resize event.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        self.surface_cfg.width  = width;
        self.surface_cfg.height = height;
        self.surface.configure(&self.device, &self.surface_cfg);
        self.window_size = (width, height);
        log::debug!("WgpuBackend resized to {width}×{height}");
    }

    /// Current window physical size.
    pub fn window_size(&self) -> (u32, u32) { self.window_size }

    // ── Private helpers ───────────────────────────────────────────────────

    fn create_canvas_texture(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("canvas"),
            size: wgpu::Extent3d {
                width: CANVAS_WIDTH,
                height: CANVAS_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

//! GPU texture management — upload CPU images to wgpu, manage bind groups.

use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

use gur_assets::HandleId;

/// A GPU-uploaded texture and its cached bind group.
pub struct GpuTexture {
    /// The wgpu texture object.
    pub texture: wgpu::Texture,
    /// View for sampling.
    pub view: wgpu::TextureView,
    /// Bind group bound at group(1) in the sprite pipeline.
    pub bind_group: wgpu::BindGroup,
    /// Pixel dimensions.
    pub width: u32,
    pub height: u32,
}

/// Cache of all GPU-uploaded textures keyed by [`HandleId`].
pub struct GpuTextureCache {
    textures: HashMap<HandleId, GpuTexture>,
    /// The 1×1 white fallback texture used when no texture is found.
    fallback_id: HandleId,
}

impl GpuTextureCache {
    /// Create a new cache and upload the fallback white pixel texture.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let mut cache = Self {
            textures: HashMap::new(),
            fallback_id: 0,
        };
        // Upload a 1×1 opaque white texture as the fallback
        let white = cache.upload_rgba8(
            device, queue, texture_bgl,
            &[255u8, 255, 255, 255],
            1, 1,
            0, // handle id 0 = fallback
        );
        cache
    }

    /// Upload raw RGBA8 bytes as a GPU texture, bound at `handle_id`.
    pub fn upload_rgba8(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bgl: &wgpu::BindGroupLayout,
        data: &[u8],
        width: u32,
        height: u32,
        handle_id: HandleId,
    ) -> &GpuTexture {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("sprite_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // pixel-art crispness
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.textures.insert(handle_id, GpuTexture {
            texture, view, bind_group, width, height,
        });

        self.textures.get(&handle_id).unwrap()
    }

    /// Get a texture by handle ID. Returns fallback white if not found.
    pub fn get(&self, handle_id: HandleId) -> &GpuTexture {
        self.textures.get(&handle_id)
            .or_else(|| self.textures.get(&self.fallback_id))
            .expect("fallback texture must always exist")
    }

    /// Returns true if a texture with this handle_id is already uploaded.
    pub fn contains(&self, handle_id: HandleId) -> bool {
        self.textures.contains_key(&handle_id)
    }
}

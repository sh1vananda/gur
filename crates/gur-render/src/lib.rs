//! # gur-render
//!
//! The wgpu-based pixel-art renderer for the GUR engine.
//!
//! ## Pipeline
//! 1. Game systems call [`WgpuBackend::draw_sprite`] each frame.
//! 2. `end_frame()` batches and renders all sprites to the **320×180 canvas**.
//! 3. The canvas is upscaled to the native window via a nearest-neighbour pass.

#![warn(missing_docs)]

pub mod backend;
pub mod camera;
pub mod color;
pub mod gpu_texture;
pub mod pipeline;
pub mod sprite;

pub use backend::WgpuBackend;
// Note: RenderBackend trait removed — WgpuBackend is used directly.
pub use camera::Camera2D;
pub use color::Color;
pub use sprite::{Sprite, SpriteFlip, SpriteDrawCall, Rect};
pub use pipeline::{CameraUniform, SpriteVertex};

/// Internal canvas width (retro pixel-art target).
pub const CANVAS_WIDTH: u32 = 320;
/// Internal canvas height.
pub const CANVAS_HEIGHT: u32 = 180;

/// Plugin that inserts the [`Camera2D`] resource.
/// The actual `WgpuBackend` is created by the platform layer, not this plugin.
pub mod plugin {
    use gur_core::prelude::*;

    /// Inserts the 2D camera resource. Backend is owned by the platform.
    pub struct RenderPlugin {
        /// Window title (set on the winit window by the platform layer).
        pub title: String,
        /// Desired window width in screen pixels.
        pub window_width: u32,
        /// Desired window height in screen pixels.
        pub window_height: u32,
    }

    impl Default for RenderPlugin {
        fn default() -> Self {
            Self {
                title: "GUR".to_string(),
                window_width: 1280,
                window_height: 720,
            }
        }
    }

    impl Plugin for RenderPlugin {
        fn name(&self) -> &'static str { "gur_render" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            world.insert_resource(super::Camera2D::default());
            log::info!(
                "RenderPlugin — canvas {}×{}, window {}×{}",
                super::CANVAS_WIDTH, super::CANVAS_HEIGHT,
                self.window_width, self.window_height
            );
            Ok(())
        }
    }
}

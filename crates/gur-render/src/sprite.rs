//! Sprite component and draw call types.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use gur_assets::HandleId;

/// A simple axis-aligned rectangle in 2D space (used for UV source rects).
/// Glam does not expose a `Rect` — we define our own minimal version.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Left edge (U origin in pixels).
    pub x: f32,
    /// Top edge (V origin in pixels).
    pub y: f32,
    /// Width in pixels.
    pub w: f32,
    /// Height in pixels.
    pub h: f32,
}

impl Rect {
    /// Construct from origin + size.
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// Which axes to flip a sprite on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpriteFlip {
    /// No flip.
    #[default]
    None,
    /// Mirror horizontally.
    Horizontal,
    /// Mirror vertically.
    Vertical,
    /// Mirror both.
    Both,
}

/// ECS component that makes an entity visible as a sprite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    /// Handle ID of the texture to use (from `AssetServer`).
    pub texture: HandleId,
    /// Source rectangle in the texture (UV in pixels). `None` = full texture.
    pub source_rect: Option<Rect>,
    /// Tint color multiplied with the sprite pixels.
    pub color: Color,
    /// Flip state.
    pub flip: SpriteFlip,
    /// Size override in world units. `None` = use texture pixel dimensions.
    pub size: Option<Vec2>,
    /// Z-depth for painter's algorithm sorting.
    pub z_index: f32,
    /// Whether this sprite is currently visible.
    pub visible: bool,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            texture: 0,
            source_rect: None,
            color: Color::WHITE,
            flip: SpriteFlip::None,
            size: None,
            z_index: 0.0,
            visible: true,
        }
    }
}

impl Sprite {
    /// Create a basic sprite from a texture handle ID.
    pub fn from_texture(texture: HandleId) -> Self {
        Self { texture, ..Default::default() }
    }

    /// Set the source rect for sprite sheet / atlas sampling.
    pub fn with_source(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.source_rect = Some(Rect::new(x, y, w, h));
        self
    }

    /// Set the tint color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Flip horizontally.
    pub fn flipped(mut self) -> Self {
        self.flip = SpriteFlip::Horizontal;
        self
    }
}

/// A single prepared sprite draw call, ready for the GPU batcher.
#[derive(Debug, Clone)]
pub struct SpriteDrawCall {
    /// Texture to bind.
    pub texture: HandleId,
    /// Position in canvas space (pixels).
    pub position: Vec2,
    /// Size in canvas pixels.
    pub size: Vec2,
    /// Source UV rect in texture pixels.
    pub source: Option<Rect>,
    /// Tint.
    pub color: Color,
    /// Flip flags.
    pub flip: SpriteFlip,
    /// Z depth for sorting.
    pub z: f32,
}

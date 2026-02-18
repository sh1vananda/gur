//! Collider component — defines an entity's physical shape for collision.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// The geometric shape of a collider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColliderShape {
    /// Axis-aligned bounding box. Most efficient for pixel art entities.
    Aabb {
        /// Half-extents: the collider extends ±width from center on X, ±height on Y.
        half_extents: Vec2,
    },
    /// Circle collider. Good for projectiles, area effects.
    Circle {
        /// Radius in world units (pixels).
        radius: f32,
    },
    /// Convex polygon (for complex hitboxes). Points are in local space.
    Convex {
        /// Vertices of the convex polygon in counter-clockwise order.
        vertices: Vec<Vec2>,
    },
}

impl ColliderShape {
    /// Shorthand: create an AABB half-extents from width/height.
    pub fn aabb(w: f32, h: f32) -> Self {
        Self::Aabb { half_extents: Vec2::new(w * 0.5, h * 0.5) }
    }

    /// Shorthand: create a circle with the given radius.
    pub fn circle(radius: f32) -> Self {
        Self::Circle { radius }
    }
}

/// Collider ECS component — attach to any entity that needs physics.
///
/// Offset allows the collider to be positioned relative to the entity's `Transform`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collider {
    /// The geometric shape.
    pub shape: ColliderShape,
    /// Local-space offset from the entity's `Transform.translation`.
    pub offset: Vec2,
    /// Whether this collider is solid (blocks movement) or a sensor (triggers only).
    pub is_sensor: bool,
    /// Collision layer mask (bitfield). Colliders only interact with matching layers.
    pub layer: u32,
    /// Which layers this collider can collide with.
    pub mask: u32,
}

impl Collider {
    /// Create a solid AABB collider with the default collision layer.
    pub fn aabb(w: f32, h: f32) -> Self {
        Self {
            shape: ColliderShape::aabb(w, h),
            offset: Vec2::ZERO,
            is_sensor: false,
            layer: 0x01,
            mask: 0xFF,
        }
    }

    /// Create a sensor AABB (trigger zone, not solid).
    pub fn sensor_aabb(w: f32, h: f32) -> Self {
        Self {
            is_sensor: true,
            ..Self::aabb(w, h)
        }
    }

    /// Set the local offset.
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Set collision layer and mask.
    pub fn with_layers(mut self, layer: u32, mask: u32) -> Self {
        self.layer = layer;
        self.mask = mask;
        self
    }
}

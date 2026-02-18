//! The `Transform` component — position, rotation, and scale in world space.
//!
//! Every visible or physical entity in the game world carries a `Transform`.
//! The engine's render and physics systems read this component; game systems
//! write to it. No other data lives here — pure spatial state.

use glam::{Mat4, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// World-space transform for a game entity.
///
/// ## Coordinate System
/// - X axis: right
/// - Y axis: up  
/// - Z axis: out of screen (used for layer/depth ordering in 2D)
/// - 1 unit = 1 pixel at base resolution (configurable via camera zoom)
///
/// ## LOAD-BEARING
/// The unit definition (1 unit = 1 pixel) must never change after content exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// World-space position. Use `.z` for layer depth (higher = in front).
    pub translation: Vec3,
    /// Rotation quaternion. For pure 2D, keep x/y at 0 and use `.z`/`.w`.
    pub rotation: Quat,
    /// Non-uniform scale. `Vec3::ONE` is normal size.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    /// A transform at the origin with no rotation and unit scale.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Create a transform at a 2D position (Z defaults to 0.0).
    #[inline]
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, 0.0),
            ..Self::IDENTITY
        }
    }

    /// Create a transform at a 2D position with an explicit depth layer.
    #[inline]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            ..Self::IDENTITY
        }
    }

    /// Create a transform with a uniform scale factor.
    #[inline]
    pub fn with_scale(mut self, s: f32) -> Self {
        self.scale = Vec3::splat(s);
        self
    }

    /// Create a transform with a 2D rotation (radians, counter-clockwise).
    #[inline]
    pub fn with_rotation_2d(mut self, radians: f32) -> Self {
        self.rotation = Quat::from_rotation_z(radians);
        self
    }

    /// The 2D position, ignoring the depth (Z) component.
    #[inline]
    pub fn position_2d(&self) -> Vec2 {
        self.translation.truncate()
    }

    /// Compute the full 4×4 model matrix (for passing to the GPU).
    #[inline]
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Translate by a 2D offset (does not affect Z).
    #[inline]
    pub fn translate_2d(&mut self, delta: Vec2) {
        self.translation.x += delta.x;
        self.translation.y += delta.y;
    }

    /// Flip the sprite horizontally (negate X scale).
    #[inline]
    pub fn flip_x(&mut self) {
        self.scale.x = -self.scale.x.abs();
    }

    /// Unflip the sprite horizontally.
    #[inline]
    pub fn unflip_x(&mut self) {
        self.scale.x = self.scale.x.abs();
    }

    /// Returns `true` if the sprite is currently flipped on the X axis.
    #[inline]
    pub fn is_flipped_x(&self) -> bool {
        self.scale.x < 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_matrix_is_identity() {
        let t = Transform::IDENTITY;
        let m = t.matrix();
        assert!((m - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-6));
    }

    #[test]
    fn position_2d_ignores_z() {
        let t = Transform::from_xyz(3.0, 4.0, 99.0);
        assert_eq!(t.position_2d(), Vec2::new(3.0, 4.0));
    }

    #[test]
    fn flip_x_toggles() {
        let mut t = Transform::IDENTITY;
        assert!(!t.is_flipped_x());
        t.flip_x();
        assert!(t.is_flipped_x());
        t.unflip_x();
        assert!(!t.is_flipped_x());
    }
}

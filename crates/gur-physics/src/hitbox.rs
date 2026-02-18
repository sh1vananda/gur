//! Hitbox/hurtbox system — the souls-like combat hitbox layer.

use glam::Vec2;
use hecs::Entity;
use serde::{Deserialize, Serialize};

use crate::collider::ColliderShape;

/// What role this hitbox plays in combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitboxKind {
    /// An attack hitbox — this entity is dealing damage.
    Attack,
    /// A hurtbox — this entity can receive damage.
    Hurt,
    /// A pushbox — prevents entity overlap.
    Push,
}

/// Hitbox ECS component.
#[derive(Debug, Clone)]
pub struct Hitbox {
    pub kind:              HitboxKind,
    pub shape:             ColliderShape,
    pub offset:            Vec2,
    pub owner:             Entity,
    pub damage:            u32,
    pub knockback:         Vec2,
    pub poise_damage:      u32,
    pub active_frames:     u32,
    pub frames_remaining:  u32,
}

impl Hitbox {
    /// Create a standard attack hitbox.
    pub fn attack(owner: Entity, w: f32, h: f32, damage: u32) -> Self {
        Self {
            kind: HitboxKind::Attack,
            shape: ColliderShape::aabb(w, h),
            offset: Vec2::ZERO,
            owner,
            damage,
            knockback: Vec2::ZERO,
            poise_damage: damage / 2,
            active_frames: 6,
            frames_remaining: 6,
        }
    }

    /// Create a standard hurtbox (receives damage).
    pub fn hurtbox(owner: Entity, w: f32, h: f32) -> Self {
        Self {
            kind: HitboxKind::Hurt,
            shape: ColliderShape::aabb(w, h),
            offset: Vec2::ZERO,
            owner,
            damage: 0,
            knockback: Vec2::ZERO,
            poise_damage: 0,
            active_frames: 0,
            frames_remaining: 0,
        }
    }

    /// Returns `true` if this hitbox should be auto-despawned.
    #[inline]
    pub fn should_despawn(&self) -> bool {
        self.active_frames > 0 && self.frames_remaining == 0
    }

    /// Decrement the frame counter. Returns `true` if expired.
    #[inline]
    pub fn tick(&mut self) -> bool {
        if self.active_frames > 0 {
            self.frames_remaining = self.frames_remaining.saturating_sub(1);
            self.frames_remaining == 0
        } else {
            false
        }
    }
}

/// Marker component — disables hurtbox (i-frames during dodge).
#[derive(Debug, Clone, Copy, Default)]
pub struct HurtboxDisabled;

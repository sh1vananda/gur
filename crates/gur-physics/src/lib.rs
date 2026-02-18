//! # gur-physics
//!
//! 2D physics, collision detection, and the **hitbox system** for the GUR engine.
//!
//! ## Architecture
//! - Wraps `rapier2d` for broad-phase/narrow-phase collision
//! - Provides a thin **hitbox layer** (attack boxes, hurt boxes, push boxes)
//!   that game systems create/destroy per-frame for souls-like combat
//! - Physics objects (rigid bodies, colliders) are stored as ECS components
//!
//! ## Hitbox Philosophy (souls-like)
//! Attack hitboxes are **ephemeral** — spawned for a swing, destroyed after.
//! Hurt boxes persist while an entity is vulnerable.
//! I-frame support: hurtbox disabled during dodge roll frames.

#![warn(missing_docs)]

pub mod collider;
pub mod hitbox;
pub mod world;

pub use collider::{Collider, ColliderShape};
pub use hitbox::{Hitbox, HitboxKind, HurtboxDisabled};
pub use world::PhysicsWorld;

/// Plugin that registers physics resources and systems.
pub mod plugin {
    use gur_core::prelude::*;

    /// Registers the [`PhysicsWorld`] and physics step systems.
    pub struct PhysicsPlugin {
        /// Fixed step rate (default: 60 Hz).
        pub timestep: f32,
        /// Gravity vector (default: zero for top-down game).
        pub gravity: Vec2,
    }

    impl Default for PhysicsPlugin {
        fn default() -> Self {
            Self {
                timestep: 1.0 / 60.0,
                gravity: Vec2::ZERO, // top-down — no gravity
            }
        }
    }

    impl Plugin for PhysicsPlugin {
        fn name(&self) -> &'static str { "gur_physics" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            let phys = super::PhysicsWorld::new(self.gravity, self.timestep);
            world.insert_resource(phys);
            log::info!(
                "PhysicsPlugin ready — gravity={:?}, step={}Hz",
                self.gravity,
                (1.0 / self.timestep) as u32
            );
            Ok(())
        }
    }
}

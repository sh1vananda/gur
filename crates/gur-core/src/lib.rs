//! # gur-core
//!
//! The foundational layer of the GUR game engine.
//!
//! This crate contains:
//! - **ECS World** — entity/component storage via `hecs`
//! - **Scheduler** — system dependency ordering and parallel execution
//! - **Event Bus** — typed, decoupled message passing
//! - **Time** — delta-time, fixed-step accumulator, game clock
//! - **Transform** — spatial component shared across all engine crates
//! - **Plugin** — composable engine configuration trait
//!
//! ## Design Contract
//! This crate has **zero** knowledge of game logic. It provides primitives only.
//! Game systems register into the scheduler; game data registers into the ECS world.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod ecs;
pub mod error;
pub mod event;
pub mod plugin;
pub mod scheduler;
pub mod time;
pub mod transform;

// Re-export hecs as the authoritative ECS interface for the whole engine.
// All crates use `gur_core::hecs::*` rather than depending on hecs directly.
pub use hecs;
pub use glam;

/// Convenience prelude — `use gur_core::prelude::*;` in every engine crate.
pub mod prelude {
    pub use crate::ecs::{GurWorld, EntityId};
    pub use crate::error::{EngineError, EngineResult};
    pub use crate::event::{EventBus, EventReader, EventWriter};
    pub use crate::plugin::Plugin;
    pub use crate::scheduler::{Schedule, SystemStage, SystemFn};
    pub use crate::time::{Time, FixedTime};
    pub use crate::transform::Transform;
    pub use glam::{Vec2, Vec3, Quat, Mat4};
    pub use hecs::{Entity, World, Query, QueryBorrow};
}

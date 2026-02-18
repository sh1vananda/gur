//! Combat events — emitted into the `EventBus` by the effect pipeline.

use hecs::Entity;

/// Emitted when an entity receives damage.
#[derive(Debug, Clone)]
pub struct DamageEvent {
    /// The entity that received damage.
    pub target: Entity,
    /// The entity that dealt the damage (may equal target for DoT/fall damage).
    pub source: Entity,
    /// Actual damage dealt after defenses.
    pub amount: f32,
    /// Whether this damage triggered a stagger.
    pub staggered: bool,
    /// Whether this hit killed the target.
    pub lethal: bool,
}

/// Emitted when an entity's HP reaches zero.
#[derive(Debug, Clone)]
pub struct DeathEvent {
    /// The entity that died.
    pub entity: Entity,
    /// The entity that delivered the killing blow.
    pub killer: Option<Entity>,
    /// World position of death (for drop/echo spawning in souls-like).
    pub position: glam::Vec2,
}

/// Emitted when an entity's poise breaks and they enter stagger state.
#[derive(Debug, Clone)]
pub struct StaggerEvent {
    /// The entity that was staggered.
    pub entity: Entity,
    /// Duration of the stagger in fixed steps.
    pub duration: u32,
}

/// Emitted when an entity is healed.
#[derive(Debug, Clone)]
pub struct HealEvent {
    /// The entity that was healed.
    pub target: Entity,
    /// Amount restored.
    pub amount: f32,
}

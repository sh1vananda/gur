//! Effect pipeline — composable combat effects applied in order.
//!
//! When a hit lands, the engine runs a chain of `Effect`s:
//! Damage → Poise → Knockback → StatusEffects → Death check
//!
//! Each effect is a pure transformation on the world state + event emission.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A single effect in the combat pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Effect {
    /// Deal physical damage.
    Damage {
        /// Base damage before scaling and defense.
        base: f32,
        /// Strength scaling coefficient.
        str_scaling: f32,
    },
    /// Apply poise damage (may trigger stagger).
    PoiseDamage {
        /// Amount of poise to remove.
        amount: f32,
    },
    /// Apply a velocity impulse (knockback).
    Knockback {
        /// World-space impulse vector.
        impulse: Vec2,
    },
    /// Apply a damage-over-time status.
    ApplyBleed {
        /// Damage per fixed step.
        dps: f32,
        /// Duration in fixed steps.
        duration: u32,
    },
    /// Apply a slow debuff.
    ApplySlow {
        /// Speed multiplier (e.g. 0.5 = half speed).
        factor: f32,
        /// Duration in fixed steps.
        duration: u32,
    },
    /// Heal the target.
    Heal {
        /// Amount to restore.
        amount: f32,
    },
    /// Instantly kill the target (boss finisher, fall damage).
    InstantKill,
}

/// The ordered pipeline of effects applied when a hit resolves.
///
/// Stored as a global resource, accessed by the combat system each fixed step.
pub struct EffectPipeline {
    /// Default pipeline applied to a standard physical hit.
    default_chain: Vec<Effect>,
}

impl EffectPipeline {
    /// Create a pipeline with the standard physical hit chain.
    pub fn new() -> Self {
        Self {
            default_chain: vec![
                Effect::Damage { base: 0.0, str_scaling: 1.0 }, // filled in per-weapon
                Effect::PoiseDamage { amount: 10.0 },
            ],
        }
    }

    /// Build a pipeline from a weapon's effect list.
    pub fn from_effects(effects: Vec<Effect>) -> Self {
        Self { default_chain: effects }
    }

    /// Access the effect chain.
    pub fn chain(&self) -> &[Effect] {
        &self.default_chain
    }
}

impl Default for EffectPipeline {
    fn default() -> Self { Self::new() }
}

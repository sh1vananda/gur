//! # gur-combat
//!
//! The souls-like combat engine for GUR.
//!
//! ## Design Pillars
//! 1. **Mode-agnostic**: real-time and turn-based are both supported via config
//! 2. **Composable effects**: damage, knockback, bleed, stagger are pipeline stages
//! 3. **Code-first**: all weapons, enemies, abilities are Rust structs — no editor
//! 4. **Event-driven**: combat outcomes flow through `EventBus`, never function calls
//!
//! ## Combat Loop (real-time mode)
//! ```text
//! Input → ActionQueue → priority resolve → Effect pipeline → Stats → Events
//! ```

#![warn(missing_docs)]

pub mod action;
pub mod effect;
pub mod events;
pub mod stats;
pub mod weapon;

pub use action::{CombatAction, ActionQueue, CombatMode};
pub use effect::{Effect, EffectPipeline};
pub use events::{DamageEvent, DeathEvent, StaggerEvent, HealEvent};
pub use stats::{Stats, StatModifier, ModifierStack};
pub use weapon::{Weapon, WeaponClass};

/// Plugin that registers combat resources and systems.
pub mod plugin {
    use gur_core::prelude::*;

    /// Configures and registers all combat systems.
    pub struct CombatPlugin {
        /// Real-time or turn-based mode.
        pub mode: super::CombatMode,
    }

    impl Default for CombatPlugin {
        fn default() -> Self {
            Self { mode: super::CombatMode::RealTime }
        }
    }

    impl Plugin for CombatPlugin {
        fn name(&self) -> &'static str { "gur_combat" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            world.insert_resource(super::EffectPipeline::new());
            log::info!("CombatPlugin ready — mode: {:?}", self.mode);
            Ok(())
        }
    }
}

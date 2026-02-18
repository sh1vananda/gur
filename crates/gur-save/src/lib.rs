//! # gur-save
//!
//! World serialization, save slots, and incremental snapshots.
//!
//! ## Design
//! - Save data is a structured JSON document (human-readable, diff-able in git)
//! - Incremental snapshots: only changed zones are re-serialized
//! - Multiple save slots supported
//! - No raw ECS data is saved — only semantic game state

#![warn(missing_docs)]

pub mod slot;
pub mod snapshot;

pub use slot::{SaveSlot, SaveManager};
pub use snapshot::GameSnapshot;

/// Plugin that registers the save manager.
pub mod plugin {
    use gur_core::prelude::*;

    /// Registers the [`SaveManager`] resource.
    #[derive(Default)]
    pub struct SavePlugin;

    impl Plugin for SavePlugin {
        fn name(&self) -> &'static str { "gur_save" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            world.insert_resource(super::SaveManager::new("saves"));
            log::info!("SavePlugin ready — save directory: saves/");
            Ok(())
        }
    }
}

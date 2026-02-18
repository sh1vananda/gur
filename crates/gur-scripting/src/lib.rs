//! # gur-scripting
//!
//! RON/TOML data loader and game data registry.
//! Game designers define enemies, items, abilities in data files — no Rust needed.

#![warn(missing_docs)]

pub mod loader;
pub mod registry;

pub use loader::DataLoader;
pub use registry::{DataRegistry, EnemyDef, ItemDef};

/// Plugin that registers the data loader and registry.
pub mod plugin {
    use gur_core::prelude::*;

    /// Loads all RON data files and registers them in the data registry.
    pub struct ScriptingPlugin {
        /// Path to game data root (e.g. "assets/data").
        pub data_root: String,
    }

    impl Default for ScriptingPlugin {
        fn default() -> Self {
            Self { data_root: "assets/data".to_string() }
        }
    }

    impl Plugin for ScriptingPlugin {
        fn name(&self) -> &'static str { "gur_scripting" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            let mut registry = super::DataRegistry::new();
            let loader = super::DataLoader::new(&self.data_root);

            // Load all enemy definitions
            if let Ok(enemies) = loader.load_enemies() {
                let count = enemies.len();
                for def in enemies {
                    registry.register_enemy(def);
                }
                log::info!("Loaded {} enemy definitions", count);
            }

            // Load all item definitions
            if let Ok(items) = loader.load_items() {
                let count = items.len();
                for def in items {
                    registry.register_item(def);
                }
                log::info!("Loaded {} item definitions", count);
            }

            world.insert_resource(registry);
            log::info!("ScriptingPlugin ready — data root: {}", self.data_root);
            Ok(())
        }
    }
}

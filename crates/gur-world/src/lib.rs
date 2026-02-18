//! # gur-world
//!
//! Open-world zone system, tilemap loading, and spatial indexing.
//!
//! ## Concepts
//! - **Zone** — a named region of the world (e.g. "Ashlands", "Crystalveil").
//!   Defined entirely in code or RON files. Contains tilemap ref, enemies, music.
//! - **Chunk** — a subdivision of a zone (e.g. 32×32 tiles). Loaded on demand.
//! - **Spatial Index** — a flat grid for fast entity proximity queries.

#![warn(missing_docs)]

pub mod zone;
pub mod spatial;

pub use zone::{Zone, ZoneRegistry, ZoneTransition};
pub use spatial::SpatialGrid;

/// Plugin that registers world resources and systems.
pub mod plugin {
    use gur_core::prelude::*;

    /// Registers the zone registry and spatial index.
    #[derive(Default)]
    pub struct WorldPlugin;

    impl Plugin for WorldPlugin {
        fn name(&self) -> &'static str { "gur_world" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            world.insert_resource(super::ZoneRegistry::new());
            world.insert_resource(super::SpatialGrid::new(64.0)); // 64-pixel cells
            log::info!("WorldPlugin ready");
            Ok(())
        }
    }
}

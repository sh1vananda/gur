//! # gur-audio
//!
//! Audio engine for GUR — powered by `kira`.
//!
//! ## Features
//! - Background music with crossfade/loop support
//! - Sound effects with pooling (no duplicate allocations per frame)
//! - Spatial audio (distance-based volume falloff for world sounds)
//! - Dynamic music layers (add/remove stems based on game state)
//!
//! ## Usage
//! ```rust,ignore
//! let audio = world.resource_mut::<AudioManager>();
//! audio.play_music("audio/ashlands.ogg")?;
//! audio.play_sfx("audio/sfx/sword_swing.ogg")?;
//! ```

#![warn(missing_docs)]

pub mod manager;
pub mod spatial;

pub use manager::AudioManager;

/// Plugin that initialises the audio engine.
pub mod plugin {
    use gur_core::prelude::*;

    /// Initialises kira and registers the [`AudioManager`] resource.
    #[derive(Default)]
    pub struct AudioPlugin;

    impl Plugin for AudioPlugin {
        fn name(&self) -> &'static str { "gur_audio" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            match super::AudioManager::new() {
                Ok(mgr) => {
                    world.insert_resource(mgr);
                    log::info!("AudioPlugin ready — kira audio engine initialised");
                    Ok(())
                }
                Err(e) => {
                    // Audio failure is non-fatal — game runs without sound.
                    log::warn!("AudioPlugin: failed to initialise audio — {e}. Running silently.");
                    Ok(())
                }
            }
        }
    }
}

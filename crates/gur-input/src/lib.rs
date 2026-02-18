//! # gur-input
//!
//! Keyboard, mouse, and gamepad input abstraction for the GUR engine.
//!
//! ## Design
//! - Translates raw `winit` events into engine-native [`Action`]s
//! - Provides a rebindable [`InputMap`] (action → key/button binding)
//! - Emits [`ActionEvent`]s into the [`EventBus`] each frame
//! - Game code queries [`InputState`] for held/pressed/released checks
//!
//! Game code NEVER references `winit` key codes directly — only [`Action`]s.
//! This makes rebinding, gamepad support, and testing trivial.

#![warn(missing_docs)]

pub mod action;
pub mod binding;
pub mod map;
pub mod state;

pub use action::{Action, ActionEvent, ActionKind};
pub use map::InputMap;
pub use state::InputState;

/// Plugin that registers input resources and systems.
pub mod plugin {
    use gur_core::prelude::*;

    /// Registers [`InputState`] and [`InputMap`] as resources.
    pub struct InputPlugin {
        /// Initial key bindings to load at startup.
        pub map: super::InputMap,
    }

    impl Default for InputPlugin {
        fn default() -> Self {
            Self { map: super::InputMap::default_bindings() }
        }
    }

    impl Plugin for InputPlugin {
        fn name(&self) -> &'static str { "gur_input" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            world.insert_resource(super::InputState::new());
            world.insert_resource(self.map.clone());
            log::info!("InputPlugin ready — {} bindings loaded", self.map.binding_count());
            Ok(())
        }
    }
}

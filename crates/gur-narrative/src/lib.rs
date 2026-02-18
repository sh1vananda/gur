//! # gur-narrative
//!
//! Dialogue graph, story state machine, and condition evaluator.
//! Writers create dialogue in RON files — no Rust required.

#![warn(missing_docs)]

pub mod dialogue;
pub mod state;

pub use dialogue::{DialogueTree, DialogueNode, DialogueChoice, DialogueAction};
pub use state::StoryState;

/// Plugin that registers narrative resources.
pub mod plugin {
    use gur_core::prelude::*;

    /// Registers the story state resource.
    #[derive(Default)]
    pub struct NarrativePlugin;

    impl Plugin for NarrativePlugin {
        fn name(&self) -> &'static str { "gur_narrative" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            world.insert_resource(super::StoryState::new());
            log::info!("NarrativePlugin ready");
            Ok(())
        }
    }
}

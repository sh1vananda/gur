//! # gur-tools
//!
//! In-engine developer tooling: egui inspector, debug console, entity viewer,
//! performance overlay. All tools are compiled out in release builds.

#![warn(missing_docs)]

pub mod console;
pub mod inspector;
pub mod overlay;

pub use console::DevConsole;
pub use inspector::EntityInspector;
pub use overlay::PerfOverlay;

/// Plugin that registers all developer tools.
pub mod plugin {
    use gur_core::prelude::*;

    /// Registers dev tools as resources. No-op in release builds.
    #[derive(Default)]
    pub struct ToolsPlugin;

    impl Plugin for ToolsPlugin {
        fn name(&self) -> &'static str { "gur_tools" }

        fn build(&self, world: &mut GurWorld, _schedule: &mut Schedule) -> EngineResult<()> {
            #[cfg(debug_assertions)]
            {
                world.insert_resource(super::DevConsole::new());
                world.insert_resource(super::EntityInspector::new());
                world.insert_resource(super::PerfOverlay::new());
                log::info!("ToolsPlugin ready — dev tools active");
            }
            #[cfg(not(debug_assertions))]
            log::debug!("ToolsPlugin: release build, tools disabled");
            Ok(())
        }
    }
}

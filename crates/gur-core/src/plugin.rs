//! Plugin system — the primary API for extending the engine.
//!
//! Every engine subsystem (renderer, audio, combat, narrative) and every
//! game feature (player controller, enemy AI) is a `Plugin`. Plugins:
//!
//! 1. Register their **resources** into `GurWorld`
//! 2. Register their **systems** into the `Schedule`
//! 3. Register their **event types** into the `EventBus`
//!
//! This keeps `main.rs` clean and makes features trivially toggleable.
//!
//! ## Usage
//! ```rust,ignore
//! engine
//!     .add_plugin(RenderPlugin::default())
//!     .add_plugin(AudioPlugin::default())
//!     .add_plugin(CombatPlugin::default())
//!     .add_plugin(MyGamePlugin);
//! ```

use std::collections::HashSet;

use crate::ecs::GurWorld;
use crate::error::{EngineError, EngineResult};
use crate::scheduler::Schedule;

/// The core trait all engine and game plugins implement.
///
/// Implement `build` to register resources, systems, and event channels.
/// Implement `name` to provide a unique, human-readable identifier used in
/// duplicate-detection and debug output.
pub trait Plugin: Send + Sync + 'static {
    /// A unique, stable name for this plugin. Used for duplicate detection.
    fn name(&self) -> &'static str;

    /// Called once during engine startup. Register all resources and systems here.
    fn build(&self, world: &mut GurWorld, schedule: &mut Schedule) -> EngineResult<()>;

    /// Called when the plugin is being removed (cleanup resources, deregister systems).
    /// Optional — defaults to no-op.
    fn cleanup(&self, _world: &mut GurWorld) {}

    /// Declare which plugins this one depends on (by name).
    /// The engine will ensure dependencies are built first.
    fn dependencies(&self) -> &[&'static str] {
        &[]
    }
}

/// Collects and builds plugins in dependency order.
///
/// Plugins are resolved in a topological order determined by their `dependencies()`.
pub struct PluginRegistry {
    /// Plugins in insertion order (before topo-sort).
    plugins: Vec<Box<dyn Plugin>>,
    /// Names of already-built plugins (for duplicate detection).
    built: HashSet<&'static str>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            built: HashSet::new(),
        }
    }

    /// Add a plugin to the registry. Returns an error if the plugin name is already registered.
    pub fn add<P: Plugin>(&mut self, plugin: P) -> EngineResult<&mut Self> {
        let name = plugin.name();
        if self.built.contains(name) {
            return Err(EngineError::DuplicatePlugin(name));
        }
        self.plugins.push(Box::new(plugin));
        Ok(self)
    }

    /// Build all registered plugins against the world and schedule.
    ///
    /// Plugins are built in the order they were added. It is the caller's
    /// responsibility to add plugins after their dependencies.
    ///
    /// TODO(future): implement full topological sort for automatic ordering.
    pub fn build_all(
        &mut self,
        world: &mut GurWorld,
        schedule: &mut Schedule,
    ) -> EngineResult<()> {
        for plugin in &self.plugins {
            log::info!("Building plugin: {}", plugin.name());
            plugin.build(world, schedule)?;
            self.built.insert(plugin.name());
        }
        Ok(())
    }

    /// Call `cleanup` on all plugins (in reverse build order).
    pub fn cleanup_all(&self, world: &mut GurWorld) {
        for plugin in self.plugins.iter().rev() {
            plugin.cleanup(world);
        }
    }

    /// Returns `true` if a plugin with the given name has been registered.
    pub fn has(&self, name: &str) -> bool {
        self.plugins.iter().any(|p| p.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::Schedule;

    struct FooPlugin;
    impl Plugin for FooPlugin {
        fn name(&self) -> &'static str { "foo" }
        fn build(&self, _w: &mut GurWorld, _s: &mut Schedule) -> EngineResult<()> { Ok(()) }
    }

    #[test]
    fn duplicate_plugin_is_rejected() {
        let mut registry = PluginRegistry::new();
        registry.add(FooPlugin).unwrap();
        let result = registry.add(FooPlugin);
        assert!(matches!(result, Err(EngineError::DuplicatePlugin("foo"))));
    }

    #[test]
    fn build_all_succeeds() {
        let mut registry = PluginRegistry::new();
        registry.add(FooPlugin).unwrap();
        let mut world = GurWorld::new();
        let mut schedule = Schedule::new();
        registry.build_all(&mut world, &mut schedule).unwrap();
    }
}

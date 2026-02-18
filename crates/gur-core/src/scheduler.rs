//! System scheduler — organises and runs game systems in stage order.
//!
//! ## Stage Model
//! Systems are grouped into named [`SystemStage`]s that run in a fixed order
//! each frame. Within a stage, systems run sequentially (parallelism is a
//! future extension via Rayon task graphs).
//!
//! ## Standard Stage Order
//! ```text
//! PreUpdate  →  Input  →  FixedUpdate  →  Update  →  PostUpdate  →  Render  →  Debug
//! ```
//!
//! ## Adding Systems
//! ```rust,ignore
//! schedule.add_system(SystemStage::Update, "player_movement", player_movement_system);
//! schedule.add_system(SystemStage::PostUpdate, "camera_follow", camera_follow_system);
//! ```

use std::collections::HashMap;

use crate::ecs::GurWorld;
use crate::error::{EngineError, EngineResult};
use crate::event::EventBus;

// ─── System Function Type ────────────────────────────────────────────────────

/// The signature every system function must match.
///
/// Systems receive mutable access to the whole world and the event bus.
/// They produce `EngineResult<()>` so errors propagate cleanly instead of panicking.
pub type SystemFn = fn(&mut GurWorld, &mut EventBus) -> EngineResult<()>;

// ─── System Stage ────────────────────────────────────────────────────────────

/// Named execution stage within a single game frame.
///
/// Stages run in the order defined by [`Schedule::stage_order`].
/// Adding new stages is possible via `Schedule::insert_stage_after`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SystemStage {
    /// Before any input is read — good for clearing per-frame state.
    PreUpdate,
    /// Input processing — read device state, emit input events.
    Input,
    /// Fixed-timestep physics / combat tick (may run 0–N times per frame).
    FixedUpdate,
    /// Main game logic: movement, AI, state machines.
    Update,
    /// Post-logic passes: camera follow, transform hierarchy propagation.
    PostUpdate,
    /// Rendering: collect draw calls, submit to GPU.
    Render,
    /// Debug overlays, egui inspector — always last.
    Debug,
    /// Custom stage with a user-provided name.
    Custom(String),
}

impl SystemStage {
    /// Returns the string name of this stage (used in error messages).
    pub fn name(&self) -> &str {
        match self {
            Self::PreUpdate   => "PreUpdate",
            Self::Input       => "Input",
            Self::FixedUpdate => "FixedUpdate",
            Self::Update      => "Update",
            Self::PostUpdate  => "PostUpdate",
            Self::Render      => "Render",
            Self::Debug       => "Debug",
            Self::Custom(n)   => n.as_str(),
        }
    }
}

// ─── System Descriptor ───────────────────────────────────────────────────────

/// A registered system with its metadata.
struct SystemEntry {
    /// Unique name within its stage (used for ordering and debugging).
    label:  String,
    /// Whether this system is currently active.
    active: bool,
    /// The function pointer.
    func:   SystemFn,
}

// ─── Schedule ────────────────────────────────────────────────────────────────

/// The full frame schedule: an ordered list of stages, each containing systems.
///
/// ## LOAD-BEARING
/// The stage ordering is fixed at startup. Changing stage names after save-game
/// data or replay data references them may break determinism.
pub struct Schedule {
    /// Ordered list of stage keys (determines execution order).
    stage_order: Vec<SystemStage>,
    /// Systems per stage, in registration order.
    stages: HashMap<SystemStage, Vec<SystemEntry>>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

impl Schedule {
    /// Create a schedule with the standard stage order.
    pub fn new() -> Self {
        let stage_order = vec![
            SystemStage::PreUpdate,
            SystemStage::Input,
            SystemStage::FixedUpdate,
            SystemStage::Update,
            SystemStage::PostUpdate,
            SystemStage::Render,
            SystemStage::Debug,
        ];
        let stages = stage_order
            .iter()
            .cloned()
            .map(|s| (s, Vec::new()))
            .collect();
        Self { stage_order, stages }
    }

    // ─── Stage Management ────────────────────────────────────────────────

    /// Insert a custom stage immediately after an existing stage.
    pub fn insert_stage_after(
        &mut self,
        after: &SystemStage,
        new_stage: SystemStage,
    ) -> EngineResult<()> {
        let pos = self
            .stage_order
            .iter()
            .position(|s| s == after)
            .ok_or_else(|| EngineError::UnknownStage(after.name().to_owned()))?;
        self.stage_order.insert(pos + 1, new_stage.clone());
        self.stages.insert(new_stage, Vec::new());
        Ok(())
    }

    // ─── System Registration ─────────────────────────────────────────────

    /// Register a system in the given stage.
    ///
    /// Systems in the same stage run in registration order.
    /// `label` must be unique within the stage (panics in debug, silently
    /// overwrites in release — label uniqueness is a programmer contract).
    pub fn add_system(
        &mut self,
        stage: SystemStage,
        label: impl Into<String>,
        func: SystemFn,
    ) -> EngineResult<()> {
        let label = label.into();
        let entry = self
            .stages
            .get_mut(&stage)
            .ok_or_else(|| EngineError::UnknownStage(stage.name().to_owned()))?;

        debug_assert!(
            !entry.iter().any(|e| e.label == label),
            "Duplicate system label `{label}` in stage `{}`",
            stage.name()
        );

        entry.push(SystemEntry { label, active: true, func });
        Ok(())
    }

    /// Disable a system by label so it is skipped during `run_stage`.
    pub fn set_system_active(
        &mut self,
        stage: &SystemStage,
        label: &str,
        active: bool,
    ) -> EngineResult<()> {
        let entries = self
            .stages
            .get_mut(stage)
            .ok_or_else(|| EngineError::UnknownStage(stage.name().to_owned()))?;

        entries
            .iter_mut()
            .find(|e| e.label == label)
            .map(|e| e.active = active)
            .ok_or_else(|| EngineError::UnknownStage(format!("{}::{}", stage.name(), label)))
    }

    // ─── Execution ───────────────────────────────────────────────────────

    /// Run all active systems in the given stage against `world` and `bus`.
    ///
    /// Returns the first error encountered (stops execution of the stage).
    pub fn run_stage(
        &self,
        stage: &SystemStage,
        world: &mut GurWorld,
        bus: &mut EventBus,
    ) -> EngineResult<()> {
        let entries = match self.stages.get(stage) {
            Some(e) => e,
            None => return Err(EngineError::UnknownStage(stage.name().to_owned())),
        };
        for entry in entries.iter().filter(|e| e.active) {
            profiling::scope!(&entry.label);
            (entry.func)(world, bus)?;
        }
        Ok(())
    }

    /// Run **all** stages in order. This is the main per-frame call.
    ///
    /// On error, returns immediately with the stage that failed.
    pub fn run_all(
        &self,
        world: &mut GurWorld,
        bus: &mut EventBus,
    ) -> EngineResult<()> {
        for stage in &self.stage_order {
            self.run_stage(stage, world, bus)?;
        }
        Ok(())
    }

    /// Run all stages up to (but not including) `Render`.
    /// Useful for headless simulation / testing.
    pub fn run_logic_only(
        &self,
        world: &mut GurWorld,
        bus: &mut EventBus,
    ) -> EngineResult<()> {
        for stage in self.stage_order.iter().take_while(|s| **s != SystemStage::Render) {
            self.run_stage(stage, world, bus)?;
        }
        Ok(())
    }

    /// Returns a slice of stage labels in execution order (for debugging).
    pub fn stage_names(&self) -> Vec<&str> {
        self.stage_order.iter().map(|s| s.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBus;

    fn noop_system(_w: &mut GurWorld, _b: &mut EventBus) -> EngineResult<()> {
        Ok(())
    }

    fn error_system(_w: &mut GurWorld, _b: &mut EventBus) -> EngineResult<()> {
        Err(EngineError::other("intentional test error"))
    }

    #[test]
    fn add_and_run_system() {
        let mut sched = Schedule::new();
        sched.add_system(SystemStage::Update, "noop", noop_system).unwrap();
        let mut world = GurWorld::new();
        let mut bus = EventBus::new();
        sched.run_stage(&SystemStage::Update, &mut world, &mut bus).unwrap();
    }

    #[test]
    fn error_propagates_from_system() {
        let mut sched = Schedule::new();
        sched.add_system(SystemStage::Update, "err", error_system).unwrap();
        let mut world = GurWorld::new();
        let mut bus = EventBus::new();
        assert!(sched.run_stage(&SystemStage::Update, &mut world, &mut bus).is_err());
    }

    #[test]
    fn disabled_system_is_skipped() {
        let mut sched = Schedule::new();
        sched.add_system(SystemStage::Update, "err", error_system).unwrap();
        sched.set_system_active(&SystemStage::Update, "err", false).unwrap();
        let mut world = GurWorld::new();
        let mut bus = EventBus::new();
        // Should not error because system is disabled
        sched.run_stage(&SystemStage::Update, &mut world, &mut bus).unwrap();
    }
}

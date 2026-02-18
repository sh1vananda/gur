//! Story state — tracks active dialogue, quest progress, and world flags.

use std::collections::HashMap;

/// Global narrative state. Stored as a resource in `GurWorld`.
#[derive(Debug, Default)]
pub struct StoryState {
    /// Active world flags (e.g. "boss_ashlands_defeated", "npc_met_merchant").
    flags: HashMap<String, bool>,
    /// Numeric story counters.
    counters: HashMap<String, i64>,
    /// Currently active dialogue tree ID (if any).
    pub active_dialogue: Option<String>,
    /// Current node ID in the active dialogue.
    pub current_node: Option<String>,
}

impl StoryState {
    /// Create an empty story state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a world flag.
    pub fn set_flag(&mut self, key: impl Into<String>, value: bool) {
        self.flags.insert(key.into(), value);
    }

    /// Check a world flag (defaults `false`).
    pub fn flag(&self, key: &str) -> bool {
        self.flags.get(key).copied().unwrap_or(false)
    }

    /// Increment a counter and return the new value.
    pub fn increment(&mut self, key: impl Into<String>) -> i64 {
        let c = self.counters.entry(key.into()).or_insert(0);
        *c += 1;
        *c
    }

    /// Get a counter value (defaults 0).
    pub fn counter(&self, key: &str) -> i64 {
        self.counters.get(key).copied().unwrap_or(0)
    }

    /// Start a dialogue.
    pub fn begin_dialogue(&mut self, tree_id: impl Into<String>, entry_node: impl Into<String>) {
        self.active_dialogue = Some(tree_id.into());
        self.current_node = Some(entry_node.into());
    }

    /// End the current dialogue.
    pub fn end_dialogue(&mut self) {
        self.active_dialogue = None;
        self.current_node = None;
    }

    /// Returns `true` if a dialogue is currently active.
    pub fn in_dialogue(&self) -> bool {
        self.active_dialogue.is_some()
    }
}

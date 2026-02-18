//! [`InputState`] — per-frame snapshot of all active actions.
//!
//! Updated by the input system at the start of the `Input` stage.
//! Game systems query this instead of polling winit directly.

use std::collections::HashSet;
use winit::keyboard::PhysicalKey;

use crate::action::Action;

/// Snapshot of which actions are active this frame.
///
/// Stored as a resource in `GurWorld`. The input system writes it;
/// game systems read it.
#[derive(Debug, Default)]
pub struct InputState {
    /// Actions pressed this frame (first frame only).
    just_pressed: HashSet<Action>,
    /// Actions currently held (includes the press frame).
    held: HashSet<Action>,
    /// Actions released this frame (last frame only).
    just_released: HashSet<Action>,
    /// Raw keys held according to winit (for InputMap evaluation).
    keys_held: HashSet<PhysicalKey>,
}

impl InputState {
    /// Create an empty state.
    pub fn new() -> Self {
        Self::default()
    }

    // ─── Frame lifecycle ─────────────────────────────────────────────────

    /// Advance state: clear `just_pressed` / `just_released` from previous frame.
    /// Called at the start of each `Input` stage tick.
    pub fn begin_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Record a key-down event from winit.
    pub fn key_down(&mut self, key: PhysicalKey) {
        self.keys_held.insert(key);
    }

    /// Record a key-up event from winit.
    pub fn key_up(&mut self, key: PhysicalKey) {
        self.keys_held.remove(&key);
    }

    /// Mark an action as activated this frame.
    pub fn press(&mut self, action: Action) {
        if !self.held.contains(&action) {
            self.just_pressed.insert(action);
        }
        self.held.insert(action);
    }

    /// Mark an action as deactivated this frame.
    pub fn release(&mut self, action: Action) {
        self.held.remove(&action);
        self.just_released.insert(action);
    }

    // ─── Queries ─────────────────────────────────────────────────────────

    /// `true` if the action was activated for the first time this frame.
    #[inline]
    pub fn just_pressed(&self, action: Action) -> bool {
        self.just_pressed.contains(&action)
    }

    /// `true` if the action is currently active (held or pressed).
    #[inline]
    pub fn held(&self, action: Action) -> bool {
        self.held.contains(&action)
    }

    /// `true` if the action was released this frame.
    #[inline]
    pub fn just_released(&self, action: Action) -> bool {
        self.just_released.contains(&action)
    }

    /// Access the raw set of held winit physical keys (for InputMap evaluation).
    pub fn keys_held(&self) -> &HashSet<PhysicalKey> {
        &self.keys_held
    }

    /// `true` if any action is currently held.
    pub fn any_held(&self) -> bool {
        !self.held.is_empty()
    }

    /// Returns a vec of all currently held actions (for debugging / rebinding UI).
    pub fn held_actions(&self) -> Vec<Action> {
        self.held.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_hold_release_cycle() {
        let mut state = InputState::new();
        state.press(Action::AttackLight);
        assert!(state.just_pressed(Action::AttackLight));
        assert!(state.held(Action::AttackLight));

        // Next frame
        state.begin_frame();
        state.press(Action::AttackLight); // still held
        assert!(!state.just_pressed(Action::AttackLight)); // no longer "just"
        assert!(state.held(Action::AttackLight));

        // Release
        state.begin_frame();
        state.release(Action::AttackLight);
        assert!(state.just_released(Action::AttackLight));
        assert!(!state.held(Action::AttackLight));
    }
}

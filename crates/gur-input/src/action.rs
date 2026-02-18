//! Game actions — the semantic layer between raw input and game behaviour.
//!
//! An `Action` is what the player *intends*, not what physical key they pressed.
//! Game systems react to `Action::Attack`, not `KeyCode::Z`.
//!
//! ## Extending Actions
//! Add a variant here. Add its binding in [`InputMap::default_bindings`].
//! No other files need changing.

use serde::{Deserialize, Serialize};

/// Every possible player intent the input system can express.
///
/// Variants are grouped by context — movement, combat, menu — for readability.
/// Add new variants freely; they have no runtime cost until bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    // ── Movement ────────────────────────────────────────────────────────
    /// Move up / north.
    MoveUp,
    /// Move down / south.
    MoveDown,
    /// Move left / west.
    MoveLeft,
    /// Move right / east.
    MoveRight,

    // ── Combat ──────────────────────────────────────────────────────────
    /// Primary attack (light hit).
    AttackLight,
    /// Heavy / charged attack.
    AttackHeavy,
    /// Dodge roll / i-frame escape.
    Dodge,
    /// Parry / raise guard.
    Guard,
    /// Use item / flask.
    UseItem,
    /// Cycle item left.
    CycleItemLeft,
    /// Cycle item right.
    CycleItemRight,
    /// Lock on to nearest enemy / toggle lock.
    LockOn,

    // ── Interaction ─────────────────────────────────────────────────────
    /// Interact with environment, NPCs, open dialogue.
    Interact,
    /// Cancel / back in menus.
    Cancel,
    /// Advance dialogue line.
    DialogueAdvance,

    // ── Menus & UI ──────────────────────────────────────────────────────
    /// Open / close pause menu.
    PauseMenu,
    /// Open inventory.
    Inventory,
    /// Open map.
    Map,

    // ── Debug (stripped in release builds) ─────────────────────────────
    /// Open / close dev console.
    #[cfg(debug_assertions)]
    DevConsole,
}

/// Whether an action was just pressed, held, or just released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    /// Action transitioned from idle to active this frame.
    Pressed,
    /// Action has been active for more than one frame.
    Held,
    /// Action transitioned from active to idle this frame.
    Released,
}

/// An event emitted by the input system each frame for every active action.
///
/// Emitted into the [`EventBus`] — game systems subscribe to this event
/// rather than polling `InputState` when they need transition semantics.
#[derive(Debug, Clone, Copy)]
pub struct ActionEvent {
    /// The semantic action that occurred.
    pub action: Action,
    /// Whether it was pressed, held, or released.
    pub kind: ActionKind,
}

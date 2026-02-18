//! Combat action queue — souls-like action-priority system.

use serde::{Deserialize, Serialize};

/// Whether the combat system resolves actions in real-time or turn order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatMode {
    /// Elden Ring style — actions resolve immediately, time-based.
    RealTime,
    /// Undertale style — turn order, initiative-based.
    TurnBased,
}

/// A single combat intention queued by a player or AI controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatAction {
    /// Execute a light attack.
    AttackLight {
        /// Direction of the attack (for hitbox placement).
        direction: glam::Vec2,
    },
    /// Execute a heavy/charged attack.
    AttackHeavy {
        /// Direction of the attack.
        direction: glam::Vec2,
        /// Charge level [0.0, 1.0] — affects damage multiplier.
        charge: f32,
    },
    /// Dodge roll — grants i-frames.
    Dodge {
        /// Direction to roll.
        direction: glam::Vec2,
    },
    /// Raise guard / parry window.
    Guard,
    /// Use a consumable item.
    UseItem {
        /// Item type identifier (matches RON data key).
        item_id: String,
    },
    /// Do nothing this turn (turn-based idle / wait).
    Wait,
}

impl CombatAction {
    /// Stamina cost of this action (consumed before execution).
    pub fn stamina_cost(&self) -> f32 {
        match self {
            Self::AttackLight { .. }      => 15.0,
            Self::AttackHeavy { .. }      => 30.0,
            Self::Dodge { .. }            => 20.0,
            Self::Guard                   =>  5.0, // per frame held
            Self::UseItem { .. }          =>  0.0,
            Self::Wait                    =>  0.0,
        }
    }

    /// Priority value for turn-based ordering (higher = executes first).
    pub fn priority(&self) -> i32 {
        match self {
            Self::Dodge { .. }            => 100,
            Self::Guard                   =>  80,
            Self::AttackLight { .. }      =>  50,
            Self::AttackHeavy { .. }      =>  30,
            Self::UseItem { .. }          =>  20,
            Self::Wait                    =>   0,
        }
    }
}

/// Per-entity action queue — at most one action resolved per fixed step (real-time)
/// or per turn (turn-based).
#[derive(Debug, Default)]
pub struct ActionQueue {
    /// Pending actions (FIFO for real-time, sorted by priority for turn-based).
    queue: std::collections::VecDeque<CombatAction>,
}

impl ActionQueue {
    /// Create an empty queue.
    pub fn new() -> Self { Self::default() }

    /// Push an action onto the queue.
    pub fn push(&mut self, action: CombatAction) {
        self.queue.push_back(action);
    }

    /// Pop the next action for real-time processing (FIFO).
    pub fn pop(&mut self) -> Option<CombatAction> {
        self.queue.pop_front()
    }

    /// Pop the highest-priority action (for turn-based processing).
    pub fn pop_priority(&mut self) -> Option<CombatAction> {
        if self.queue.is_empty() { return None; }
        let idx = self.queue
            .iter()
            .enumerate()
            .max_by_key(|(_, a)| a.priority())
            .map(|(i, _)| i)
            .unwrap();
        self.queue.remove(idx)
    }

    /// Discard all queued actions.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Returns `true` if there are no pending actions.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

//! [`GameSnapshot`] — the serialisable game state saved to disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The complete save state for a game session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    /// Save file format version (bump when schema changes).
    pub version: u32,
    /// Unix timestamp of when this save was created.
    pub timestamp: u64,
    /// Total play time in seconds.
    pub play_time_seconds: f64,
    /// ID of the zone the player is currently in.
    pub current_zone: String,
    /// Player world-space position.
    pub player_position: [f32; 2],
    /// Player stats snapshot.
    pub player_stats: PlayerStatsSnapshot,
    /// World flags (quest progression, doors opened, NPCs talked to).
    pub flags: HashMap<String, bool>,
    /// Numeric counters (enemies killed, items collected, etc.).
    pub counters: HashMap<String, i64>,
    /// String values (chosen character name, etc.).
    pub strings: HashMap<String, String>,
}

impl GameSnapshot {
    /// Create a new save with default values.
    pub fn new_game(zone: impl Into<String>) -> Self {
        Self {
            version: 1,
            timestamp: current_unix_timestamp(),
            play_time_seconds: 0.0,
            current_zone: zone.into(),
            player_position: [0.0, 0.0],
            player_stats: PlayerStatsSnapshot::default(),
            flags: HashMap::new(),
            counters: HashMap::new(),
            strings: HashMap::new(),
        }
    }

    /// Set a world flag.
    pub fn set_flag(&mut self, key: impl Into<String>, value: bool) {
        self.flags.insert(key.into(), value);
    }

    /// Check a world flag (defaults to `false` if not set).
    pub fn flag(&self, key: &str) -> bool {
        self.flags.get(key).copied().unwrap_or(false)
    }

    /// Increment a counter.
    pub fn increment(&mut self, key: impl Into<String>) {
        *self.counters.entry(key.into()).or_insert(0) += 1;
    }
}

/// Snapshot of player stats saved between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStatsSnapshot {
    /// Current HP.
    pub hp: f32,
    /// Max HP.
    pub max_hp: f32,
    /// Strength level.
    pub strength: f32,
    /// Dexterity level.
    pub dexterity: f32,
    /// Intelligence level.
    pub intelligence: f32,
    /// Number of souls/echoes collected.
    pub echoes: u64,
}

impl Default for PlayerStatsSnapshot {
    fn default() -> Self {
        Self {
            hp: 100.0,
            max_hp: 100.0,
            strength: 10.0,
            dexterity: 8.0,
            intelligence: 5.0,
            echoes: 0,
        }
    }
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

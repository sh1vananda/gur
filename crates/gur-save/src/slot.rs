//! Save slot management — read/write save files from disk.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::snapshot::GameSnapshot;

/// Metadata shown on the save-select screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSlot {
    /// Slot index (0, 1, 2).
    pub index: usize,
    /// Display name (e.g. "Slot 1").
    pub label: String,
    /// Zone name at time of save.
    pub zone: String,
    /// Play time in seconds.
    pub play_time: f64,
    /// Unix timestamp of last save.
    pub timestamp: u64,
    /// Whether this slot has any save data.
    pub occupied: bool,
}

/// Manages multiple save slots and disk I/O.
pub struct SaveManager {
    /// Root directory for save files.
    save_dir: PathBuf,
}

impl SaveManager {
    /// Create a manager pointing at `save_dir`.
    pub fn new(save_dir: impl AsRef<Path>) -> Self {
        let path = save_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&path).ok(); // best-effort directory creation
        Self { save_dir: path }
    }

    fn slot_path(&self, slot: usize) -> PathBuf {
        self.save_dir.join(format!("slot_{slot}.json"))
    }

    /// Save a snapshot to the given slot.
    pub fn save(&self, slot: usize, snapshot: &GameSnapshot) -> std::io::Result<()> {
        let path = self.slot_path(slot);
        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Load a snapshot from the given slot. Returns `None` if the slot is empty.
    pub fn load(&self, slot: usize) -> std::io::Result<Option<GameSnapshot>> {
        let path = self.slot_path(slot);
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let snapshot: GameSnapshot = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(snapshot))
    }

    /// Delete a save slot.
    pub fn delete(&self, slot: usize) -> std::io::Result<()> {
        let path = self.slot_path(slot);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// List all occupied save slots with metadata.
    pub fn list_slots(&self) -> Vec<SaveSlot> {
        (0..3).map(|i| {
            match self.load(i) {
                Ok(Some(snap)) => SaveSlot {
                    index: i,
                    label: format!("Slot {}", i + 1),
                    zone: snap.current_zone.clone(),
                    play_time: snap.play_time_seconds,
                    timestamp: snap.timestamp,
                    occupied: true,
                },
                _ => SaveSlot {
                    index: i,
                    label: format!("Slot {}", i + 1),
                    zone: String::new(),
                    play_time: 0.0,
                    timestamp: 0,
                    occupied: false,
                },
            }
        }).collect()
    }
}

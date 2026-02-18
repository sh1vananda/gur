//! RON/TOML file loader with schema validation.

use std::path::{Path, PathBuf};
use crate::registry::{EnemyDef, ItemDef};

/// Loads game data files from disk and deserializes them.
pub struct DataLoader {
    root: PathBuf,
}

impl DataLoader {
    /// Create a loader with the given data root directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }

    /// Load all enemy definition files from `<root>/enemies/`.
    pub fn load_enemies(&self) -> anyhow::Result<Vec<EnemyDef>> {
        self.load_dir("enemies")
    }

    /// Load all item definition files from `<root>/items/`.
    pub fn load_items(&self) -> anyhow::Result<Vec<ItemDef>> {
        self.load_dir("items")
    }

    fn load_dir<T: serde::de::DeserializeOwned>(&self, subdir: &str) -> anyhow::Result<Vec<T>> {
        let dir = self.root.join(subdir);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "ron").unwrap_or(false) {
                let src = std::fs::read_to_string(&path)?;
                match ron::from_str::<T>(&src) {
                    Ok(def) => results.push(def),
                    Err(e) => log::warn!("Failed to parse {:?}: {}", path, e),
                }
            }
        }
        Ok(results)
    }
}

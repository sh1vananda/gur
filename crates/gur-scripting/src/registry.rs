//! Data registry — typed storage for all game data definitions.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Enemy definition loaded from RON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyDef {
    /// Unique identifier (matches zone `enemy_spawns` list).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Maximum HP.
    pub max_hp: f32,
    /// Base damage per attack.
    pub damage: f32,
    /// Move speed.
    pub move_speed: f32,
    /// Defense (flat damage reduction).
    pub defense: f32,
    /// Poise (stagger resistance).
    pub poise: f32,
    /// Sprite sheet path.
    pub sprite: String,
    /// Echo (soul) value on death.
    pub echo_value: u64,
    /// Detection range (how far the enemy can "see").
    pub detect_range: f32,
    /// Attack range.
    pub attack_range: f32,
}

/// Item definition loaded from RON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description shown in inventory.
    pub description: String,
    /// Item category.
    pub category: ItemCategory,
    /// How many can the player carry at once.
    pub max_stack: u32,
    /// Sprite sheet path.
    pub sprite: String,
}

/// Item category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemCategory {
    /// Heals the player.
    Consumable,
    /// Equippable weapon.
    Weapon,
    /// Equippable armor.
    Armor,
    /// Key item / quest item.
    Key,
    /// Crafting material.
    Material,
}

/// Central registry of all game data definitions.
pub struct DataRegistry {
    enemies: HashMap<String, EnemyDef>,
    items:   HashMap<String, ItemDef>,
}

impl DataRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { enemies: HashMap::new(), items: HashMap::new() }
    }

    /// Register an enemy definition.
    pub fn register_enemy(&mut self, def: EnemyDef) {
        self.enemies.insert(def.id.clone(), def);
    }

    /// Register an item definition.
    pub fn register_item(&mut self, def: ItemDef) {
        self.items.insert(def.id.clone(), def);
    }

    /// Get an enemy definition by ID.
    pub fn enemy(&self, id: &str) -> Option<&EnemyDef> {
        self.enemies.get(id)
    }

    /// Get an item definition by ID.
    pub fn item(&self, id: &str) -> Option<&ItemDef> {
        self.items.get(id)
    }

    /// All registered enemy IDs.
    pub fn enemy_ids(&self) -> impl Iterator<Item = &str> {
        self.enemies.keys().map(|s| s.as_str())
    }
}

impl Default for DataRegistry {
    fn default() -> Self { Self::new() }
}

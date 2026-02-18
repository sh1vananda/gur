//! Zone definitions — named regions of the open world.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use glam::Vec2;

use gur_render::Color;

/// A trigger rect that transports the player to another zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneTransition {
    /// World-space trigger rectangle: (x, y, width, height).
    pub trigger: [f32; 4],
    /// ID of the destination zone.
    pub to_zone: String,
    /// Spawn position in the destination zone.
    pub spawn_pos: Vec2,
}

/// A complete zone definition — defined in code or loaded from RON.
///
/// # Example (code):
/// ```rust,ignore
/// ZoneRegistry::new().register(Zone {
///     id: "ashlands".to_string(),
///     display_name: "The Ashlands".to_string(),
///     tilemap_path: "maps/ashlands.tmx".to_string(),
///     ambient_color: Color::ORANGE,
///     ambient_intensity: 0.6,
///     music_path: Some("audio/ashlands.ogg".to_string()),
///     enemy_spawns: vec!["ember_knight".to_string()],
///     transitions: vec![],
/// });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    /// Unique identifier used for zone lookup and transitions.
    pub id: String,
    /// Human-readable name shown on screen.
    pub display_name: String,
    /// Relative path to the tilemap file (TMX or custom format).
    pub tilemap_path: String,
    /// Ambient light color overlay.
    pub ambient_color: Color,
    /// Ambient light intensity [0.0, 1.0].
    pub ambient_intensity: f32,
    /// Background music path (relative to asset root). `None` = silence.
    pub music_path: Option<String>,
    /// Enemy type IDs to spawn in this zone (matched against RON data registry).
    pub enemy_spawns: Vec<String>,
    /// Zone transition triggers.
    pub transitions: Vec<ZoneTransition>,
}

impl Zone {
    /// Check if a world-space point is inside any transition trigger.
    pub fn find_transition(&self, point: Vec2) -> Option<&ZoneTransition> {
        self.transitions.iter().find(|t| {
            let [x, y, w, h] = t.trigger;
            point.x >= x && point.x <= x + w && point.y >= y && point.y <= y + h
        })
    }
}

/// Registry of all zones in the game. Stored as a resource.
pub struct ZoneRegistry {
    zones: HashMap<String, Zone>,
    /// ID of the currently active zone.
    pub active: Option<String>,
}

impl ZoneRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { zones: HashMap::new(), active: None }
    }

    /// Register a zone definition.
    pub fn register(&mut self, zone: Zone) -> &mut Self {
        self.zones.insert(zone.id.clone(), zone);
        self
    }

    /// Get a zone by ID.
    pub fn get(&self, id: &str) -> Option<&Zone> {
        self.zones.get(id)
    }

    /// Set the active zone.
    pub fn set_active(&mut self, id: impl Into<String>) {
        self.active = Some(id.into());
    }

    /// Get the currently active zone.
    pub fn active_zone(&self) -> Option<&Zone> {
        self.active.as_deref().and_then(|id| self.zones.get(id))
    }

    /// All registered zone IDs.
    pub fn zone_ids(&self) -> impl Iterator<Item = &str> {
        self.zones.keys().map(|s| s.as_str())
    }
}

impl Default for ZoneRegistry {
    fn default() -> Self { Self::new() }
}

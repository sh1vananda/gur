//! Entity inspector — displays ECS component data for selected entities.

use hecs::Entity;

/// Egui-based entity inspector window.
#[derive(Debug, Default)]
pub struct EntityInspector {
    /// Whether the inspector window is visible.
    pub visible: bool,
    /// The entity currently being inspected. `None` = no selection.
    pub selected: Option<Entity>,
    /// Filter string for the entity list search box.
    pub filter: String,
}

impl EntityInspector {
    /// Create a new, hidden inspector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select an entity for inspection.
    pub fn select(&mut self, entity: Entity) {
        self.selected = Some(entity);
        self.visible = true;
    }

    /// Deselect.
    pub fn deselect(&mut self) {
        self.selected = None;
    }

    /// Toggle visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

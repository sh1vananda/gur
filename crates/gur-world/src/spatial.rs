//! Spatial grid — fast entity proximity queries for AI and combat.

use std::collections::HashMap;
use glam::Vec2;
use hecs::Entity;

/// A uniform grid for O(1) entity insertion and O(k) radius queries,
/// where k = number of entities in nearby cells.
///
/// Cell size should match the typical entity interaction range.
pub struct SpatialGrid {
    /// Cell size in world units (pixels).
    cell_size: f32,
    /// Grid cells → entities.
    cells: HashMap<(i32, i32), Vec<Entity>>,
}

impl SpatialGrid {
    /// Create a new empty grid with the given cell size.
    pub fn new(cell_size: f32) -> Self {
        assert!(cell_size > 0.0);
        Self { cell_size, cells: HashMap::new() }
    }

    /// Clear all entities from the grid. Call at the start of each physics step.
    pub fn clear(&mut self) {
        for cell in self.cells.values_mut() {
            cell.clear();
        }
    }

    /// Insert an entity at the given world position.
    pub fn insert(&mut self, entity: Entity, position: Vec2) {
        let key = self.cell_key(position);
        self.cells.entry(key).or_default().push(entity);
    }

    /// Find all entities within `radius` of `center`.
    pub fn query_radius(&self, center: Vec2, radius: f32) -> Vec<Entity> {
        let cell_radius = (radius / self.cell_size).ceil() as i32;
        let center_key = self.cell_key(center);
        let mut result = Vec::new();

        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let key = (center_key.0 + dx, center_key.1 + dy);
                if let Some(entities) = self.cells.get(&key) {
                    result.extend_from_slice(entities);
                }
            }
        }
        result
    }

    /// Find the nearest entity to `center` within `max_range`.
    pub fn nearest(&self, center: Vec2, max_range: f32, positions: &HashMap<Entity, Vec2>) -> Option<Entity> {
        self.query_radius(center, max_range)
            .into_iter()
            .filter_map(|e| positions.get(&e).map(|p| (e, center.distance(*p))))
            .filter(|(_, d)| *d <= max_range)
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(e, _)| e)
    }

    fn cell_key(&self, pos: Vec2) -> (i32, i32) {
        (
            (pos.x / self.cell_size).floor() as i32,
            (pos.y / self.cell_size).floor() as i32,
        )
    }
}

//! ECS world wrapper around `hecs`.
//!
//! `GurWorld` extends `hecs::World` with engine-specific conveniences:
//! - Named entity lookup
//! - Resource storage (global singletons — not ECS components)
//! - Typed convenience methods with proper error propagation
//!
//! ## Resources vs Components
//! - **Component** — data attached to a specific entity (`Health`, `Transform`)
//! - **Resource** — global singleton not tied to any entity (`Time`, `InputState`)
//!
//! Resources live in a side-map keyed by `TypeId`. Access them with
//! `world.resource::<T>()` and `world.resource_mut::<T>()`.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use hecs::{Entity, World};

use crate::error::{EngineError, EngineResult};

/// Type alias used throughout the engine for clarity.
pub type EntityId = Entity;

/// The central game world: entities, components, and global resources.
///
/// Every engine system and game system receives a `&mut GurWorld` (or a
/// split borrow of parts of it). No raw `hecs::World` should escape this wrapper.
pub struct GurWorld {
    /// The underlying ECS storage (archetype-based, cache-friendly).
    pub ecs: World,
    /// Global resources — one instance per type, not tied to any entity.
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// Human-readable names for debugging (entity → name string).
    names: HashMap<Entity, String>,
}

impl Default for GurWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl GurWorld {
    /// Create an empty world.
    pub fn new() -> Self {
        Self {
            ecs: World::new(),
            resources: HashMap::new(),
            names: HashMap::new(),
        }
    }

    // ─── Entity Helpers ────────────────────────────────────────────────────

    /// Spawn an entity with a bundle of components and an optional debug name.
    ///
    /// # Example
    /// ```rust,ignore
    /// let player = world.spawn("player", (Transform::from_xy(0.0, 0.0), Health::new(100)));
    /// ```
    pub fn spawn(&mut self, name: impl Into<String>, components: impl hecs::DynamicBundle) -> Entity {
        let entity = self.ecs.spawn(components);
        self.names.insert(entity, name.into());
        entity
    }

    /// Spawn an entity without a name (use when name is not meaningful).
    pub fn spawn_anon(&mut self, components: impl hecs::DynamicBundle) -> Entity {
        self.ecs.spawn(components)
    }

    /// Despawn an entity, removing it and all its components.
    pub fn despawn(&mut self, entity: Entity) -> EngineResult<()> {
        self.names.remove(&entity);
        self.ecs.despawn(entity).map_err(|_| EngineError::NoSuchEntity(entity))
    }

    /// Returns `true` if the entity exists in the world.
    #[inline]
    pub fn exists(&self, entity: Entity) -> bool {
        self.ecs.contains(entity)
    }

    /// Get the debug name of an entity (if it was given one).
    pub fn name(&self, entity: Entity) -> Option<&str> {
        self.names.get(&entity).map(|s| s.as_str())
    }

    /// Add a component to an existing entity.
    pub fn insert<C: hecs::Component>(&mut self, entity: Entity, component: C) -> EngineResult<()> {
        self.ecs
            .insert_one(entity, component)
            .map_err(|_| EngineError::NoSuchEntity(entity))
    }

    /// Remove a component from an entity. Returns the component if present.
    pub fn remove<C: hecs::Component>(&mut self, entity: Entity) -> EngineResult<C> {
        self.ecs
            .remove_one::<C>(entity)
            .map_err(|_| EngineError::NoSuchEntity(entity))
    }

    /// Get a shared reference to a component on an entity.
    pub fn get<C: hecs::Component>(&self, entity: Entity) -> EngineResult<hecs::Ref<C>> {
        self.ecs.get::<&C>(entity).map_err(|e| match e {
            hecs::ComponentError::NoSuchEntity => EngineError::NoSuchEntity(entity),
            hecs::ComponentError::MissingComponent(_) => EngineError::MissingComponent {
                entity,
                component: std::any::type_name::<C>(),
            },
        })
    }

    /// Get a mutable reference to a component on an entity.
    pub fn get_mut<C: hecs::Component>(&self, entity: Entity) -> EngineResult<hecs::RefMut<C>> {
        self.ecs.get::<&mut C>(entity).map_err(|e| match e {
            hecs::ComponentError::NoSuchEntity => EngineError::NoSuchEntity(entity),
            hecs::ComponentError::MissingComponent(_) => EngineError::MissingComponent {
                entity,
                component: std::any::type_name::<C>(),
            },
        })
    }

    // ─── Resource API ──────────────────────────────────────────────────────

    /// Insert a global resource. Replaces any existing resource of the same type.
    pub fn insert_resource<R: Any + Send + Sync>(&mut self, resource: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(resource));
    }

    /// Remove a global resource, returning it if it existed.
    pub fn remove_resource<R: Any + Send + Sync>(&mut self) -> Option<R> {
        self.resources
            .remove(&TypeId::of::<R>())
            .and_then(|b| b.downcast::<R>().ok())
            .map(|b| *b)
    }

    /// Get a shared reference to a global resource.
    ///
    /// # Panics
    /// Panics if the resource has not been inserted. Use `try_resource` for fallible access.
    pub fn resource<R: Any + Send + Sync>(&self) -> &R {
        self.try_resource::<R>()
            .unwrap_or_else(|| panic!("Resource `{}` not found", std::any::type_name::<R>()))
    }

    /// Get a mutable reference to a global resource.
    ///
    /// # Panics
    /// Panics if the resource has not been inserted.
    pub fn resource_mut<R: Any + Send + Sync>(&mut self) -> &mut R {
        self.try_resource_mut::<R>()
            .unwrap_or_else(|| panic!("Resource `{}` not found", std::any::type_name::<R>()))
    }

    /// Fallible shared resource access — returns `None` if not present.
    pub fn try_resource<R: Any + Send + Sync>(&self) -> Option<&R> {
        self.resources
            .get(&TypeId::of::<R>())
            .and_then(|b| b.downcast_ref::<R>())
    }

    /// Fallible mutable resource access — returns `None` if not present.
    pub fn try_resource_mut<R: Any + Send + Sync>(&mut self) -> Option<&mut R> {
        self.resources
            .get_mut(&TypeId::of::<R>())
            .and_then(|b| b.downcast_mut::<R>())
    }

    /// Returns `true` if a resource of type `R` has been inserted.
    pub fn has_resource<R: Any + Send + Sync>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<R>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Health(u32);

    #[derive(Debug, PartialEq)]
    struct Gold(u32);

    #[test]
    fn spawn_and_get_component() {
        let mut world = GurWorld::new();
        let e = world.spawn("hero", (Health(100),));
        let hp = world.get::<Health>(e).unwrap();
        assert_eq!(hp.0, 100);
    }

    #[test]
    fn despawn_removes_entity() {
        let mut world = GurWorld::new();
        let e = world.spawn_anon((Health(50),));
        assert!(world.exists(e));
        world.despawn(e).unwrap();
        assert!(!world.exists(e));
    }

    #[test]
    fn resources_insert_and_retrieve() {
        let mut world = GurWorld::new();
        world.insert_resource(Gold(999));
        assert_eq!(world.resource::<Gold>().0, 999);
        world.resource_mut::<Gold>().0 = 42;
        assert_eq!(world.resource::<Gold>().0, 42);
    }

    #[test]
    fn missing_resource_returns_none() {
        let world = GurWorld::new();
        assert!(world.try_resource::<Gold>().is_none());
    }
}

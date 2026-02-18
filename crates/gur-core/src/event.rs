//! Typed event bus — decoupled message passing between engine systems.
//!
//! The event bus is the nervous system of the engine. Systems never call each
//! other directly; instead they emit events and react to events. This keeps
//! systems independent and testable in isolation.
//!
//! ## Two Dispatch Modes
//!
//! ### Immediate (`emit`)
//! The event is processed by the **next system that reads it in the same frame**.
//! Use for: combat hits, sound triggers, animation state changes.
//!
//! ### Deferred (`emit_deferred`)
//! The event is queued and processed at the **start of the next frame**.
//! Use for: zone transitions, cutscene starts, save triggers.
//!
//! ## Usage
//! ```rust,ignore
//! // In combat system:
//! bus.emit(DamageEvent { target: enemy, amount: 42 });
//!
//! // In health system:
//! for evt in bus.read::<DamageEvent>() {
//!     apply_damage(world, evt.target, evt.amount);
//! }
//! ```
//!
//! ## LOAD-BEARING
//! Event types are keyed by `TypeId`. If an event struct is renamed or moved,
//! any persisted event queues (e.g., from save files) will fail to deserialize.
//! Do not persist raw events — persist their semantic effect instead.

use std::any::{Any, TypeId};
use std::collections::HashMap;

// ─── Untyped storage ─────────────────────────────────────────────────────────

/// Internal untyped event queue for one event type.
struct RawQueue {
    /// Events emitted this frame (available for reading immediately).
    current: Vec<Box<dyn Any + Send + Sync>>,
    /// Events to become available next frame.
    deferred: Vec<Box<dyn Any + Send + Sync>>,
}

impl RawQueue {
    fn new() -> Self {
        Self {
            current: Vec::new(),
            deferred: Vec::new(),
        }
    }

    /// Move deferred events into the current queue and clear current.
    /// Called once per frame before systems run.
    fn advance(&mut self) {
        self.current.clear();
        std::mem::swap(&mut self.current, &mut self.deferred);
    }
}

// ─── EventBus ────────────────────────────────────────────────────────────────

/// The central event bus. Stored as a resource in `GurWorld`.
///
/// Every engine crate and game system reads/writes through this single bus.
/// Thread-safety: the bus is accessed from the main thread only. For
/// cross-thread emission, use a channel and flush into the bus on `PreUpdate`.
pub struct EventBus {
    queues: HashMap<TypeId, RawQueue>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create an empty event bus.
    pub fn new() -> Self {
        Self {
            queues: HashMap::new(),
        }
    }

    // ─── Writing ─────────────────────────────────────────────────────────

    /// Emit an event that is readable by systems running **later this frame**.
    pub fn emit<E: Any + Send + Sync>(&mut self, event: E) {
        self.queue_for::<E>().current.push(Box::new(event));
    }

    /// Emit an event that becomes readable at the **start of the next frame**.
    pub fn emit_deferred<E: Any + Send + Sync>(&mut self, event: E) {
        self.queue_for::<E>().deferred.push(Box::new(event));
    }

    // ─── Reading ─────────────────────────────────────────────────────────

    /// Read all current-frame events of type `E`.
    ///
    /// Returns an iterator over shared references. Events are **not** consumed —
    /// multiple systems can read the same event in the same frame.
    pub fn read<E: Any + Send + Sync>(&self) -> impl Iterator<Item = &E> {
        self.queues
            .get(&TypeId::of::<E>())
            .into_iter()
            .flat_map(|q| q.current.iter())
            .filter_map(|b| b.downcast_ref::<E>())
    }

    /// Returns `true` if there are any current-frame events of type `E`.
    pub fn has<E: Any + Send + Sync>(&self) -> bool {
        self.queues
            .get(&TypeId::of::<E>())
            .map(|q| !q.current.is_empty())
            .unwrap_or(false)
    }

    /// Count of current-frame events of type `E`.
    pub fn count<E: Any + Send + Sync>(&self) -> usize {
        self.queues
            .get(&TypeId::of::<E>())
            .map(|q| q.current.len())
            .unwrap_or(0)
    }

    // ─── Frame Lifecycle ─────────────────────────────────────────────────

    /// Advance all queues: promote deferred events, clear immediate events.
    ///
    /// Call this **once per frame** in `PreUpdate`, before any systems run.
    pub fn advance_frame(&mut self) {
        for queue in self.queues.values_mut() {
            queue.advance();
        }
    }

    /// Clear all events of every type. Use for testing or scene resets.
    pub fn clear_all(&mut self) {
        for queue in self.queues.values_mut() {
            queue.current.clear();
            queue.deferred.clear();
        }
    }

    // ─── Private ─────────────────────────────────────────────────────────

    fn queue_for<E: Any + Send + Sync>(&mut self) -> &mut RawQueue {
        self.queues
            .entry(TypeId::of::<E>())
            .or_insert_with(RawQueue::new)
    }
}

// ─── Typed wrappers (ergonomic API) ──────────────────────────────────────────

/// Ergonomic write-only view into the bus for a specific event type.
///
/// Useful for passing to systems that only emit, not read.
pub struct EventWriter<'a, E: Any + Send + Sync> {
    bus: &'a mut EventBus,
    _phantom: std::marker::PhantomData<E>,
}

impl<'a, E: Any + Send + Sync> EventWriter<'a, E> {
    /// Create a writer bound to the given bus.
    pub fn new(bus: &'a mut EventBus) -> Self {
        Self { bus, _phantom: std::marker::PhantomData }
    }

    /// Emit an immediate event.
    pub fn emit(&mut self, event: E) {
        self.bus.emit(event);
    }

    /// Emit a deferred event.
    pub fn emit_deferred(&mut self, event: E) {
        self.bus.emit_deferred(event);
    }
}

/// Ergonomic read-only view into the bus for a specific event type.
pub struct EventReader<'a, E: Any + Send + Sync> {
    bus: &'a EventBus,
    _phantom: std::marker::PhantomData<E>,
}

impl<'a, E: Any + Send + Sync> EventReader<'a, E> {
    /// Create a reader bound to the given bus.
    pub fn new(bus: &'a EventBus) -> Self {
        Self { bus, _phantom: std::marker::PhantomData }
    }

    /// Iterate over all current-frame events.
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.bus.read::<E>()
    }

    /// Returns `true` if any events of this type exist this frame.
    pub fn has_events(&self) -> bool {
        self.bus.has::<E>()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct DamageEvent {
        amount: u32,
    }

    #[derive(Debug, PartialEq)]
    struct ZoneChangeEvent {
        zone: &'static str,
    }

    #[test]
    fn immediate_emit_readable_same_frame() {
        let mut bus = EventBus::new();
        bus.emit(DamageEvent { amount: 42 });
        let events: Vec<_> = bus.read::<DamageEvent>().collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 42);
    }

    #[test]
    fn deferred_emit_only_readable_next_frame() {
        let mut bus = EventBus::new();
        bus.emit_deferred(ZoneChangeEvent { zone: "ashlands" });
        // Not readable yet
        assert!(!bus.has::<ZoneChangeEvent>());
        // Advance frame
        bus.advance_frame();
        // Now readable
        let events: Vec<_> = bus.read::<ZoneChangeEvent>().collect();
        assert_eq!(events[0].zone, "ashlands");
    }

    #[test]
    fn advance_frame_clears_immediate_events() {
        let mut bus = EventBus::new();
        bus.emit(DamageEvent { amount: 10 });
        assert_eq!(bus.count::<DamageEvent>(), 1);
        bus.advance_frame();
        // Immediate events are cleared after frame advance
        assert_eq!(bus.count::<DamageEvent>(), 0);
    }

    #[test]
    fn multiple_events_same_type() {
        let mut bus = EventBus::new();
        bus.emit(DamageEvent { amount: 1 });
        bus.emit(DamageEvent { amount: 2 });
        bus.emit(DamageEvent { amount: 3 });
        let total: u32 = bus.read::<DamageEvent>().map(|e| e.amount).sum();
        assert_eq!(total, 6);
    }

    #[test]
    fn event_writer_and_reader() {
        let mut bus = EventBus::new();
        {
            let mut writer = EventWriter::<DamageEvent>::new(&mut bus);
            writer.emit(DamageEvent { amount: 99 });
        }
        let reader = EventReader::<DamageEvent>::new(&bus);
        assert!(reader.has_events());
        let v: Vec<_> = reader.iter().collect();
        assert_eq!(v[0].amount, 99);
    }
}

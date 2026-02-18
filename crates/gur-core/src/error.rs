//! Engine-wide error types.
//!
//! All public library functions return `EngineResult<T>` rather than panicking.
//! Game-layer code may use `anyhow::Result` for convenience.

use thiserror::Error;

/// The canonical error type for engine operations.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A system attempted to access an entity that does not exist.
    #[error("Entity {0:?} does not exist in the world")]
    NoSuchEntity(hecs::Entity),

    /// A component query failed because the entity lacks a required component.
    #[error("Entity {entity:?} is missing component `{component}`")]
    MissingComponent {
        /// The entity that was queried.
        entity: hecs::Entity,
        /// The name of the missing component type.
        component: &'static str,
    },

    /// An asset failed to load.
    #[error("Failed to load asset at `{path}`: {reason}")]
    AssetLoad {
        /// The asset path that was requested.
        path: String,
        /// Human-readable load failure reason.
        reason: String,
    },

    /// A plugin attempted to register itself twice.
    #[error("Plugin `{0}` has already been registered")]
    DuplicatePlugin(&'static str),

    /// A system stage name was not found in the schedule.
    #[error("System stage `{0}` not found in schedule")]
    UnknownStage(String),

    /// An event channel is disconnected (sender/receiver dropped).
    #[error("Event channel `{0}` is disconnected")]
    EventChannelDisconnected(String),

    /// A serialization or deserialization error occurred.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// A generic I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for errors from external crates wrapped with context.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias used throughout the engine.
pub type EngineResult<T> = Result<T, EngineError>;

impl EngineError {
    /// Wrap any `Display`-able error as `EngineError::Other`.
    pub fn other(msg: impl std::fmt::Display) -> Self {
        Self::Other(msg.to_string())
    }
}

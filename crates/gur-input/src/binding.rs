//! Input binding types — the bridge between physical devices and [`Action`]s.
//!
//! `winit::keyboard::KeyCode` does not implement `Serialize/Deserialize/Hash/Eq`,
//! so we serialise keyboard bindings as their debug string and re-parse on load.

use serde::{Deserialize, Serialize};

/// A physical input source that can be bound to an [`Action`].
///
/// For serialisation we store the `KeyCode` as its `Debug` string (e.g. `"KeyW"`).
/// This is robust to winit version changes and works with RON/JSON config files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Binding {
    /// A keyboard key, stored as the `Debug` name of `winit::keyboard::KeyCode`.
    Key(String),
    /// A mouse button (0=left, 1=right, 2=middle).
    MouseButton(u32),
    /// A gamepad button (SDL-style index).
    GamepadButton(u32),
    /// A gamepad axis threshold.
    GamepadAxis { axis: u32, positive: bool, threshold: f32 },
}

impl Binding {
    /// Convenience: bind to a keyboard key by its debug name.
    pub fn key(code: winit::keyboard::KeyCode) -> Self {
        Self::Key(format!("{:?}", code))
    }

    /// Check if this binding matches the given winit key code.
    pub fn matches_key(&self, code: &winit::keyboard::KeyCode) -> bool {
        if let Self::Key(name) = self {
            name == &format!("{:?}", code)
        } else {
            false
        }
    }

    /// Convenience: bind to a gamepad button by index.
    pub fn gamepad(button: u32) -> Self {
        Self::GamepadButton(button)
    }
}

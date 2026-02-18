//! [`InputMap`] — action-to-binding registry, hot-reloadable at runtime.

use std::collections::HashMap;
use winit::keyboard::KeyCode;

use crate::action::Action;
use crate::binding::Binding;

/// Maps [`Action`]s to one or more physical [`Binding`]s.
///
/// Multiple bindings per action are supported (e.g. WASD + arrow keys both map
/// to movement actions). The first active binding wins.
#[derive(Debug, Clone, Default)]
pub struct InputMap {
    /// action → list of accepted bindings (primary first).
    bindings: HashMap<Action, Vec<Binding>>,
}

impl InputMap {
    /// Create an empty map (no bindings).
    pub fn empty() -> Self {
        Self { bindings: HashMap::new() }
    }

    /// Load the default keyboard bindings used by the game.
    ///
    /// Design note: these defaults should feel natural for both WASD and arrow-key players.
    /// Gamepad support will layer on top of these without replacing them.
    pub fn default_bindings() -> Self {
        let mut map = Self::empty();

        // Movement — WASD primary, arrow keys secondary
        map.bind(Action::MoveUp,    Binding::key(KeyCode::KeyW));
        map.bind(Action::MoveUp,    Binding::key(KeyCode::ArrowUp));
        map.bind(Action::MoveDown,  Binding::key(KeyCode::KeyS));
        map.bind(Action::MoveDown,  Binding::key(KeyCode::ArrowDown));
        map.bind(Action::MoveLeft,  Binding::key(KeyCode::KeyA));
        map.bind(Action::MoveLeft,  Binding::key(KeyCode::ArrowLeft));
        map.bind(Action::MoveRight, Binding::key(KeyCode::KeyD));
        map.bind(Action::MoveRight, Binding::key(KeyCode::ArrowRight));

        // Combat
        map.bind(Action::AttackLight,  Binding::key(KeyCode::KeyZ));
        map.bind(Action::AttackHeavy,  Binding::key(KeyCode::KeyX));
        map.bind(Action::Dodge,        Binding::key(KeyCode::Space));
        map.bind(Action::Guard,        Binding::key(KeyCode::ShiftLeft));
        map.bind(Action::UseItem,      Binding::key(KeyCode::KeyQ));
        map.bind(Action::CycleItemLeft,  Binding::key(KeyCode::BracketLeft));
        map.bind(Action::CycleItemRight, Binding::key(KeyCode::BracketRight));
        map.bind(Action::LockOn,       Binding::key(KeyCode::KeyR));

        // Interaction
        map.bind(Action::Interact,        Binding::key(KeyCode::KeyE));
        map.bind(Action::Cancel,          Binding::key(KeyCode::Escape));
        map.bind(Action::DialogueAdvance, Binding::key(KeyCode::KeyZ));

        // Menus
        map.bind(Action::PauseMenu, Binding::key(KeyCode::Escape));
        map.bind(Action::Inventory, Binding::key(KeyCode::KeyI));
        map.bind(Action::Map,       Binding::key(KeyCode::KeyM));

        // Debug
        #[cfg(debug_assertions)]
        map.bind(Action::DevConsole, Binding::key(KeyCode::Backquote));

        map
    }

    /// Add a binding for an action. Multiple bindings per action are allowed.
    pub fn bind(&mut self, action: Action, binding: Binding) -> &mut Self {
        self.bindings.entry(action).or_default().push(binding);
        self
    }

    /// Replace all bindings for an action.
    pub fn rebind(&mut self, action: Action, bindings: Vec<Binding>) {
        self.bindings.insert(action, bindings);
    }

    /// Remove all bindings for an action.
    pub fn unbind(&mut self, action: Action) {
        self.bindings.remove(&action);
    }

    /// Get all bindings registered for an action.
    pub fn get(&self, action: &Action) -> &[Binding] {
        self.bindings.get(action).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Total number of bindings registered across all actions.
    pub fn binding_count(&self) -> usize {
        self.bindings.values().map(|v| v.len()).sum()
    }

    /// Iterate over all (action, bindings) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Action, &Vec<Binding>)> {
        self.bindings.iter()
    }
}

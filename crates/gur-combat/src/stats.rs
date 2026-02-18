//! Character stats and the modifier stack system.
//!
//! Every stat (HP, stamina, strength, etc.) has a base value and a stack of
//! temporary modifiers (buffs/debuffs). The effective value is:
//!   `base * product(multiplicative) + sum(additive) - sum(reductions)`

use serde::{Deserialize, Serialize};

// ─── Modifier ────────────────────────────────────────────────────────────────

/// How a modifier changes a stat value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModifierKind {
    /// Add a flat value to the stat.
    Additive(f32),
    /// Multiply the stat by a factor (e.g. 1.2 = +20%).
    Multiplicative(f32),
    /// Override: clamp the stat to a maximum value.
    Cap(f32),
}

/// A single buff or debuff applied to a stat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatModifier {
    /// Unique tag for removal (e.g. "ember_flame_buff", "frost_slow").
    pub tag: String,
    /// How this modifier changes the value.
    pub kind: ModifierKind,
    /// Remaining duration in fixed steps. `None` = permanent until explicitly removed.
    pub duration: Option<u32>,
}

impl StatModifier {
    /// Create a permanent additive modifier.
    pub fn additive(tag: impl Into<String>, value: f32) -> Self {
        Self { tag: tag.into(), kind: ModifierKind::Additive(value), duration: None }
    }

    /// Create a timed multiplicative modifier.
    pub fn multiplicative(tag: impl Into<String>, factor: f32, duration_steps: u32) -> Self {
        Self {
            tag: tag.into(),
            kind: ModifierKind::Multiplicative(factor),
            duration: Some(duration_steps),
        }
    }
}

// ─── ModifierStack ────────────────────────────────────────────────────────────

/// A list of active modifiers for a single stat.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModifierStack {
    modifiers: Vec<StatModifier>,
}

impl ModifierStack {
    /// Add a modifier to the stack.
    pub fn push(&mut self, modifier: StatModifier) {
        self.modifiers.push(modifier);
    }

    /// Remove a modifier by tag.
    pub fn remove(&mut self, tag: &str) {
        self.modifiers.retain(|m| m.tag != tag);
    }

    /// Tick all timed modifiers, removing expired ones.
    pub fn tick(&mut self) {
        for m in self.modifiers.iter_mut() {
            if let Some(ref mut dur) = m.duration {
                *dur = dur.saturating_sub(1);
            }
        }
        self.modifiers.retain(|m| m.duration.map(|d| d > 0).unwrap_or(true));
    }

    /// Apply all modifiers to a base value and return the effective value.
    pub fn apply(&self, base: f32) -> f32 {
        let mut value = base;
        // 1. Additive flat bonuses
        for m in &self.modifiers {
            if let ModifierKind::Additive(v) = m.kind { value += v; }
        }
        // 2. Multiplicative bonuses
        for m in &self.modifiers {
            if let ModifierKind::Multiplicative(f) = m.kind { value *= f; }
        }
        // 3. Caps
        for m in &self.modifiers {
            if let ModifierKind::Cap(c) = m.kind { value = value.min(c); }
        }
        value.max(0.0) // stats never go negative
    }
}

// ─── Stats ────────────────────────────────────────────────────────────────────

/// Full character stats — health, stamina, damage scaling, resistances.
///
/// Attach this as an ECS component to any entity that participates in combat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    // ── Vitality ─────────────────────────────────────────────────────────
    /// Maximum hit points.
    pub max_hp: f32,
    /// Current hit points.
    pub current_hp: f32,

    // ── Stamina ───────────────────────────────────────────────────────────
    /// Maximum stamina.
    pub max_stamina: f32,
    /// Current stamina.
    pub current_stamina: f32,
    /// Stamina regeneration per fixed step (when not attacking).
    pub stamina_regen: f32,

    // ── Damage Scaling ────────────────────────────────────────────────────
    /// Strength (scales physical damage).
    pub strength: f32,
    /// Dexterity (scales dex weapons, attack speed).
    pub dexterity: f32,
    /// Intelligence (scales magic damage).
    pub intelligence: f32,

    // ── Defense ───────────────────────────────────────────────────────────
    /// Flat physical damage reduction.
    pub defense: f32,
    /// Poise (stagger resistance). Zero = staggers on any hit.
    pub max_poise: f32,
    /// Current poise.
    pub current_poise: f32,
    /// Poise recovery per fixed step.
    pub poise_regen: f32,

    // ── Move Speed ────────────────────────────────────────────────────────
    /// Base movement speed in units/second.
    pub move_speed: f32,

    // ── Modifier stacks (one per tracked stat) ────────────────────────────
    /// Active modifiers on move speed.
    #[serde(default)]
    pub speed_modifiers: ModifierStack,
    /// Active modifiers on defense.
    #[serde(default)]
    pub defense_modifiers: ModifierStack,
    /// Active modifiers on strength.
    #[serde(default)]
    pub strength_modifiers: ModifierStack,
}

impl Stats {
    /// Reasonable defaults for a starting player character.
    pub fn player_default() -> Self {
        Self {
            max_hp:            100.0,
            current_hp:        100.0,
            max_stamina:        80.0,
            current_stamina:    80.0,
            stamina_regen:       0.4, // per fixed step (~24/s at 60Hz)
            strength:           10.0,
            dexterity:           8.0,
            intelligence:        5.0,
            defense:             5.0,
            max_poise:          50.0,
            current_poise:      50.0,
            poise_regen:         0.5,
            move_speed:        120.0,
            speed_modifiers:   ModifierStack::default(),
            defense_modifiers: ModifierStack::default(),
            strength_modifiers: ModifierStack::default(),
        }
    }

    /// Reasonable defaults for a basic enemy.
    pub fn enemy_default() -> Self {
        Self {
            max_hp:           60.0,
            current_hp:       60.0,
            max_stamina:      40.0,
            current_stamina:  40.0,
            stamina_regen:     0.2,
            strength:          8.0,
            dexterity:         6.0,
            intelligence:      2.0,
            defense:           3.0,
            max_poise:        30.0,
            current_poise:    30.0,
            poise_regen:       0.3,
            move_speed:       80.0,
            speed_modifiers:   ModifierStack::default(),
            defense_modifiers: ModifierStack::default(),
            strength_modifiers: ModifierStack::default(),
        }
    }

    /// Returns `true` if the entity is alive.
    #[inline]
    pub fn is_alive(&self) -> bool { self.current_hp > 0.0 }

    /// Returns `true` if the entity has enough stamina for an action.
    #[inline]
    pub fn has_stamina(&self, cost: f32) -> bool { self.current_stamina >= cost }

    /// Drain stamina. Clamps to zero.
    pub fn spend_stamina(&mut self, amount: f32) {
        self.current_stamina = (self.current_stamina - amount).max(0.0);
    }

    /// Apply flat damage after defense reduction. Returns actual damage dealt.
    pub fn apply_damage(&mut self, raw_damage: f32) -> f32 {
        let effective_defense = self.defense_modifiers.apply(self.defense);
        let actual = (raw_damage - effective_defense).max(1.0); // minimum 1 damage
        self.current_hp = (self.current_hp - actual).max(0.0);
        actual
    }

    /// Apply a heal (clamped to max_hp).
    pub fn apply_heal(&mut self, amount: f32) {
        self.current_hp = (self.current_hp + amount).min(self.max_hp);
    }

    /// Tick per-step regeneration and modifier durations.
    pub fn tick(&mut self) {
        // Stamina regen
        self.current_stamina = (self.current_stamina + self.stamina_regen).min(self.max_stamina);
        // Poise regen
        self.current_poise = (self.current_poise + self.poise_regen).min(self.max_poise);
        // Tick modifiers
        self.speed_modifiers.tick();
        self.defense_modifiers.tick();
        self.strength_modifiers.tick();
    }

    /// Effective move speed (base + modifiers).
    #[inline]
    pub fn effective_speed(&self) -> f32 {
        self.speed_modifiers.apply(self.move_speed)
    }
}

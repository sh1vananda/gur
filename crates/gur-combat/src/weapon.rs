//! Weapon definitions — fully code-configurable.
//!
//! Weapons are pure data. The combat system reads weapon data when resolving
//! an attack action and builds the appropriate hitbox + effect chain.
//!
//! Weapons can also be defined in RON files and loaded by `gur-scripting`.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::effect::Effect;

/// Broad category of weapon — affects animation set and move options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponClass {
    /// Short, fast, low damage. Two-hit light combo.
    Dagger,
    /// Balanced. Standard 3-hit light combo.
    Sword,
    /// Slow, high damage, high poise damage. 1–2 hits.
    Greatsword,
    /// Long range, low stagger. Piercing.
    Spear,
    /// Area swing, high poise damage.
    Axe,
    /// Magic catalyst. Fires projectiles.
    Staff,
    /// Ranged. Consumes arrows resource.
    Bow,
    /// Bare hands. Lowest damage, highest stagger control.
    Fist,
}

/// Scaling coefficients for a weapon's damage formula.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageScaling {
    /// Flat base damage before scaling.
    pub base: f32,
    /// Strength coefficient (multiplied by entity's strength stat).
    pub strength: f32,
    /// Dexterity coefficient.
    pub dexterity: f32,
    /// Intelligence coefficient.
    pub intelligence: f32,
}

impl DamageScaling {
    /// A pure strength weapon (greatsword, axe).
    pub fn strength_weapon(base: f32, coeff: f32) -> Self {
        Self { base, strength: coeff, dexterity: 0.0, intelligence: 0.0 }
    }

    /// A pure dex weapon (dagger, spear).
    pub fn dex_weapon(base: f32, coeff: f32) -> Self {
        Self { base, strength: 0.0, dexterity: coeff, intelligence: 0.0 }
    }

    /// Calculate total damage given entity stats.
    pub fn calculate(&self, strength: f32, dexterity: f32, intelligence: f32) -> f32 {
        self.base
            + self.strength * strength
            + self.dexterity * dexterity
            + self.intelligence * intelligence
    }
}

/// A complete weapon definition.
///
/// Define weapons in Rust code or load from RON files:
/// ```ron
/// // assets/data/weapons/greatsword.ron
/// Weapon(
///     name: "Iron Greatsword",
///     class: Greatsword,
///     damage: DamageScaling(base: 40.0, strength: 1.2, dexterity: 0.0, intelligence: 0.0),
///     stamina_cost: 28.0,
///     hitbox_size: (48.0, 16.0),
///     hitbox_offset: (24.0, 0.0),
///     poise_damage: 40.0,
///     swing_startup_frames: 8,
///     swing_active_frames: 6,
///     swing_recovery_frames: 12,
/// )
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    /// Display name.
    pub name: String,
    /// Weapon class (determines animation set).
    pub class: WeaponClass,
    /// Damage formula.
    pub damage: DamageScaling,
    /// Stamina cost per light attack swing.
    pub stamina_cost: f32,
    /// Hitbox size (width, height) in world units.
    pub hitbox_size: Vec2,
    /// Hitbox offset from entity center (positive = in front).
    pub hitbox_offset: Vec2,
    /// Poise damage per hit.
    pub poise_damage: f32,
    /// Frames before hitbox activates (wind-up).
    pub swing_startup_frames: u32,
    /// Frames the hitbox is active.
    pub swing_active_frames: u32,
    /// Frames after swing before next action (recovery / end-lag).
    pub swing_recovery_frames: u32,
    /// Additional effects applied on hit (bleed, slow, etc.).
    pub on_hit_effects: Vec<Effect>,
}

impl Weapon {
    /// Total frame duration of a full swing.
    pub fn total_frames(&self) -> u32 {
        self.swing_startup_frames + self.swing_active_frames + self.swing_recovery_frames
    }

    /// The Iron Sword — balanced starter weapon.
    pub fn iron_sword() -> Self {
        Self {
            name: "Iron Sword".into(),
            class: WeaponClass::Sword,
            damage: DamageScaling { base: 20.0, strength: 0.8, dexterity: 0.6, intelligence: 0.0 },
            stamina_cost: 18.0,
            hitbox_size: Vec2::new(36.0, 14.0),
            hitbox_offset: Vec2::new(18.0, 0.0),
            poise_damage: 20.0,
            swing_startup_frames: 6,
            swing_active_frames: 5,
            swing_recovery_frames: 10,
            on_hit_effects: vec![],
        }
    }

    /// Greatsword — slow but devastating.
    pub fn greatsword() -> Self {
        Self {
            name: "Iron Greatsword".into(),
            class: WeaponClass::Greatsword,
            damage: DamageScaling { base: 45.0, strength: 1.5, dexterity: 0.2, intelligence: 0.0 },
            stamina_cost: 30.0,
            hitbox_size: Vec2::new(56.0, 20.0),
            hitbox_offset: Vec2::new(28.0, 0.0),
            poise_damage: 50.0,
            swing_startup_frames: 14,
            swing_active_frames: 8,
            swing_recovery_frames: 18,
            on_hit_effects: vec![],
        }
    }
}

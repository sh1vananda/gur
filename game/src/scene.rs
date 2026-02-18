//! The Ashlands — complete game scene.
//!
//! Implements: tilemap, player (WASD + attack + dodge + stamina),
//! enemy AI (patrol → aggro → attack), camera follow, HUD, screenshake.
//! All rendering uses coloured quads via the 1×1 white fallback texture.

use glam::Vec2;
use gur_render::{
    Color, WgpuBackend, CANVAS_WIDTH, CANVAS_HEIGHT,
    sprite::{SpriteDrawCall, SpriteFlip},
};

// ─── Map constants ───────────────────────────────────────────────────────────
pub const TILE_SIZE: f32 = 16.0;
pub const MAP_W:     usize = 20;
pub const MAP_H:     usize = 11;

pub const T_VOID:   u8 = 0;
pub const T_GROUND: u8 = 1;
pub const T_WALL:   u8 = 2;
pub const T_EMBER:  u8 = 3;  // glowing ground

#[rustfmt::skip]
pub const ASHLANDS: [u8; MAP_W * MAP_H] = [
    // row 0  – top border
    2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
    // row 1
    2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,2,
    // row 2  – pillars left
    2,1,1,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,2,
    // row 3  – ember zone centre
    2,1,1,2,2,2,1,1,3,3,3,1,1,1,1,1,1,1,1,2,
    // row 4
    2,1,1,1,1,1,1,1,3,3,3,1,1,1,1,1,1,1,1,2,
    // row 5  – open corridor
    2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,2,
    // row 6  – twin barriers
    2,1,1,1,1,1,2,2,2,1,1,1,1,1,2,2,2,1,1,2,
    // row 7
    2,1,1,1,1,1,2,2,2,1,1,1,1,1,2,2,2,1,1,2,
    // row 8
    2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,2,
    // row 9  – ember alcove
    2,1,1,3,3,3,1,1,1,1,1,1,1,1,1,1,1,1,1,2,
    // row 10 – bottom border
    2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
];

fn tile_at(map: &[u8; MAP_W * MAP_H], tx: i32, ty: i32) -> u8 {
    if tx < 0 || ty < 0 || tx >= MAP_W as i32 || ty >= MAP_H as i32 {
        return T_WALL;
    }
    map[(ty as usize) * MAP_W + (tx as usize)]
}

fn is_solid(t: u8) -> bool { matches!(t, T_WALL | T_VOID) }

// ─── Input snapshot ──────────────────────────────────────────────────────────
#[derive(Default, Clone)]
pub struct Input {
    pub up:     bool,
    pub down:   bool,
    pub left:   bool,
    pub right:  bool,
    /// Z / J — light attack (just pressed this frame)
    pub attack: bool,
    /// X / K — dodge roll (just pressed this frame)
    pub dodge:  bool,
}

// ─── Player ──────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
pub enum PlayerState {
    Idle,
    Walking,
    /// Windup(0.12s) → Active(0.1s) → Recovery(0.18s)
    Attacking { phase: u8, timer: f32 },
    Dodging   { timer: f32 },
    Dead,
}

pub struct Player {
    pub pos:      Vec2,
    pub vel:      Vec2,
    pub facing:   f32,      // +1 right / -1 left
    pub state:    PlayerState,
    pub hp:       f32,
    pub max_hp:   f32,
    pub stamina:  f32,
    pub max_sta:  f32,
    pub i_frames: f32,
    pub combo:    u8,       // 0-2 combo hit index
    pub flash:    f32,      // damage flash timer
}

impl Player {
    fn new(x: f32, y: f32) -> Self {
        Self {
            pos: Vec2::new(x, y), vel: Vec2::ZERO, facing: 1.0,
            state: PlayerState::Idle,
            hp: 100.0, max_hp: 100.0,
            stamina: 100.0, max_sta: 100.0,
            i_frames: 0.0, combo: 0, flash: 0.0,
        }
    }

    pub fn alive(&self) -> bool { !matches!(self.state, PlayerState::Dead) }

    fn half_w(&self) -> f32 { 5.0 }
    fn half_h(&self) -> f32 { 6.0 }

    /// AABB in world pixels
    fn aabb(&self) -> (Vec2, Vec2) {
        (self.pos - Vec2::new(self.half_w(), self.half_h()),
         self.pos + Vec2::new(self.half_w(), self.half_h()))
    }

    /// Attack hitbox (rectangle in front of player)
    fn attack_box(&self) -> (Vec2, Vec2) {
        let offset = self.facing * (self.half_w() + 2.0);
        let cx = self.pos.x + offset;
        let cy = self.pos.y;
        (Vec2::new(cx - 6.0, cy - 8.0), Vec2::new(cx + 6.0, cy + 8.0))
    }
}

// ─── Enemy ───────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
pub enum EnemyState {
    Patrol { dir: f32, timer: f32 },
    Chase,
    Windup   { timer: f32 },
    Striking { timer: f32 },
    Recovery { timer: f32 },
    Stunned  { timer: f32 },
    Dead,
}

pub struct Enemy {
    pub pos:      Vec2,
    pub facing:   f32,
    pub state:    EnemyState,
    pub hp:       f32,
    pub max_hp:   f32,
    pub flash:    f32,   // red-flash on hit
    home:         Vec2,  // patrol origin
    aggro_r:      f32,
    atk_r:        f32,
}

impl Enemy {
    fn new(x: f32, y: f32) -> Self {
        Self {
            pos: Vec2::new(x, y), facing: -1.0,
            state: EnemyState::Patrol { dir: 1.0, timer: 1.5 },
            hp: 60.0, max_hp: 60.0, flash: 0.0,
            home: Vec2::new(x, y),
            aggro_r: 64.0, atk_r: 14.0,
        }
    }

    pub fn alive(&self) -> bool { !matches!(self.state, EnemyState::Dead) }

    fn half_w(&self) -> f32 { 5.0 }
    fn half_h(&self) -> f32 { 7.0 }

    fn aabb(&self) -> (Vec2, Vec2) {
        (self.pos - Vec2::new(self.half_w(), self.half_h()),
         self.pos + Vec2::new(self.half_w(), self.half_h()))
    }
}

fn aabb_overlap(a: (Vec2,Vec2), b: (Vec2,Vec2)) -> bool {
    a.0.x < b.1.x && a.1.x > b.0.x &&
    a.0.y < b.1.y && a.1.y > b.0.y
}

// ─── Scene ───────────────────────────────────────────────────────────────────
pub struct Scene {
    pub map:      [u8; MAP_W * MAP_H],
    pub player:   Player,
    pub enemies:  Vec<Enemy>,
    pub camera:   Vec2,
    shake:        f32,
    shake_mag:    f32,
    pub time:     f32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            map: ASHLANDS,
            player:  Player::new(48.0, 88.0),
            enemies: vec![
                Enemy::new(200.0, 88.0),
                Enemy::new(280.0, 56.0),
                Enemy::new(280.0, 136.0),
            ],
            camera:    Vec2::new(160.0, 90.0),
            shake:     0.0,
            shake_mag: 0.0,
            time:      0.0,
        }
    }

    // ── Update ────────────────────────────────────────────────────────────
    pub fn update(&mut self, dt: f32, input: &Input) {
        self.time += dt;

        // Decay shake
        self.shake = (self.shake - dt).max(0.0);

        if self.player.alive() {
            self.update_player(dt, input);
        }
        self.update_enemies(dt);
        self.resolve_enemy_attacks();
        self.update_camera(dt);
    }

    fn update_player(&mut self, dt: f32, input: &Input) {
        // ── Timers ────────────────────────────────────────────────────────
        self.player.i_frames = (self.player.i_frames - dt).max(0.0);
        self.player.flash    = (self.player.flash    - dt).max(0.0);

        // ── State machine (PlayerState is Copy — match copies it) ─────────
        let mut do_attack = false;

        match self.player.state {
            PlayerState::Idle | PlayerState::Walking => {
                let mut dir = Vec2::ZERO;
                if input.up    { dir.y -= 1.0; }
                if input.down  { dir.y += 1.0; }
                if input.left  { dir.x -= 1.0; self.player.facing = -1.0; }
                if input.right { dir.x += 1.0; self.player.facing =  1.0; }
                if dir.length_squared() > 0.0 { dir = dir.normalize(); }

                self.player.vel = dir * 55.0;
                self.player.state = if dir.length_squared() > 0.01 {
                    PlayerState::Walking
                } else {
                    PlayerState::Idle
                };

                if input.attack && self.player.stamina >= 15.0 {
                    self.player.stamina -= 15.0;
                    self.player.vel = Vec2::ZERO;
                    self.player.state = PlayerState::Attacking { phase: 0, timer: 0.12 };
                }
                if input.dodge && self.player.stamina >= 25.0 {
                    self.player.stamina -= 25.0;
                    self.player.i_frames = 0.4;
                    let dodge_dir = if dir.length_squared() > 0.01 {
                        dir
                    } else {
                        Vec2::new(self.player.facing, 0.0)
                    };
                    self.player.vel = dodge_dir * 120.0;
                    self.player.state = PlayerState::Dodging { timer: 0.22 };
                }
            }

            PlayerState::Attacking { phase, timer } => {
                let t = timer - dt;
                if t <= 0.0 {
                    match phase {
                        0 => {
                            self.player.state = PlayerState::Attacking { phase: 1, timer: 0.10 };
                            do_attack = true;   // deferred: called AFTER match ends
                        }
                        1 => {
                            self.player.state = PlayerState::Attacking { phase: 2, timer: 0.18 };
                        }
                        _ => {
                            self.player.combo = (self.player.combo + 1) % 3;
                            self.player.state = PlayerState::Idle;
                        }
                    }
                } else {
                    self.player.state = PlayerState::Attacking { phase, timer: t };
                }
                self.player.vel = Vec2::ZERO;
            }

            PlayerState::Dodging { timer } => {
                let t = timer - dt;
                if t <= 0.0 {
                    self.player.state = PlayerState::Idle;
                    self.player.vel   = Vec2::ZERO;
                } else {
                    self.player.state  = PlayerState::Dodging { timer: t };
                    self.player.vel   *= 1.0 - dt * 4.0;
                }
            }

            PlayerState::Dead => {}
        }

        // ── Deferred attack (no active borrow of self.player) ─────────────
        if do_attack { self.deal_player_attack(); }

        // ── Stamina regen ─────────────────────────────────────────────────
        let in_dodge = matches!(self.player.state, PlayerState::Dodging { .. });
        if !in_dodge {
            self.player.stamina = (self.player.stamina + 22.0 * dt).min(self.player.max_sta);
        }

        // ── Move + tile collision ─────────────────────────────────────────
        let new_pos = self.player.pos + self.player.vel * dt;
        let hw = self.player.half_w();
        let hh = self.player.half_h();
        let old = self.player.pos;
        self.player.pos = self.move_entity(new_pos, hw, hh, old);

        let map_px_w = MAP_W as f32 * TILE_SIZE;
        let map_px_h = MAP_H as f32 * TILE_SIZE;
        self.player.pos.x = self.player.pos.x.clamp(hw, map_px_w - hw);
        self.player.pos.y = self.player.pos.y.clamp(hh, map_px_h - hh);
    }

    fn deal_player_attack(&mut self) {
        let atk = self.player.attack_box();
        let dmg = 20.0 + (self.player.combo as f32) * 5.0;
        for e in &mut self.enemies {
            if e.alive() && aabb_overlap(atk, e.aabb()) {
                e.hp -= dmg;
                e.flash = 0.25;
                if e.hp <= 0.0 {
                    e.state = EnemyState::Dead;
                } else {
                    e.state = EnemyState::Stunned { timer: 0.3 };
                }
                self.shake = 0.2;
                self.shake_mag = 3.0;
            }
        }
    }

    fn update_enemies(&mut self, dt: f32) {
        let player_pos   = self.player.pos;
        let player_alive = self.player.alive();
        // Copy the map so we can borrow it independently of &mut self.enemies.
        // [u8; MAP_W*MAP_H] = 220 bytes — trivially cheap.
        let map = self.map;

        for e in &mut self.enemies {
            if !e.alive() { continue; }
            e.flash = (e.flash - dt).max(0.0);

            let to_player = player_pos - e.pos;
            let dist = to_player.length();
            let dir_to_player = if dist > 0.1 { to_player / dist } else { Vec2::X };

            e.state = match e.state {
                // ── Patrol ────────────────────────────────────────────────
                EnemyState::Patrol { dir, timer } => {
                    if player_alive && dist < e.aggro_r {
                        EnemyState::Chase
                    } else {
                        let t = timer - dt;
                        let mut next_dir = dir;
                        if t <= 0.0 { next_dir = -dir; }
                        e.vel_move(dt, Vec2::new(next_dir * 20.0, 0.0), &map);
                        if e.facing != next_dir { e.facing = next_dir; }
                        EnemyState::Patrol { dir: next_dir, timer: if t <= 0.0 { 2.0 } else { t } }
                    }
                }
                // ── Chase ─────────────────────────────────────────────────
                EnemyState::Chase => {
                    if !player_alive || dist > e.aggro_r * 1.5 {
                        EnemyState::Patrol { dir: 1.0, timer: 2.0 }
                    } else if dist < e.atk_r {
                        EnemyState::Windup { timer: 0.5 }
                    } else {
                        e.vel_move(dt, dir_to_player * 28.0, &map);
                        e.facing = if dir_to_player.x >= 0.0 { 1.0 } else { -1.0 };
                        EnemyState::Chase
                    }
                }
                // ── Windup → Strike ───────────────────────────────────────
                EnemyState::Windup { timer } => {
                    let t = timer - dt;
                    if t <= 0.0 {
                        EnemyState::Striking { timer: 0.12 }
                    } else {
                        EnemyState::Windup { timer: t }
                    }
                }
                EnemyState::Striking { timer } => {
                    let t = timer - dt;
                    if t <= 0.0 {
                        EnemyState::Recovery { timer: 0.4 }
                    } else {
                        EnemyState::Striking { timer: t }
                    }
                }
                EnemyState::Recovery { timer } => {
                    let t = timer - dt;
                    if t <= 0.0 { EnemyState::Chase } else { EnemyState::Recovery { timer: t } }
                }
                EnemyState::Stunned { timer } => {
                    let t = timer - dt;
                    if t <= 0.0 { EnemyState::Chase } else { EnemyState::Stunned { timer: t } }
                }
                EnemyState::Dead => EnemyState::Dead,
            };
        }
    }

    fn resolve_enemy_attacks(&mut self) {
        if self.player.i_frames > 0.0 || !self.player.alive() { return; }
        for e in &self.enemies {
            if !matches!(e.state, EnemyState::Striking { .. }) { continue; }
            // Enemy strikes: check if player in melee range
            let dist = (e.pos - self.player.pos).length();
            if dist < e.atk_r + 4.0 {
                self.player.hp -= 15.0;
                self.player.flash = 0.3;
                self.player.i_frames = 0.5;
                self.shake = 0.18; self.shake_mag = 2.5;
                if self.player.hp <= 0.0 {
                    self.player.hp = 0.0;
                    self.player.state = PlayerState::Dead;
                }
                break;
            }
        }
    }

    fn update_camera(&mut self, _dt: f32) {
        let target = self.player.pos;
        let cw    = CANVAS_WIDTH  as f32;
        let ch    = CANVAS_HEIGHT as f32;
        let map_w = MAP_W as f32 * TILE_SIZE;
        let map_h = MAP_H as f32 * TILE_SIZE;

        // Lerp camera toward player
        self.camera = self.camera.lerp(target, 0.12);

        // Clamp — ensure max >= min when map fits inside canvas
        let min_x = cw * 0.5;
        let max_x = (map_w - cw * 0.5).max(min_x);
        let min_y = ch * 0.5;
        let max_y = (map_h - ch * 0.5).max(min_y);
        self.camera.x = self.camera.x.clamp(min_x, max_x);
        self.camera.y = self.camera.y.clamp(min_y, max_y);
    }

    /// Returns the camera center with optional shake offset.
    pub fn camera_with_shake(&self) -> Vec2 {
        if self.shake > 0.0 {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ns = SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().subsec_nanos();
            let angle = (ns % 10000) as f32 / 10000.0 * std::f32::consts::TAU;
            let mag = self.shake_mag * self.shake / 0.2_f32.max(self.shake);
            self.camera + Vec2::new(angle.cos() * mag, angle.sin() * mag)
        } else {
            self.camera
        }
    }

    // ── Tile collision move ───────────────────────────────────────────────────
    fn move_entity(&self, new_pos: Vec2, hw: f32, hh: f32, old_pos: Vec2) -> Vec2 {
        let mut pos = old_pos;

        // Try X move
        let try_x = Vec2::new(new_pos.x, old_pos.y);
        if !self.entity_collides(try_x, hw, hh) {
            pos.x = try_x.x;
        }
        // Try Y move
        let try_y = Vec2::new(pos.x, new_pos.y);
        if !self.entity_collides(try_y, hw, hh) {
            pos.y = try_y.y;
        }
        pos
    }

    fn entity_collides(&self, pos: Vec2, hw: f32, hh: f32) -> bool {
        let left  = ((pos.x - hw) / TILE_SIZE).floor() as i32;
        let right = ((pos.x + hw) / TILE_SIZE).floor() as i32;
        let top   = ((pos.y - hh) / TILE_SIZE).floor() as i32;
        let bot   = ((pos.y + hh) / TILE_SIZE).floor() as i32;
        for ty in top..=bot {
            for tx in left..=right {
                if is_solid(tile_at(&self.map, tx, ty)) { return true; }
            }
        }
        false
    }

    // ── Render ───────────────────────────────────────────────────────────────
    pub fn draw(&self, backend: &mut WgpuBackend) {
        backend.begin_frame();
        self.draw_map(backend);
        self.draw_enemies(backend);
        self.draw_player(backend);
        self.draw_hud(backend);
        backend.end_frame();
    }

    fn q(texture: u32, px: f32, py: f32, sw: f32, sh: f32, col: Color, z: f32) -> SpriteDrawCall {
        SpriteDrawCall {
            texture: texture as u64,
            position: Vec2::new(px, py),
            size: Vec2::new(sw, sh),
            source: None,
            color: col,
            flip: SpriteFlip::None,
            z,
        }
    }

    fn draw_map(&self, backend: &mut WgpuBackend) {
        // Sky / void background
        backend.draw_sprite(Self::q(0, 160.0, 90.0, 320.0, 180.0, Color::rgb8(8, 6, 22), 0.0));

        for row in 0..MAP_H {
            for col in 0..MAP_W {
                let t = self.map[row * MAP_W + col];
                if t == T_VOID { continue; }

                let cx = col as f32 * TILE_SIZE + TILE_SIZE * 0.5;
                let cy = row as f32 * TILE_SIZE + TILE_SIZE * 0.5;

                let base_col = match t {
                    T_GROUND => Color::rgb8(72, 54, 36),
                    T_WALL   => Color::rgb8(42, 36, 52),
                    T_EMBER  => Color::rgb8(90, 44, 18),
                    _        => Color::rgb8(20, 20, 30),
                };

                // Slight bevel: lighter inner quad
                backend.draw_sprite(Self::q(0, cx, cy, TILE_SIZE, TILE_SIZE, base_col, 1.0));
                if t != T_WALL {
                    let inner = Color::rgb8(
                        (base_col.r * 255.0 + 16.0).min(255.0) as u8,
                        (base_col.g * 255.0 + 14.0).min(255.0) as u8,
                        (base_col.b * 255.0 +  8.0).min(255.0) as u8,
                    );
                    backend.draw_sprite(Self::q(0, cx, cy, TILE_SIZE - 2.0, TILE_SIZE - 2.0, inner, 1.1));

                    // Ember glow pulse
                    if t == T_EMBER {
                        let pulse = ((self.time * 3.0).sin() * 0.5 + 0.5) * 0.35;
                        let glow = Color { r: 1.0, g: 0.45, b: 0.1, a: pulse };
                        backend.draw_sprite(Self::q(0, cx, cy, TILE_SIZE - 4.0, TILE_SIZE - 4.0, glow, 1.2));
                    }
                } else {
                    // Wall: darker top-bevel to give depth
                    let shadow = Color::rgb8(28, 22, 36);
                    backend.draw_sprite(Self::q(0, cx, cy - 2.0, TILE_SIZE, 4.0, shadow, 1.2));
                }
            }
        }
    }

    fn draw_player(&self, backend: &mut WgpuBackend) {
        let p = &self.player;
        let (px, py) = (p.pos.x, p.pos.y);

        if !p.alive() {
            // Death — grey crumple
            backend.draw_sprite(Self::q(0, px, py + 3.0, 10.0, 6.0, Color::rgb8(90, 88, 95), 5.0));
            return;
        }

        let flash = p.flash > 0.0;
        let dodge = matches!(p.state, PlayerState::Dodging { .. });
        let atk   = matches!(p.state, PlayerState::Attacking { phase: 1, .. });

        // Body
        let body_col = if flash { Color::rgb8(255, 100, 100) }
                       else if dodge { Color::rgb8(180, 230, 255) }
                       else { Color::WHITE };

        // Legs (animated walk bob)
        let bob = if matches!(p.state, PlayerState::Walking) {
            (self.time * 12.0).sin() * 1.5
        } else { 0.0 };

        // Draw body (slightly taller than hitbox for visual clarity)
        backend.draw_sprite(Self::q(0, px, py + 2.0 + bob * 0.3, 8.0, 10.0, body_col, 4.5));

        // Legs
        let leg_col = Color::rgb8(80, 70, 60);
        backend.draw_sprite(Self::q(0, px - 2.0 + bob, py + 9.0 + bob.abs(), 3.0, 4.0, leg_col, 4.4));
        backend.draw_sprite(Self::q(0, px + 2.0 - bob, py + 9.0 + bob.abs(), 3.0, 4.0, leg_col, 4.4));

        // Head
        let head_col = Color::rgb8(240, 200, 160);
        backend.draw_sprite(Self::q(0, px, py - 5.0, 6.0, 6.0, head_col, 4.6));

        // Eyes
        let eye_x = px + p.facing * 1.5;
        backend.draw_sprite(Self::q(0, eye_x, py - 5.5, 1.5, 1.5, Color::rgb8(30, 30, 40), 4.7));

        // Hair / hood
        backend.draw_sprite(Self::q(0, px, py - 7.5, 7.0, 3.0, Color::rgb8(60, 30, 80), 4.8));

        // Weapon
        let wp_x = px + p.facing * 8.0;
        let wp_y = if atk { py + p.facing * 2.0 } else { py + 2.0 };
        let wp_angle_offset = if atk { -4.0 } else { 0.0 };
        let sword_col = Color::rgb8(200, 210, 230);
        backend.draw_sprite(Self::q(0, wp_x, wp_y + wp_angle_offset, 3.0, 10.0, sword_col, 4.3));
        // Crossguard
        let guard_col = Color::rgb8(160, 120, 60);
        backend.draw_sprite(Self::q(0, wp_x, wp_y + wp_angle_offset + 3.0, 7.0, 2.0, guard_col, 4.35));

        // Attack slash arc  
        if atk {
            let slash_col = Color { r: 1.0, g: 0.95, b: 0.7, a: 0.7 };
            backend.draw_sprite(Self::q(0, px + p.facing * 14.0, py, 12.0, 14.0, slash_col, 4.9));
        }
    }

    fn draw_enemies(&self, backend: &mut WgpuBackend) {
        for e in &self.enemies {
            if !e.alive() {
                // Death marker
                backend.draw_sprite(Self::q(0, e.pos.x, e.pos.y + 2.0, 10.0, 4.0, Color::rgb8(60, 20, 10), 3.0));
                continue;
            }

            let (ex, ey) = (e.pos.x, e.pos.y);
            let windup = matches!(e.state, EnemyState::Windup { .. });
            let strike = matches!(e.state, EnemyState::Striking { .. });

            let base = if e.flash > 0.0 { Color::rgb8(255, 120, 80) }
                       else if windup   { Color::rgb8(200, 80, 40) }
                       else             { Color::rgb8(160, 40, 30) };

            // Body
            backend.draw_sprite(Self::q(0, ex, ey + 1.0, 9.0, 12.0, base, 3.5));
            // Armour plate
            let armour = Color::rgb8(80, 60, 50);
            backend.draw_sprite(Self::q(0, ex, ey - 1.0, 10.0, 7.0, armour, 3.6));
            // Head
            backend.draw_sprite(Self::q(0, ex, ey - 8.0, 7.0, 6.0, Color::rgb8(50, 40, 35), 3.7));
            // Eyes (menacing orange)
            let eye_x = ex + e.facing * 2.0;
            backend.draw_sprite(Self::q(0, eye_x, ey - 8.0, 2.0, 2.0, Color::rgb8(255, 120, 0), 3.8));

            // Weapon (ember blade)
            if strike {
                // Extended strike pos
                let blade = Color::rgb8(220, 80, 20);
                backend.draw_sprite(Self::q(0, ex + e.facing * 10.0, ey - 2.0, 3.0, 12.0, blade, 3.4));
            } else {
                // Idle carry pos
                let blade = Color::rgb8(160, 55, 20);
                backend.draw_sprite(Self::q(0, ex + e.facing * 6.0, ey, 2.0, 10.0, blade, 3.4));
            }

            // HP bar (above enemy)
            let bar_w = 14.0;
            let hp_ratio = e.hp / e.max_hp;
            backend.draw_sprite(Self::q(0, ex, ey - 15.0, bar_w, 2.0, Color::rgb8(60, 20, 20), 4.0));
            backend.draw_sprite(Self::q(0, ex - (bar_w - bar_w * hp_ratio) * 0.5, ey - 15.0, bar_w * hp_ratio, 2.0, Color::rgb8(200, 50, 30), 4.1));
        }
    }

    fn draw_hud(&self, backend: &mut WgpuBackend) {
        let p = &self.player;
        // HUD panel
        backend.draw_sprite(Self::q(0, 52.0, 8.0, 100.0, 18.0, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 }, 9.0));

        // HP bar
        let hp_ratio = (p.hp / p.max_hp).clamp(0.0, 1.0);
        let bar_w = 80.0;
        backend.draw_sprite(Self::q(0, 52.0, 5.0, bar_w, 5.0, Color::rgb8(30, 10, 10), 9.1));
        if hp_ratio > 0.0 {
            backend.draw_sprite(Self::q(0, 52.0 - (bar_w - bar_w * hp_ratio) * 0.5, 5.0, bar_w * hp_ratio, 5.0, Color::rgb8(210, 40, 30), 9.2));
        }
        backend.draw_sprite(Self::q(0, 52.0, 5.0, bar_w, 1.0, Color::rgb8(255, 130, 120), 9.3)); // highlight

        // Stamina bar
        let sta_ratio = (p.stamina / p.max_sta).clamp(0.0, 1.0);
        backend.draw_sprite(Self::q(0, 52.0, 13.0, bar_w, 4.0, Color::rgb8(10, 25, 10), 9.1));
        if sta_ratio > 0.0 {
            let sta_col = if p.stamina < 30.0 { Color::rgb8(200, 170, 20) } else { Color::rgb8(50, 200, 70) };
            backend.draw_sprite(Self::q(0, 52.0 - (bar_w - bar_w * sta_ratio) * 0.5, 13.0, bar_w * sta_ratio, 4.0, sta_col, 9.2));
        }

        // "HP" / "ST" labels (pixel-art colour coded dots)
        backend.draw_sprite(Self::q(0, 8.0, 5.0, 4.0, 5.0, Color::rgb8(210, 40, 30), 9.5));  // HP dot
        backend.draw_sprite(Self::q(0, 8.0, 13.0, 4.0, 4.0, Color::rgb8(50, 200, 70), 9.5)); // STA dot

        // Controls hint (bottom right corner)
        let hint_col = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.25 };
        backend.draw_sprite(Self::q(0, 280.0, 173.0, 80.0, 10.0, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 }, 9.0));
        backend.draw_sprite(Self::q(0, 256.0, 173.0, 4.0, 4.0, hint_col, 9.1));  // W key
        backend.draw_sprite(Self::q(0, 262.0, 173.0, 4.0, 4.0, hint_col, 9.1));  // A key
        backend.draw_sprite(Self::q(0, 268.0, 173.0, 4.0, 4.0, hint_col, 9.1));  // S key
        backend.draw_sprite(Self::q(0, 274.0, 173.0, 4.0, 4.0, hint_col, 9.1));  // D key
        backend.draw_sprite(Self::q(0, 284.0, 173.0, 5.0, 4.0, Color::rgb8(200, 160, 50), 9.1));  // Z atk
        backend.draw_sprite(Self::q(0, 294.0, 173.0, 5.0, 4.0, Color::rgb8(50, 150, 220), 9.1));  // X dodge

        // Dead screen
        if matches!(p.state, PlayerState::Dead) {
            backend.draw_sprite(Self::q(0, 160.0, 90.0, 320.0, 180.0, Color { r: 0.3, g: 0.0, b: 0.0, a: 0.55 }, 10.0));
            // "YOU DIED" — big text represented as thick bars
            let y = 80.0;
            let bars = [
                (120.0,y,4.0,20.0),(128.0,y+8.0,14.0,4.0),(128.0,y,4.0,10.0),
                (140.0,y,4.0,20.0),(148.0,y,4.0,20.0),(148.0,y,14.0,4.0),
                (148.0,y+8.0,10.0,4.0),(148.0,y+16.0,14.0,4.0),
            ];
            for (bx,by,bw,bh) in bars {
                backend.draw_sprite(Self::q(0, bx, by, bw, bh, Color::rgb8(200, 30, 30), 10.5));
            }
        }
    }
}

// ── Enemy velocity helper ─────────────────────────────────────────────────────
impl Enemy {
    fn vel_move(&mut self, dt: f32, vel: Vec2, map: &[u8; MAP_W * MAP_H]) {
        let hw = self.half_w();
        let hh = self.half_h();
        // X
        let nx = self.pos.x + vel.x * dt;
        let tx = (nx / TILE_SIZE).floor() as i32;
        let ty_top = ((self.pos.y - hh) / TILE_SIZE).floor() as i32;
        let ty_bot = ((self.pos.y + hh) / TILE_SIZE).floor() as i32;
        let solid_x = (ty_top..=ty_bot).any(|ty| is_solid(tile_at(map, tx, ty)));
        if !solid_x { self.pos.x = nx; }
        // Y
        let ny = self.pos.y + vel.y * dt;
        let ty = (ny / TILE_SIZE).floor() as i32;
        let tx_left  = ((self.pos.x - hw) / TILE_SIZE).floor() as i32;
        let tx_right = ((self.pos.x + hw) / TILE_SIZE).floor() as i32;
        let solid_y = (tx_left..=tx_right).any(|tx| is_solid(tile_at(map, tx, ty)));
        if !solid_y { self.pos.y = ny; }
    }
}

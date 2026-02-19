//! The Ashlands — Tiny Sprites in a Vast World
//!
//! A 640×480 px world (80×60 tiles at 8px each) with 3×4 px sprites.
//! Player is half a tile tall. Camera scrolls to reveal ~½ the world.
//! Action telegraphs via effects (flashes, trails) not sprite detail.

use glam::Vec2;
use gur_render::{
    Color, WgpuBackend, CANVAS_WIDTH, CANVAS_HEIGHT,
    sprite::{SpriteDrawCall, SpriteFlip},
};

// ─── Scale Constants ─────────────────────────────────────────────────────────
pub const TILE_SIZE: f32 = 8.0;
pub const MAP_W:     usize = 80;
pub const MAP_H:     usize = 60;

pub const T_VOID:   u8 = 0;
pub const T_GROUND: u8 = 1;
pub const T_WALL:   u8 = 2;
pub const T_EMBER:  u8 = 3;
pub const T_ASH:    u8 = 4;  // void/pit tiles

// ─── Map Generation ──────────────────────────────────────────────────────────

fn build_map() -> [u8; MAP_W * MAP_H] {
    let mut map = [T_GROUND; MAP_W * MAP_H];

    // Wall border
    for x in 0..MAP_W {
        map[x] = T_WALL;                    // top row
        map[(MAP_H - 1) * MAP_W + x] = T_WALL; // bottom row
    }
    for y in 0..MAP_H {
        map[y * MAP_W] = T_WALL;            // left column
        map[y * MAP_W + (MAP_W - 1)] = T_WALL; // right column
    }

    // Zone 1: Starting clearing (top-left) — already ground, add some decoration
    // Small pillar cluster near start
    for dy in 0..3 {
        for dx in 0..2 {
            map[(12 + dy) * MAP_W + (8 + dx)] = T_WALL;
        }
    }

    // Zone 2: Ruined columns (mid-left, rows 20-40, cols 10-25)
    for col in (12..24).step_by(4) {
        for dy in 0..3 {
            for dx in 0..1 {
                map[(25 + dy) * MAP_W + col + dx] = T_WALL;
            }
        }
    }

    // Zone 3: Ember lake (centre, 8×5 ember zone)
    for y in 28..33 {
        for x in 35..43 {
            map[y * MAP_W + x] = T_EMBER;
        }
    }

    // Zone 4: Wall maze (mid-right, rows 15-45, cols 55-70)
    // Horizontal walls with gaps
    for x in 56..68 {
        map[20 * MAP_W + x] = T_WALL;
        map[30 * MAP_W + x] = T_WALL;
        map[40 * MAP_W + x] = T_WALL;
    }
    // Vertical dividers
    for y in 20..41 {
        map[y * MAP_W + 62] = T_WALL;
    }
    // Gaps in walls (passages)
    map[20 * MAP_W + 60] = T_GROUND;
    map[30 * MAP_W + 64] = T_GROUND;
    map[40 * MAP_W + 58] = T_GROUND;

    // Zone 5: Boss arena (far right, large clearing)
    // Just ground — already set. Add boundary pillars
    for y in 10..50 {
        map[y * MAP_W + 75] = T_WALL;
    }
    // Entrance gap
    for y in 28..32 {
        map[y * MAP_W + 75] = T_GROUND;
    }

    // Zone 6: Ash pit (bottom-right, void tiles)
    for y in 50..58 {
        for x in 60..75 {
            map[y * MAP_W + x] = T_ASH;
        }
    }

    map
}

fn tile_at(map: &[u8; MAP_W * MAP_H], tx: i32, ty: i32) -> u8 {
    if tx < 0 || ty < 0 || tx >= MAP_W as i32 || ty >= MAP_H as i32 {
        return T_WALL;
    }
    map[(ty as usize) * MAP_W + (tx as usize)]
}

fn is_solid(t: u8) -> bool { matches!(t, T_WALL | T_VOID | T_ASH) }

// ─── Input snapshot ──────────────────────────────────────────────────────────
#[derive(Default, Clone)]
pub struct Input {
    pub up:     bool,
    pub down:   bool,
    pub left:   bool,
    pub right:  bool,
    pub attack: bool,
    pub dodge:  bool,
}

// ─── Dodge Trail ─────────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
struct TrailGhost {
    pos: Vec2,
    age: f32, // 0.0 → 1.0 (fades out)
}

impl Default for TrailGhost {
    fn default() -> Self {
        Self { pos: Vec2::ZERO, age: 1.0 }
    }
}

// ─── Player ──────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
pub enum PlayerState {
    Idle,
    Walking,
    Attacking { phase: u8, timer: f32 },
    Dodging   { timer: f32 },
    Dead,
}

pub struct Player {
    pub pos:      Vec2,
    pub vel:      Vec2,
    pub facing:   f32,
    pub state:    PlayerState,
    pub hp:       f32,
    pub max_hp:   f32,
    pub stamina:  f32,
    pub max_sta:  f32,
    pub i_frames: f32,
    pub combo:    u8,
    pub flash:    f32,
    pub walk_bob: f32, // animation timer
}

impl Player {
    fn new(x: f32, y: f32) -> Self {
        Self {
            pos: Vec2::new(x, y), vel: Vec2::ZERO, facing: 1.0,
            state: PlayerState::Idle,
            hp: 100.0, max_hp: 100.0,
            stamina: 100.0, max_sta: 100.0,
            i_frames: 0.0, combo: 0, flash: 0.0,
            walk_bob: 0.0,
        }
    }

    pub fn alive(&self) -> bool { !matches!(self.state, PlayerState::Dead) }

    fn half_w(&self) -> f32 { 1.5 }  // 3 px width hitbox
    fn half_h(&self) -> f32 { 2.0 }   // 4 px height hitbox

    fn aabb(&self) -> (Vec2, Vec2) {
        (self.pos - Vec2::new(self.half_w(), self.half_h()),
         self.pos + Vec2::new(self.half_w(), self.half_h()))
    }

    fn attack_box(&self) -> (Vec2, Vec2) {
        let offset = self.facing * 3.0;
        let cx = self.pos.x + offset;
        let cy = self.pos.y;
        (Vec2::new(cx - 3.0, cy - 3.0), Vec2::new(cx + 3.0, cy + 3.0))
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
    pub flash:    f32,
    home:         Vec2,
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
            aggro_r: 48.0, // scaled for tiny world
            atk_r: 10.0,
        }
    }

    pub fn alive(&self) -> bool { !matches!(self.state, EnemyState::Dead) }

    fn half_w(&self) -> f32 { 1.25 }  // 2.5 px width
    fn half_h(&self) -> f32 { 1.75 }   // 3.5 px height

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
    trail:        [TrailGhost; 3],
}

impl Scene {
    pub fn new() -> Self {
        Self {
            map: build_map(),
            player:  Player::new(24.0, 24.0), // Starting clearing
            enemies: vec![
                // Near ruined columns
                Enemy::new(120.0, 120.0),
                Enemy::new(150.0, 180.0),
                // Near ember lake
                Enemy::new(300.0, 140.0),
                Enemy::new(340.0, 160.0),
                // In wall maze
                Enemy::new(470.0, 200.0),
                Enemy::new(490.0, 280.0),
                // Near boss arena
                Enemy::new(580.0, 200.0),
                Enemy::new(600.0, 280.0),
                // Scattered
                Enemy::new(200.0, 350.0),
                Enemy::new(400.0, 400.0),
            ],
            camera:    Vec2::new(320.0, 240.0),
            shake:     0.0,
            shake_mag: 0.0,
            time:      0.0,
            trail:     [TrailGhost::default(); 3],
        }
    }

    // ── Update ────────────────────────────────────────────────────────────
    pub fn update(&mut self, dt: f32, input: &Input) {
        self.time += dt;
        self.shake = (self.shake - dt).max(0.0);

        // Update trail ghosts
        for ghost in &mut self.trail {
            ghost.age = (ghost.age + dt * 5.0).min(1.0);
        }

        if self.player.alive() {
            self.update_player(dt, input);
        }
        self.update_enemies(dt);
        self.resolve_enemy_attacks();
        self.update_camera(dt);
    }

    fn update_player(&mut self, dt: f32, input: &Input) {
        self.player.i_frames = (self.player.i_frames - dt).max(0.0);
        self.player.flash    = (self.player.flash    - dt).max(0.0);

        let mut do_attack = false;
        let mut is_dodging = false;

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
                    
                    // Initialize trail
                    self.trail[0] = TrailGhost { pos: self.player.pos, age: 0.0 };
                    self.trail[1] = TrailGhost { pos: self.player.pos, age: 0.33 };
                    self.trail[2] = TrailGhost { pos: self.player.pos, age: 0.66 };
                }
            }

            PlayerState::Attacking { phase, timer } => {
                let t = timer - dt;
                if t <= 0.0 {
                    match phase {
                        0 => {
                            self.player.state = PlayerState::Attacking { phase: 1, timer: 0.10 };
                            do_attack = true;
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
                is_dodging = true;
                let t = timer - dt;
                if t <= 0.0 {
                    self.player.state = PlayerState::Idle;
                    self.player.vel   = Vec2::ZERO;
                } else {
                    self.player.state  = PlayerState::Dodging { timer: t };
                    self.player.vel   *= 1.0 - dt * 4.0;
                    
                    // Update trail every ~0.1s
                    if (t * 10.0) as u32 != ((t + dt) * 10.0) as u32 {
                        // Shift trail
                        self.trail[2] = self.trail[1];
                        self.trail[1] = self.trail[0];
                        self.trail[0] = TrailGhost { pos: self.player.pos, age: 0.0 };
                    }
                }
            }

            PlayerState::Dead => {}
        }

        if do_attack { self.deal_player_attack(); }

        // Stamina regen
        if !is_dodging {
            self.player.stamina = (self.player.stamina + 22.0 * dt).min(self.player.max_sta);
        }

        // Walk bob animation
        if matches!(self.player.state, PlayerState::Walking) {
            self.player.walk_bob += dt * 10.0;
        } else {
            self.player.walk_bob = 0.0;
        }

        // Move + tile collision
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
        let map = self.map;

        for e in &mut self.enemies {
            if !e.alive() { continue; }
            e.flash = (e.flash - dt).max(0.0);

            let to_player = player_pos - e.pos;
            let dist = to_player.length();
            let dir_to_player = if dist > 0.1 { to_player / dist } else { Vec2::X };

            e.state = match e.state {
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
            let dist = (e.pos - self.player.pos).length();
            if dist < e.atk_r + 3.0 {
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

        self.camera = self.camera.lerp(target, 0.12);

        let min_x = cw * 0.5;
        let max_x = (map_w - cw * 0.5).max(min_x);
        let min_y = ch * 0.5;
        let max_y = (map_h - ch * 0.5).max(min_y);
        self.camera.x = self.camera.x.clamp(min_x, max_x);
        self.camera.y = self.camera.y.clamp(min_y, max_y);
    }

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

    // ── Tile collision ───────────────────────────────────────────────────────
    fn move_entity(&self, new_pos: Vec2, hw: f32, hh: f32, old_pos: Vec2) -> Vec2 {
        let mut pos = old_pos;

        let try_x = Vec2::new(new_pos.x, old_pos.y);
        if !self.entity_collides(try_x, hw, hh) {
            pos.x = try_x.x;
        }
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
        self.draw_trail(backend);
        self.draw_enemies(backend);
        self.draw_player(backend);
        self.draw_hud(backend);
        backend.end_frame();
    }

    fn q(texture: u64, px: f32, py: f32, sw: f32, sh: f32, col: Color, z: f32) -> SpriteDrawCall {
        SpriteDrawCall {
            texture,
            position: Vec2::new(px, py),
            size: Vec2::new(sw, sh),
            source: None,
            color: col,
            flip: SpriteFlip::None,
            z,
        }
    }

    // Screen-space helpers (camera-relative)
    fn sl(&self, sx: f32) -> f32 { self.camera.x - (CANVAS_WIDTH as f32 / 2.0) + sx }
    fn st(&self, sy: f32) -> f32 { self.camera.y - (CANVAS_HEIGHT as f32 / 2.0) + sy }

    fn draw_map(&self, backend: &mut WgpuBackend) {
        // Calculate visible tile range for culling
        let cam = self.camera;
        let half_cw = CANVAS_WIDTH as f32 / 2.0 + TILE_SIZE;
        let half_ch = CANVAS_HEIGHT as f32 / 2.0 + TILE_SIZE;
        
        let min_tx = ((cam.x - half_cw) / TILE_SIZE).floor() as i32;
        let max_tx = ((cam.x + half_cw) / TILE_SIZE).ceil() as i32;
        let min_ty = ((cam.y - half_ch) / TILE_SIZE).floor() as i32;
        let max_ty = ((cam.y + half_ch) / TILE_SIZE).ceil() as i32;

        // Sky background (screen-space)
        backend.draw_sprite(Self::q(0, cam.x, cam.y, CANVAS_WIDTH as f32, CANVAS_HEIGHT as f32, 
            Color::rgb8(8, 6, 22), 0.0));

        for row in min_ty..=max_ty {
            for col in min_tx..=max_tx {
                if col < 0 || row < 0 || col >= MAP_W as i32 || row >= MAP_H as i32 {
                    continue;
                }
                let t = self.map[(row as usize) * MAP_W + (col as usize)];
                if t == T_VOID { continue; }

                let cx = col as f32 * TILE_SIZE + TILE_SIZE * 0.5;
                let cy = row as f32 * TILE_SIZE + TILE_SIZE * 0.5;

                let base_col = match t {
                    T_GROUND => Color::rgb8(72, 54, 36),
                    T_WALL   => Color::rgb8(42, 36, 52),
                    T_EMBER  => Color::rgb8(90, 44, 18),
                    T_ASH    => Color::rgb8(15, 10, 18),
                    _        => Color::rgb8(20, 20, 30),
                };

                backend.draw_sprite(Self::q(0, cx, cy, TILE_SIZE, TILE_SIZE, base_col, 1.0));
                
                if t == T_GROUND {
                    let inner = Color::rgb8(
                        (base_col.r * 255.0 + 10.0).min(255.0) as u8,
                        (base_col.g * 255.0 + 8.0).min(255.0) as u8,
                        (base_col.b * 255.0 + 5.0).min(255.0) as u8,
                    );
                    backend.draw_sprite(Self::q(0, cx, cy, TILE_SIZE - 1.0, TILE_SIZE - 1.0, inner, 1.1));
                }

                if t == T_EMBER {
                    let pulse = ((self.time * 3.0).sin() * 0.5 + 0.5) * 0.4;
                    let glow = Color { r: 1.0, g: 0.45, b: 0.1, a: pulse };
                    backend.draw_sprite(Self::q(0, cx, cy, TILE_SIZE - 2.0, TILE_SIZE - 2.0, glow, 1.2));
                }

                if t == T_WALL {
                    let shadow = Color::rgb8(28, 22, 36);
                    backend.draw_sprite(Self::q(0, cx, cy - 1.0, TILE_SIZE, 2.0, shadow, 1.2));
                }

                if t == T_ASH {
                    let dark = Color { r: 0.05, g: 0.02, b: 0.05, a: 0.8 };
                    backend.draw_sprite(Self::q(0, cx, cy, TILE_SIZE - 1.0, TILE_SIZE - 1.0, dark, 1.1));
                }
            }
        }
    }

    fn draw_trail(&self, backend: &mut WgpuBackend) {
        for ghost in &self.trail {
            if ghost.age < 1.0 && matches!(self.player.state, PlayerState::Dodging { .. }) {
                let alpha = 1.0 - ghost.age;
                let col = Color { r: 0.3, g: 0.8, b: 1.0, a: alpha * 0.7 };
                backend.draw_sprite(Self::q(0, ghost.pos.x, ghost.pos.y, 3.0, 3.0, col, 3.8));
            }
        }
    }

    fn draw_player(&self, backend: &mut WgpuBackend) {
        let p = &self.player;
        let (px, py) = (p.pos.x, p.pos.y);

        if !p.alive() {
            // Death — collapsed 4×2 dark shape
            backend.draw_sprite(Self::q(0, px, py + 2.0, 4.0, 2.0, Color::rgb8(60, 55, 65), 5.0));
            // Sprawled limbs
            backend.draw_sprite(Self::q(0, px - 2.0, py + 3.0, 2.0, 1.0, Color::rgb8(50, 45, 55), 4.9));
            backend.draw_sprite(Self::q(0, px + 2.0, py + 3.0, 2.0, 1.0, Color::rgb8(50, 45, 55), 4.9));
            return;
        }

        let flash = p.flash > 0.0;
        let dodge = matches!(p.state, PlayerState::Dodging { .. });
        let atk_phase0 = matches!(p.state, PlayerState::Attacking { phase: 0, .. });
        let atk_phase1 = matches!(p.state, PlayerState::Attacking { phase: 1, .. });
        let atk_phase2 = matches!(p.state, PlayerState::Attacking { phase: 2, .. });
        let i_frame_flicker = p.i_frames > 0.0 && (self.time * 20.0).floor() as u32 % 2 == 0;

        if i_frame_flicker && !flash && !dodge {
            return;
        }

        // Colors based on state
        let skin_col = if flash { Color::rgb8(255, 150, 150) } else { Color::rgb8(240, 200, 170) };
        let armor_col = if flash { 
            Color::rgb8(255, 100, 100) 
        } else if dodge { 
            Color::rgb8(120, 200, 255) 
        } else { 
            Color::rgb8(80, 75, 90) // Dark armor
        };
        let cloak_col = if dodge { Color::rgb8(100, 180, 220) } else { Color::rgb8(45, 35, 55) };
        let sword_col = Color::rgb8(200, 210, 220);
        let hilt_col = Color::rgb8(140, 100, 60);
        let boot_col = Color::rgb8(50, 40, 35);

        // Walk cycle
        let walk_frame = (p.walk_bob * 2.0) as i32 % 2;
        let is_walking = matches!(p.state, PlayerState::Walking);
        
        // Leg positions for walk animation
        let (left_leg_offset, right_leg_offset) = if is_walking {
            if walk_frame == 0 { (1.5, -1.5) } else { (-1.5, 1.5) }
        } else {
            (0.0, 0.0)
        };

        // Sword angle based on state
        let (sword_x, sword_y, _sword_angle, sword_len) = if atk_phase1 {
            // Extended slash
            (px + p.facing * 10.0, py, p.facing * 8.0, 12.0)
        } else if atk_phase0 {
            // Windup - sword pulled back
            (px + p.facing * -3.0, py - 2.0, -p.facing * 4.0, 6.0)
        } else if atk_phase2 {
            // Recovery
            (px + p.facing * 6.0, py - 1.0, p.facing * 3.0, 8.0)
        } else {
            // Idle/carrying - sword at side
            (px + p.facing * 4.0, py + 1.0, 0.0, 6.0)
        };

        // ─── Draw order (back to front) ─────────────────────────────────
        
        // Cloak/cape (behind body)
        let cape_col = if dodge { Color::rgb8(80, 160, 200) } else { Color::rgb8(35, 25, 45) };
        backend.draw_sprite(Self::q(0, px - p.facing * 1.0, py + 1.0, 3.0, 4.0, cape_col, 4.0));

        // Back leg (behind)
        let back_leg_x = px - p.facing * 1.5 + if is_walking { -p.facing * left_leg_offset * 0.5 } else { 0.0 };
        let back_leg_y = py + 4.0 + if is_walking { left_leg_offset * 0.3 } else { 0.0 };
        backend.draw_sprite(Self::q(0, back_leg_x, back_leg_y, 2.0, 3.0, boot_col, 4.1));
        backend.draw_sprite(Self::q(0, back_leg_x, back_leg_y - 1.5, 1.5, 2.0, armor_col, 4.2));

        // Back arm (behind body)
        let back_arm_x = px - p.facing * 2.0;
        let back_arm_y = py - 0.5;
        backend.draw_sprite(Self::q(0, back_arm_x, back_arm_y, 1.5, 3.0, armor_col, 4.3));

        // Sword (drawn before body during idle, after during attack)
        if !atk_phase1 {
            // Sword blade (carrying position)
            backend.draw_sprite(Self::q(0, sword_x, sword_y, 1.5, sword_len, sword_col, 4.35));
            // Sword hilt
            backend.draw_sprite(Self::q(0, px + p.facing * 3.5, py + 2.0, 3.0, 1.5, hilt_col, 4.36));
        }

        // Main body/torso (center)
        backend.draw_sprite(Self::q(0, px, py, 4.0, 3.0, armor_col, 4.5));
        // Chest plate highlight
        let chest_high = if flash { Color::rgb8(255, 180, 180) } else { Color::rgb8(100, 95, 110) };
        backend.draw_sprite(Self::q(0, px, py - 0.5, 2.5, 1.5, chest_high, 4.55));

        // Front arm (holding weapon)
        let front_arm_x = px + p.facing * 2.0;
        let front_arm_y = py;
        backend.draw_sprite(Self::q(0, front_arm_x, front_arm_y, 1.5, 3.0, armor_col, 4.6));
        // Glove/hand
        backend.draw_sprite(Self::q(0, front_arm_x + p.facing * 1.0, front_arm_y + 1.5, 1.5, 1.5, Color::rgb8(60, 50, 45), 4.65));

        // Front leg (in front)
        let front_leg_x = px + p.facing * 1.5 + if is_walking { p.facing * right_leg_offset * 0.5 } else { 0.0 };
        let front_leg_y = py + 4.0 + if is_walking { right_leg_offset * 0.3 } else { 0.0 };
        backend.draw_sprite(Self::q(0, front_leg_x, front_leg_y, 2.0, 3.0, boot_col, 4.7));
        backend.draw_sprite(Self::q(0, front_leg_x, front_leg_y - 1.5, 1.5, 2.0, armor_col, 4.75));

        // Head (on top)
        backend.draw_sprite(Self::q(0, px, py - 3.0, 3.5, 3.0, skin_col, 4.8));
        
        // Helmet/hair
        backend.draw_sprite(Self::q(0, px, py - 4.5, 4.0, 2.0, cloak_col, 4.85));
        // Helmet visor slit
        let visor_col = if flash { Color::rgb8(255, 200, 150) } else { Color::rgb8(20, 20, 25) };
        backend.draw_sprite(Self::q(0, px + p.facing * 0.5, py - 3.0, 2.0, 0.8, visor_col, 4.9));
        
        // Eye glow (visible through visor)
        let eye_col = if dodge { Color::rgb8(150, 220, 255) } else { Color::rgb8(180, 160, 140) };
        backend.draw_sprite(Self::q(0, px + p.facing * 0.8, py - 3.0, 0.8, 0.5, eye_col, 4.95));

        // Attack slash effect
        if atk_phase1 {
            // Big slash arc
            let slash_x = px + p.facing * 9.0;
            let slash_y = py;
            
            // Outer glow
            let outer = Color { r: 1.0, g: 0.9, b: 0.4, a: 0.7 };
            backend.draw_sprite(Self::q(0, slash_x, slash_y, 14.0, 10.0, outer, 5.0));
            
            // Inner slash core
            let inner = Color { r: 1.0, g: 1.0, b: 0.85, a: 0.95 };
            backend.draw_sprite(Self::q(0, slash_x, slash_y, 10.0, 6.0, inner, 5.1));
            
            // Sword extended through slash
            backend.draw_sprite(Self::q(0, slash_x, slash_y, 2.0, 12.0, sword_col, 5.05));
            
            // Sparks
            for i in 0..3 {
                let spark_x = slash_x + p.facing * (3.0 + i as f32 * 2.0);
                let spark_y = slash_y + (i as f32 - 1.0) * 3.0;
                backend.draw_sprite(Self::q(0, spark_x, spark_y, 1.5, 1.5, Color::rgb8(255, 255, 200), 5.2));
            }
        }
    }

    fn draw_enemies(&self, backend: &mut WgpuBackend) {
        for e in &self.enemies {
            let (ex, ey) = (e.pos.x, e.pos.y);
            
            if !e.alive() {
                // Death — collapsed heap
                backend.draw_sprite(Self::q(0, ex, ey + 2.0, 4.0, 2.0, Color::rgb8(50, 20, 15), 3.0));
                backend.draw_sprite(Self::q(0, ex - 1.5, ey + 3.0, 2.0, 1.5, Color::rgb8(40, 15, 10), 2.9));
                backend.draw_sprite(Self::q(0, ex + 1.5, ey + 3.0, 2.0, 1.5, Color::rgb8(40, 15, 10), 2.9));
                continue;
            }

            let windup = matches!(e.state, EnemyState::Windup { .. });
            let strike = matches!(e.state, EnemyState::Striking { .. });
            let stunned = matches!(e.state, EnemyState::Stunned { .. });
            let chasing = matches!(e.state, EnemyState::Chase);

            // Colors based on state (Ember Knight theme)
            let skin_col = if e.flash > 0.0 { Color::rgb8(255, 140, 100) } else { Color::rgb8(100, 70, 60) };
            let armor_col = if e.flash > 0.0 {
                Color::rgb8(255, 110, 70)
            } else if stunned {
                Color::rgb8(90, 90, 95)
            } else if windup {
                Color::rgb8(255, 90, 20) // Glowing hot orange
            } else if chasing {
                Color::rgb8(180, 50, 35) // Brighter when chasing
            } else {
                Color::rgb8(140, 40, 30) // Dark maroon idle
            };
            let _ember_col = if windup { Color::rgb8(255, 255, 80) } 
                           else if strike { Color::rgb8(255, 200, 40) }
                           else { Color::rgb8(255, 120, 30) };
            let weapon_col = Color::rgb8(180, 80, 30); // Ember blade

            // ─── Draw order (back to front) ─────────────────────────────────
            
            // Back leg
            let back_leg_x = ex - e.facing * 1.0;
            backend.draw_sprite(Self::q(0, back_leg_x, ey + 3.5, 1.5, 3.0, armor_col, 3.1));
            backend.draw_sprite(Self::q(0, back_leg_x, ey + 2.5, 1.0, 1.5, skin_col, 3.15));

            // Back arm
            let back_arm_x = ex - e.facing * 2.0;
            backend.draw_sprite(Self::q(0, back_arm_x, ey - 0.5, 1.5, 2.5, armor_col, 3.2));

            // Weapon (ember blade - held in back hand during idle)
            if !strike {
                let wpn_x = ex - e.facing * 3.0;
                let wpn_y = ey;
                backend.draw_sprite(Self::q(0, wpn_x, wpn_y, 1.0, 8.0, weapon_col, 3.25));
                // Ember glow on blade
                let glow = Color { r: 1.0, g: 0.5, b: 0.1, a: 0.5 };
                backend.draw_sprite(Self::q(0, wpn_x, wpn_y, 1.5, 6.0, glow, 3.26));
            }

            // Main body/torso
            backend.draw_sprite(Self::q(0, ex, ey, 3.0, 3.0, armor_col, 3.5));
            // Chest armor detail
            let chest_detail = if windup { Color::rgb8(255, 140, 50) } else { Color::rgb8(100, 30, 25) };
            backend.draw_sprite(Self::q(0, ex, ey - 0.5, 2.0, 1.5, chest_detail, 3.55));
            
            // Shoulder plates
            backend.draw_sprite(Self::q(0, ex - 1.5, ey - 1.0, 1.5, 1.5, armor_col, 3.56));
            backend.draw_sprite(Self::q(0, ex + 1.5, ey - 1.0, 1.5, 1.5, armor_col, 3.56));

            // Front arm (weapon arm)
            let front_arm_x = ex + e.facing * 1.5;
            let front_arm_y = ey;
            if windup || strike {
                // Arm extended for attack
                backend.draw_sprite(Self::q(0, front_arm_x + e.facing * 1.5, front_arm_y, 1.5, 2.0, armor_col, 3.6));
            } else {
                backend.draw_sprite(Self::q(0, front_arm_x, front_arm_y, 1.5, 2.5, armor_col, 3.6));
            }

            // Front leg
            let front_leg_x = ex + e.facing * 1.0;
            backend.draw_sprite(Self::q(0, front_leg_x, ey + 3.5, 1.5, 3.0, armor_col, 3.7));
            backend.draw_sprite(Self::q(0, front_leg_x, ey + 2.5, 1.0, 1.5, skin_col, 3.75));

            // Head
            backend.draw_sprite(Self::q(0, ex, ey - 2.5, 3.0, 2.5, skin_col, 3.8));
            
            // Helmet/horned crown
            let helm_col = if windup { Color::rgb8(200, 80, 30) } else { Color::rgb8(60, 35, 30) };
            backend.draw_sprite(Self::q(0, ex, ey - 4.0, 4.0, 2.0, helm_col, 3.85));
            // Horns
            backend.draw_sprite(Self::q(0, ex - 1.5, ey - 5.0, 0.8, 2.0, Color::rgb8(40, 30, 25), 3.9));
            backend.draw_sprite(Self::q(0, ex + 1.5, ey - 5.0, 0.8, 2.0, Color::rgb8(40, 30, 25), 3.9));

            // Glowing ember eyes
            let eye_glow = if windup || strike { 
                Color { r: 1.0, g: 1.0, b: 0.5, a: 1.0 }
            } else if chasing {
                Color { r: 1.0, g: 0.6, b: 0.2, a: 1.0 }
            } else {
                Color { r: 1.0, g: 0.4, b: 0.1, a: 0.9 }
            };
            let eye_x = ex + e.facing * 0.5;
            backend.draw_sprite(Self::q(0, eye_x, ey - 2.5, 1.2, 0.8, eye_glow, 3.95));

            // Attack effect - ember slash
            if strike {
                let slash_x = ex + e.facing * 8.0;
                let slash_y = ey;
                
                // Big fire slash
                let slash_outer = Color { r: 1.0, g: 0.5, b: 0.1, a: 0.85 };
                backend.draw_sprite(Self::q(0, slash_x, slash_y, 12.0, 8.0, slash_outer, 3.3));
                
                let slash_inner = Color { r: 1.0, g: 0.9, b: 0.3, a: 0.95 };
                backend.draw_sprite(Self::q(0, slash_x, slash_y, 8.0, 4.0, slash_inner, 3.35));
                
                // Extended weapon through slash
                backend.draw_sprite(Self::q(0, slash_x, slash_y, 1.5, 10.0, weapon_col, 3.32));
                
                // Ember particles
                for i in 0..4 {
                    let px = slash_x + e.facing * (2.0 + i as f32 * 2.5);
                    let py = slash_y + (i as f32 - 1.5) * 2.0;
                    backend.draw_sprite(Self::q(0, px, py, 1.5, 1.5, Color::rgb8(255, 200, 50), 3.4));
                }
            }

            // Windup glow effect
            if windup {
                let glow = Color { r: 1.0, g: 0.5, b: 0.15, a: 0.3 + (self.time * 5.0).sin() * 0.15 };
                backend.draw_sprite(Self::q(0, ex, ey, 8.0, 10.0, glow, 3.0));
            }

            // HP bar
            let bar_w = 8.0;
            let hp_ratio = e.hp / e.max_hp;
            let bar_y = ey - 6.5;
            backend.draw_sprite(Self::q(0, ex, bar_y, bar_w, 1.2, Color::rgb8(30, 10, 10), 4.0));
            if hp_ratio > 0.0 {
                let hp_col = if hp_ratio < 0.3 { Color::rgb8(255, 80, 50) } else { Color::rgb8(200, 50, 35) };
                backend.draw_sprite(Self::q(0, ex - (bar_w - bar_w * hp_ratio) * 0.5, bar_y, 
                    bar_w * hp_ratio, 1.2, hp_col, 4.1));
            }
        }
    }

    fn draw_hud(&self, backend: &mut WgpuBackend) {
        // HUD is screen-space (camera-relative)
        let hud_left = self.sl(48.0);
        let hud_top = self.st(8.0);

        // HUD panel
        backend.draw_sprite(Self::q(0, hud_left, hud_top, 90.0, 14.0, 
            Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 }, 9.0));

        // HP bar
        let hp_ratio = (self.player.hp / self.player.max_hp).clamp(0.0, 1.0);
        let bar_w = 70.0;
        let hp_y = self.st(5.0);
        backend.draw_sprite(Self::q(0, hud_left, hp_y, bar_w, 4.0, Color::rgb8(30, 10, 10), 9.1));
        if hp_ratio > 0.0 {
            backend.draw_sprite(Self::q(0, hud_left - (bar_w - bar_w * hp_ratio) * 0.5, hp_y, 
                bar_w * hp_ratio, 4.0, Color::rgb8(210, 40, 30), 9.2));
        }
        backend.draw_sprite(Self::q(0, hud_left, hp_y, bar_w, 1.0, Color::rgb8(255, 130, 120), 9.3));

        // Stamina bar
        let sta_ratio = (self.player.stamina / self.player.max_sta).clamp(0.0, 1.0);
        let sta_y = self.st(12.0);
        backend.draw_sprite(Self::q(0, hud_left, sta_y, bar_w, 3.0, Color::rgb8(10, 25, 10), 9.1));
        if sta_ratio > 0.0 {
            let sta_col = if self.player.stamina < 30.0 { 
                Color::rgb8(200, 170, 20) 
            } else { 
                Color::rgb8(50, 200, 70) 
            };
            backend.draw_sprite(Self::q(0, hud_left - (bar_w - bar_w * sta_ratio) * 0.5, sta_y, 
                bar_w * sta_ratio, 3.0, sta_col, 9.2));
        }

        // HP/STA dots
        let dot_x = self.sl(8.0);
        backend.draw_sprite(Self::q(0, dot_x, hp_y, 3.0, 4.0, Color::rgb8(210, 40, 30), 9.5));
        backend.draw_sprite(Self::q(0, dot_x, sta_y, 3.0, 3.0, Color::rgb8(50, 200, 70), 9.5));

        // Controls hint (bottom-right corner)
        let hint_x = self.sl(280.0);
        let hint_y = self.st(172.0);
        let hint_col = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.25 };
        backend.draw_sprite(Self::q(0, hint_x, hint_y, 70.0, 8.0, 
            Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 }, 9.0));
        
        // WASD dots
        backend.draw_sprite(Self::q(0, self.sl(256.0), hint_y, 3.0, 3.0, hint_col, 9.1));
        backend.draw_sprite(Self::q(0, self.sl(260.0), hint_y, 3.0, 3.0, hint_col, 9.1));
        backend.draw_sprite(Self::q(0, self.sl(264.0), hint_y, 3.0, 3.0, hint_col, 9.1));
        backend.draw_sprite(Self::q(0, self.sl(268.0), hint_y, 3.0, 3.0, hint_col, 9.1));
        
        // Z (attack) and X (dodge) indicators
        backend.draw_sprite(Self::q(0, self.sl(278.0), hint_y, 4.0, 3.0, Color::rgb8(200, 160, 50), 9.1));
        backend.draw_sprite(Self::q(0, self.sl(286.0), hint_y, 4.0, 3.0, Color::rgb8(50, 150, 220), 9.1));

        // Death screen
        if matches!(self.player.state, PlayerState::Dead) {
            backend.draw_sprite(Self::q(0, self.camera.x, self.camera.y, 
                CANVAS_WIDTH as f32, CANVAS_HEIGHT as f32, 
                Color { r: 0.3, g: 0.0, b: 0.0, a: 0.55 }, 10.0));
            
            // "YOU DIED" represented as pixel bars
            let y = self.st(80.0);
            let bars = [
                (self.sl(100.0), y, 3.0, 16.0),
                (self.sl(108.0), y + 6.0, 12.0, 3.0),
                (self.sl(108.0), y, 3.0, 8.0),
                (self.sl(120.0), y, 3.0, 16.0),
                (self.sl(128.0), y, 3.0, 16.0),
                (self.sl(128.0), y, 12.0, 3.0),
                (self.sl(128.0), y + 6.0, 8.0, 3.0),
                (self.sl(128.0), y + 13.0, 12.0, 3.0),
            ];
            for (bx, by, bw, bh) in bars {
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
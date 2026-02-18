//! 2D orthographic camera with screenshake support.

use glam::{Mat4, Vec2};
use serde::{Deserialize, Serialize};

/// 2D orthographic camera resource.
///
/// The camera defines what portion of the world is visible on the canvas.
/// Game systems update `target` to follow the player; the camera smoothly
/// lerps to the target each frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera2D {
    /// Current world-space position of the camera center.
    pub position: Vec2,
    /// Desired position the camera is moving toward.
    pub target: Vec2,
    /// Zoom level (1.0 = one world pixel = one canvas pixel).
    pub zoom: f32,
    /// Smoothing factor [0.0, 1.0]. Higher = snappier. `1.0` = instant follow.
    pub smoothing: f32,
    /// Active screenshake offset (world space). Set by the screenshake system.
    pub shake_offset: Vec2,
    /// Remaining screenshake duration in seconds.
    pub shake_remaining: f32,
    /// Screenshake magnitude in pixels.
    pub shake_magnitude: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            // Start centred on the 320×180 canvas in Y-down pixel space.
            position:        Vec2::new(160.0, 90.0),
            target:          Vec2::new(160.0, 90.0),
            zoom:            1.0,
            smoothing:       0.1,
            shake_offset:    Vec2::ZERO,
            shake_remaining: 0.0,
            shake_magnitude: 0.0,
        }
    }
}

impl Camera2D {
    /// Update the camera each frame (smooth follow + screenshake decay).
    ///
    /// Call from a `PostUpdate` system.
    pub fn update(&mut self, dt: f32) {
        // Smooth follow
        let t = (1.0 - (1.0 - self.smoothing).powf(dt * 60.0)).clamp(0.0, 1.0);
        self.position = self.position.lerp(self.target, t);

        // Screenshake decay
        if self.shake_remaining > 0.0 {
            self.shake_remaining = (self.shake_remaining - dt).max(0.0);
            let decay = self.shake_remaining / self.shake_magnitude.max(0.001);
            let angle = rand_f32() * std::f32::consts::TAU;
            let mag = self.shake_magnitude * decay;
            self.shake_offset = Vec2::new(angle.cos() * mag, angle.sin() * mag);
        } else {
            self.shake_offset = Vec2::ZERO;
        }
    }

    /// Trigger a screenshake.
    pub fn shake(&mut self, magnitude: f32, duration: f32) {
        self.shake_magnitude = magnitude;
        self.shake_remaining = duration;
    }

    /// Snap the camera to the target immediately (no smoothing).
    pub fn snap(&mut self) {
        self.position = self.target;
    }

    /// World-space position visible at canvas center (accounting for shake).
    pub fn view_center(&self) -> Vec2 {
        self.position + self.shake_offset
    }

    /// Build the orthographic view-projection matrix for the GPU.
    ///
    /// Uses a **Y-down, top-left origin** pixel-space coordinate system so
    /// that sprite positions map directly to canvas pixels:
    /// - `(0, 0)` = top-left of canvas
    /// - `(canvas_w, canvas_h)` = bottom-right of canvas
    ///
    /// `camera.position` is the world-space pixel that appears at the
    /// centre of the canvas.
    pub fn view_projection(&self, canvas_w: f32, canvas_h: f32) -> Mat4 {
        let center = self.view_center();
        let half_w = canvas_w / (2.0 * self.zoom);
        let half_h = canvas_h / (2.0 * self.zoom);

        // orthographic_rh(left, right, bottom, top, near, far)
        // Y-down: smaller y → screen top → wgpu NDC +1
        //         larger  y → screen bot → wgpu NDC -1
        // So pass: bottom = center.y + half_h  (large y = screen bottom)
        //          top    = center.y - half_h  (small y = screen top)
        Mat4::orthographic_rh(
            center.x - half_w,
            center.x + half_w,
            center.y + half_h,   // bottom in Y-down
            center.y - half_h,   // top    in Y-down
            -1000.0, 1000.0,
        )
    }

    /// Convert a canvas-space pixel position to world space (Y-down).
    pub fn canvas_to_world(&self, canvas_pos: Vec2, canvas_w: f32, canvas_h: f32) -> Vec2 {
        let half_w = canvas_w / (2.0 * self.zoom);
        let half_h = canvas_h / (2.0 * self.zoom);
        let center = self.view_center();
        Vec2::new(
            center.x + (canvas_pos.x / canvas_w - 0.5) * 2.0 * half_w,
            center.y + (canvas_pos.y / canvas_h - 0.5) * 2.0 * half_h,
        )
    }
}

/// Pseudo-random float [0.0, 1.0] — used only for screenshake.
/// Not suitable for gameplay randomness (use a seeded RNG for that).
fn rand_f32() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (ns % 10000) as f32 / 10000.0
}

//! Winit platform layer — owns the window, wgpu backend, and game loop.
//!
//! Keyboard bindings
//! ─────────────────
//! WASD / arrow keys — move
//! Z                 — light attack
//! X                 — dodge roll
//! R                 — respawn
//! Escape            — quit

use std::sync::Arc;
use std::time::Instant;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
use glam::Mat4;

use gur_render::{WgpuBackend, CANVAS_WIDTH, CANVAS_HEIGHT};

use crate::scene::{Input, Scene};

// ─── Configuration ────────────────────────────────────────────────────────────
pub struct PlatformConfig {
    pub title:         String,
    pub window_width:  u32,
    pub window_height: u32,
}

// ─── Application handler ──────────────────────────────────────────────────────
pub struct GurApp {
    config:  PlatformConfig,

    // Live engine state (initialised in resumed())
    window:  Option<Arc<Window>>,
    backend: Option<WgpuBackend>,

    // Game state
    scene:      Scene,
    input:      Input,
    /// Keys that were pressed just this frame (cleared at end of frame)
    just_atk:   bool,
    just_dodge: bool,

    // Timing
    last_frame: Instant,
}

impl GurApp {
    pub fn new(config: PlatformConfig) -> Self {
        Self {
            config,
            window:     None,
            backend:    None,
            scene:      Scene::new(),
            input:      Input::default(),
            just_atk:   false,
            just_dodge: false,
            last_frame: Instant::now(),
        }
    }
}

// ─── Camera ortho matrix from scene camera position ───────────────────────────
fn camera_vp(cam_x: f32, cam_y: f32) -> Mat4 {
    let cw = CANVAS_WIDTH  as f32;
    let ch = CANVAS_HEIGHT as f32;
    let half_w = cw * 0.5;
    let half_h = ch * 0.5;
    // Y-down: bottom = cam_y + half_h, top = cam_y - half_h
    Mat4::orthographic_rh(
        cam_x - half_w,
        cam_x + half_w,
        cam_y + half_h,
        cam_y - half_h,
        -1000.0, 1000.0,
    )
}

// ─── winit ApplicationHandler ─────────────────────────────────────────────────
impl ApplicationHandler for GurApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        // Create window
        let attrs = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(
                self.config.window_width,
                self.config.window_height,
            ))
            .with_resizable(true);

        let window = Arc::new(
            event_loop.create_window(attrs)
                .expect("Failed to create window"),
        );
        log::info!("Window created: {}×{}", self.config.window_width, self.config.window_height);

        match WgpuBackend::new(window.clone()) {
            Ok(backend) => {
                log::info!("wgpu backend initialised");
                self.backend = Some(backend);
            }
            Err(e) => {
                log::error!("wgpu init failed: {e}");
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            // ── Quit ──────────────────────────────────────────────────────
            WindowEvent::CloseRequested => {
                log::info!("Close requested — exiting");
                event_loop.exit();
            }

            // ── Resize ────────────────────────────────────────────────────
            WindowEvent::Resized(size) => {
                if let Some(b) = &mut self.backend {
                    b.resize(size.width, size.height);
                }
            }

            // ── Keyboard ──────────────────────────────────────────────────
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),

                    // Movement
                    PhysicalKey::Code(KeyCode::KeyW | KeyCode::ArrowUp)    => self.input.up    = pressed,
                    PhysicalKey::Code(KeyCode::KeyS | KeyCode::ArrowDown)  => self.input.down  = pressed,
                    PhysicalKey::Code(KeyCode::KeyA | KeyCode::ArrowLeft)  => self.input.left  = pressed,
                    PhysicalKey::Code(KeyCode::KeyD | KeyCode::ArrowRight) => self.input.right = pressed,

                    // Actions (just-pressed only)
                    PhysicalKey::Code(KeyCode::KeyZ | KeyCode::KeyJ) => {
                        if pressed { self.just_atk = true; }
                    }
                    PhysicalKey::Code(KeyCode::KeyX | KeyCode::KeyK) => {
                        if pressed { self.just_dodge = true; }
                    }

                    // Respawn
                    PhysicalKey::Code(KeyCode::KeyR) => {
                        if pressed { self.scene = Scene::new(); }
                    }

                    _ => {}
                }
            }

            // ── Render frame ──────────────────────────────────────────────
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.05);
                self.last_frame = now;

                // Compose per-frame input
                let frame_input = Input {
                    up:     self.input.up,
                    down:   self.input.down,
                    left:   self.input.left,
                    right:  self.input.right,
                    attack: self.just_atk,
                    dodge:  self.just_dodge,
                };
                self.just_atk   = false;
                self.just_dodge = false;

                // Tick game scene
                self.scene.update(dt, &frame_input);

                // Push camera matrix to GPU
                if let Some(backend) = &mut self.backend {
                    let cam = self.scene.camera_with_shake();
                    backend.update_camera(camera_vp(cam.x, cam.y));

                    // Draw everything
                    self.scene.draw(backend);
                }

                // Request next frame
                if let Some(w) = &self.window { w.request_redraw(); }
            }

            _ => {}
        }
    }
}

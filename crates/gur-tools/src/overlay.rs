//! Performance overlay — FPS, frame time, entity count.

use std::collections::VecDeque;

/// HUD overlay displaying real-time performance metrics.
#[derive(Debug)]
pub struct PerfOverlay {
    /// Whether the overlay is visible.
    pub visible: bool,
    /// Rolling frame time samples (last 60 frames).
    frame_times: VecDeque<f32>,
    /// Current entity count (updated each frame by an Update system).
    pub entity_count: usize,
}

impl PerfOverlay {
    /// Create a new overlay (hidden by default).
    pub fn new() -> Self {
        Self {
            visible: false,
            frame_times: VecDeque::with_capacity(60),
            entity_count: 0,
        }
    }

    /// Record a frame time sample.
    pub fn record_frame(&mut self, dt: f32) {
        if self.frame_times.len() >= 60 {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(dt);
    }

    /// Current frames per second (average over last 60 frames).
    pub fn fps(&self) -> f32 {
        if self.frame_times.is_empty() { return 0.0; }
        let avg_dt: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 }
    }

    /// Average frame time in milliseconds.
    pub fn frame_time_ms(&self) -> f32 {
        if self.frame_times.is_empty() { return 0.0; }
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32 * 1000.0
    }

    /// Toggle visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

impl Default for PerfOverlay {
    fn default() -> Self { Self::new() }
}

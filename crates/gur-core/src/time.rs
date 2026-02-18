//! Game time tracking.
//!
//! - [`Time`] — delta time, elapsed time, frame count. Updated once per frame.
//! - [`FixedTime`] — fixed-step accumulator for physics/combat ticks.
//!
//! Systems that need `dt` should read from `Time`. Systems that need
//! deterministic ticks (physics, hitbox checks) should use `FixedTime`.

use std::time::{Duration, Instant};

/// Per-frame time information. Inserted as a resource into the ECS world.
///
/// Updated by the engine at the start of each main loop iteration.
#[derive(Debug, Clone)]
pub struct Time {
    /// Wall-clock time when the engine started.
    start: Instant,
    /// Time of the previous frame (for delta computation).
    last_frame: Instant,
    /// Duration of the most recent frame. Clamped to [`Self::MAX_DELTA`].
    delta: Duration,
    /// Total elapsed time since engine start (not counting pause time).
    elapsed: Duration,
    /// Number of frames rendered since startup.
    frame_count: u64,
    /// When `true`, `delta` returns zero (useful for paused state).
    paused: bool,
}

impl Default for Time {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_frame: now,
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
            frame_count: 0,
            paused: false,
        }
    }
}

impl Time {
    /// Maximum allowed delta to prevent physics explosions after lag spikes.
    pub const MAX_DELTA: Duration = Duration::from_millis(100);

    /// Create a fresh `Time` resource. Call once at engine startup.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the clock. Call exactly once per frame, before running systems.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let raw_delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        self.delta = raw_delta.min(Self::MAX_DELTA);
        if !self.paused {
            self.elapsed += self.delta;
        }
        self.frame_count += 1;
    }

    /// Delta time in seconds (f32). The value game systems should use for movement.
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        if self.paused {
            0.0
        } else {
            self.delta.as_secs_f32()
        }
    }

    /// Delta time as a [`Duration`].
    #[inline]
    pub fn delta(&self) -> Duration {
        if self.paused { Duration::ZERO } else { self.delta }
    }

    /// Total elapsed engine time in seconds.
    #[inline]
    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    /// Total elapsed engine time as a [`Duration`].
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Frame counter since engine start.
    #[inline]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Pause or unpause time. When paused, `delta_seconds()` returns `0.0`.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if !paused {
            // Reset last_frame so the first tick after unpause doesn't spike.
            self.last_frame = Instant::now();
        }
    }

    /// Returns `true` if the clock is paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

/// Fixed-timestep accumulator for deterministic systems (physics, combat ticks).
///
/// Usage: call `accumulate(dt)` each frame, then loop `try_consume()` until
/// it returns `None` to process all pending fixed steps.
///
/// ```rust
/// # use gur_core::time::FixedTime;
/// let mut fixed = FixedTime::new(1.0 / 60.0);
/// fixed.accumulate(0.025);              // Two frames worth of fixed steps
/// while let Some(step) = fixed.try_consume() {
///     // run physics / hitbox detection for one fixed step of `step` seconds
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FixedTime {
    /// Duration of each fixed step in seconds.
    timestep: f32,
    /// Leftover time not yet consumed into a full step.
    accumulator: f32,
    /// Safety cap: drop steps if more than this many accumulate (avoids spiral of death).
    max_steps: u32,
}

impl FixedTime {
    /// Create a `FixedTime` with the given timestep (e.g. `1.0/60.0` for 60 Hz).
    pub fn new(timestep: f32) -> Self {
        assert!(timestep > 0.0, "Fixed timestep must be positive");
        Self {
            timestep,
            accumulator: 0.0,
            max_steps: 8,
        }
    }

    /// Standard 60 Hz fixed update.
    pub fn sixty_hz() -> Self {
        Self::new(1.0 / 60.0)
    }

    /// Add real elapsed time (in seconds) to the accumulator.
    #[inline]
    pub fn accumulate(&mut self, delta_seconds: f32) {
        self.accumulator += delta_seconds;
        // Hard cap to prevent unbounded accumulation.
        let max = self.timestep * self.max_steps as f32;
        if self.accumulator > max {
            self.accumulator = max;
        }
    }

    /// Consume one fixed step if available. Returns `Some(step_duration)` or `None`.
    #[inline]
    pub fn try_consume(&mut self) -> Option<f32> {
        if self.accumulator >= self.timestep {
            self.accumulator -= self.timestep;
            Some(self.timestep)
        } else {
            None
        }
    }

    /// Interpolation factor [0, 1] for blending render state between fixed steps.
    /// Use this to smooth visual positions even when render FPS > physics FPS.
    #[inline]
    pub fn alpha(&self) -> f32 {
        self.accumulator / self.timestep
    }

    /// The configured fixed timestep in seconds.
    #[inline]
    pub fn timestep(&self) -> f32 {
        self.timestep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_time_consumes_correct_steps() {
        let mut ft = FixedTime::new(1.0 / 60.0);
        ft.accumulate(1.0 / 30.0); // Exactly 2 steps worth
        assert!(ft.try_consume().is_some());
        assert!(ft.try_consume().is_some());
        assert!(ft.try_consume().is_none());
    }

    #[test]
    fn fixed_time_alpha_bounded() {
        let mut ft = FixedTime::new(1.0 / 60.0);
        ft.accumulate(0.008); // Less than one step
        let a = ft.alpha();
        assert!(a >= 0.0 && a <= 1.0, "alpha={a}");
    }

    #[test]
    fn time_paused_returns_zero_delta() {
        let mut t = Time::new();
        t.tick();
        t.set_paused(true);
        t.tick();
        assert_eq!(t.delta_seconds(), 0.0);
    }
}

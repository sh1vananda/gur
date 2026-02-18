//! Spatial audio helpers — distance-based volume falloff for world sounds.

use glam::Vec2;

/// Calculate the volume [0.0, 1.0] for a world-space sound based on
/// listener and emitter positions.
pub fn calculate_volume(listener: Vec2, emitter: Vec2, max_range: f32, rolloff: f32) -> f32 {
    let distance = listener.distance(emitter);
    if distance >= max_range {
        return 0.0_f32;
    }
    let t = distance / max_range;
    (1.0_f32 - t.powf(rolloff)).clamp(0.0_f32, 1.0_f32)
}

/// Calculate a stereo pan value [-1.0, 1.0] based on emitter relative to listener.
pub fn calculate_pan(listener: Vec2, emitter: Vec2) -> f32 {
    let dx = emitter.x - listener.x;
    (dx / 300.0_f32).clamp(-1.0_f32, 1.0_f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_at_zero_distance_is_one() {
        let v = calculate_volume(Vec2::ZERO, Vec2::ZERO, 500.0, 1.0);
        assert!((v - 1.0_f32).abs() < 1e-5);
    }

    #[test]
    fn volume_at_max_range_is_zero() {
        let v = calculate_volume(Vec2::ZERO, Vec2::new(500.0, 0.0), 500.0, 1.0);
        assert_eq!(v, 0.0_f32);
    }

    #[test]
    fn pan_positive_for_right_emitter() {
        let p = calculate_pan(Vec2::ZERO, Vec2::new(100.0, 0.0));
        assert!(p > 0.0_f32);
    }
}

//! [`AudioManager`] — thin wrapper around kira's audio manager.

use kira::manager::{AudioManager as KiraManager, AudioManagerSettings, backend::cpal::CpalBackend};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::{TrackBuilder, TrackHandle};
use kira::tween::Tween;

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Audio manager resource — create once, store in `GurWorld`.
pub struct AudioManager {
    inner: KiraManager<CpalBackend>,
    /// Named audio tracks (music, sfx, ambient).
    tracks: HashMap<String, TrackHandle>,
    /// Currently playing music handle (for crossfade / stop).
    current_music: Option<StaticSoundHandle>,
}

impl AudioManager {
    /// Initialise the kira backend and create default tracks.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut inner = KiraManager::new(AudioManagerSettings::default())?;

        let mut tracks = HashMap::new();

        let music_track = inner.add_sub_track(TrackBuilder::new().volume(0.7))?;
        tracks.insert("music".to_owned(), music_track);

        let sfx_track = inner.add_sub_track(TrackBuilder::new().volume(1.0))?;
        tracks.insert("sfx".to_owned(), sfx_track);

        let ambient_track = inner.add_sub_track(TrackBuilder::new().volume(0.4))?;
        tracks.insert("ambient".to_owned(), ambient_track);

        Ok(Self { inner, tracks, current_music: None })
    }

    /// Play background music (looping). Fades out previous track if any.
    pub fn play_music(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        // Stop current music with a short fade
        if let Some(ref mut handle) = self.current_music {
            let _ = handle.stop(Tween {
                duration: Duration::from_millis(800),
                ..Default::default()
            });
        }

        let mut sound_data = StaticSoundData::from_file(path.as_ref())?;
        // Enable looping via the settings on the data itself
        sound_data.settings.loop_region = Some(kira::sound::Region {
            start: kira::sound::PlaybackPosition::Seconds(0.0),
            end: kira::sound::EndPosition::EndOfAudio,
        });

        let handle = self.inner.play(sound_data)?;
        self.current_music = Some(handle);
        Ok(())
    }

    /// Stop the current music track with a fade-out.
    pub fn stop_music(&mut self) {
        if let Some(ref mut handle) = self.current_music {
            let _ = handle.stop(Tween {
                duration: Duration::from_millis(500),
                ..Default::default()
            });
            self.current_music = None;
        }
    }

    /// Play a one-shot sound effect.
    pub fn play_sfx(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let sound_data = StaticSoundData::from_file(path.as_ref())?;
        self.inner.play(sound_data)?;
        Ok(())
    }

    /// Set the master volume [0.0, 1.0].
    pub fn set_master_volume(&mut self, volume: f64) {
        let _ = self.inner.main_track().set_volume(volume, Tween::default());
    }

    /// Set the volume of a named track ("music", "sfx", "ambient").
    pub fn set_track_volume(&mut self, track: &str, volume: f64) {
        if let Some(t) = self.tracks.get_mut(track) {
            let _ = t.set_volume(volume, Tween::default());
        }
    }
}

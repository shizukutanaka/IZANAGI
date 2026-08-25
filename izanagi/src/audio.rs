//! Audio mixer.
//!
//! Tracks named PCM clips and currently playing voices. The null backend
//! advances voice positions every frame so timing tests work; a real
//! audio backend (e.g. cpal) reads `mix_into` to fill the output stream.

use crate::audio_pcm::PcmBuffer;
use std::collections::HashMap;

/// A handle to a playing sound.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Voice(u64);

struct Playing {
    clip: String,
    volume: f32,
    /// Sample-frame cursor (per-channel position in the clip).
    cursor: usize,
    looping: bool,
}

/// Audio mixer.
pub struct Audio {
    clips: HashMap<String, PcmBuffer>,
    voices: HashMap<Voice, Playing>,
    master: f32,
    next: u64,
}

impl Audio {
    /// Create a mixer at full volume with no clips loaded.
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            voices: HashMap::new(),
            master: 1.0,
            next: 1,
        }
    }

    /// Register a PCM clip under `name`. Replaces any clip with the same name.
    pub fn add_clip(&mut self, name: impl Into<String>, clip: PcmBuffer) {
        self.clips.insert(name.into(), clip);
    }

    /// Drop a clip. Voices currently playing it continue to their existing
    /// cursor but the clip can no longer be looked up.
    pub fn remove_clip(&mut self, name: &str) -> Option<PcmBuffer> {
        self.clips.remove(name)
    }

    /// Number of registered clips.
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    /// Play a clip by name. Returns a voice handle. The handle is valid even
    /// if the clip is missing — the voice simply produces silence.
    pub fn play(&mut self, name: &str, volume: f32) -> Voice {
        self.play_with_loop(name, volume, false)
    }

    /// Play a clip by name on a loop.
    pub fn play_loop(&mut self, name: &str, volume: f32) -> Voice {
        self.play_with_loop(name, volume, true)
    }

    fn play_with_loop(&mut self, name: &str, volume: f32, looping: bool) -> Voice {
        let v = Voice(self.next);
        self.next += 1;
        self.voices.insert(
            v,
            Playing {
                clip: name.to_string(),
                volume: volume.clamp(0.0, 1.0),
                cursor: 0,
                looping,
            },
        );
        v
    }

    /// Stop a playing voice.
    pub fn stop(&mut self, v: Voice) {
        self.voices.remove(&v);
    }

    /// Stop every voice.
    pub fn stop_all(&mut self) {
        self.voices.clear();
    }

    /// Set master volume (0.0 to 1.0).
    pub fn set_master(&mut self, volume: f32) {
        self.master = volume.clamp(0.0, 1.0);
    }

    /// Current master volume.
    pub fn master(&self) -> f32 {
        self.master
    }

    /// Number of currently active voices.
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    /// Mix all active voices into `out` (interleaved, length = frames * 2).
    /// Stops one-shot voices that reach the end of their clip.
    ///
    /// Backends call this once per audio buffer.
    pub fn mix_into(&mut self, out: &mut [f32], frames: usize) {
        for s in out.iter_mut() {
            *s = 0.0;
        }
        let master = self.master;
        let mut to_remove: Vec<Voice> = Vec::new();
        for (&voice, p) in self.voices.iter_mut() {
            let Some(clip) = self.clips.get(&p.clip) else {
                continue;
            };
            let ch = clip.channels.max(1) as usize;
            for f in 0..frames {
                let pos = p.cursor + f;
                let clip_len_frames = clip.samples.len() / ch;
                let frame_idx = if p.looping && clip_len_frames > 0 {
                    pos % clip_len_frames
                } else {
                    pos
                };
                if frame_idx >= clip_len_frames {
                    if !p.looping {
                        to_remove.push(voice);
                    }
                    break;
                }
                let l = clip.samples[frame_idx * ch];
                let r = if ch >= 2 {
                    clip.samples[frame_idx * ch + 1]
                } else {
                    l
                };
                let gain = p.volume * master;
                out[f * 2] += l * gain;
                out[f * 2 + 1] += r * gain;
            }
            p.cursor += frames;
            // Normalize cursor for looping voices to prevent unbounded growth.
            if p.looping {
                let clip_len_frames = clip.samples.len() / ch;
                if clip_len_frames > 0 {
                    p.cursor %= clip_len_frames;
                }
            }
        }
        for v in to_remove {
            self.voices.remove(&v);
        }
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_pcm::sine_wave;

    #[test]
    fn play_and_stop() {
        let mut a = Audio::new();
        let v = a.play("ding", 1.0);
        assert_eq!(a.voice_count(), 1);
        a.stop(v);
        assert_eq!(a.voice_count(), 0);
    }

    #[test]
    fn stop_all() {
        let mut a = Audio::new();
        a.play("a", 1.0);
        a.play("b", 1.0);
        a.play("c", 1.0);
        a.stop_all();
        assert_eq!(a.voice_count(), 0);
    }

    #[test]
    fn master_volume_clamps() {
        let mut a = Audio::new();
        a.set_master(2.0);
        assert_eq!(a.master(), 1.0);
        a.set_master(-1.0);
        assert_eq!(a.master(), 0.0);
    }

    #[test]
    fn add_and_remove_clip() {
        let mut a = Audio::new();
        a.add_clip("tone", sine_wave(440.0, 0.1, 44100));
        assert_eq!(a.clip_count(), 1);
        assert!(a.remove_clip("tone").is_some());
        assert_eq!(a.clip_count(), 0);
    }

    #[test]
    fn mix_into_produces_audio() {
        let mut a = Audio::new();
        a.add_clip("tone", sine_wave(440.0, 1.0, 44100));
        a.play("tone", 1.0);
        let mut buf = vec![0.0f32; 2 * 256]; // 256 stereo frames
        a.mix_into(&mut buf, 256);
        let energy: f32 = buf.iter().map(|s| s * s).sum();
        assert!(energy > 0.0, "mix_into produced silence");
    }

    #[test]
    fn one_shot_voice_stops_at_end() {
        let mut a = Audio::new();
        // very short clip — 100 frames
        let clip = PcmBuffer {
            samples: vec![0.5; 100],
            channels: 1,
            sample_rate: 44100,
        };
        a.add_clip("blip", clip);
        a.play("blip", 1.0);
        assert_eq!(a.voice_count(), 1);
        let mut buf = vec![0.0f32; 2 * 200]; // 200 frames > 100
        a.mix_into(&mut buf, 200);
        assert_eq!(a.voice_count(), 0, "one-shot should auto-stop");
    }

    #[test]
    fn looping_voice_keeps_going() {
        let mut a = Audio::new();
        let clip = PcmBuffer {
            samples: vec![0.5; 100],
            channels: 1,
            sample_rate: 44100,
        };
        a.add_clip("loop", clip);
        a.play_loop("loop", 1.0);
        let mut buf = vec![0.0f32; 2 * 1000];
        a.mix_into(&mut buf, 1000);
        assert_eq!(a.voice_count(), 1, "looping should not auto-stop");
    }

    #[test]
    fn missing_clip_produces_silence_not_panic() {
        let mut a = Audio::new();
        a.play("nonexistent", 1.0);
        let mut buf = vec![0.0f32; 2 * 64];
        a.mix_into(&mut buf, 64);
        let energy: f32 = buf.iter().map(|s| s * s).sum();
        assert_eq!(energy, 0.0);
    }
}

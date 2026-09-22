//! Minimal WAV / PCM loader — no external dependencies.
//!
//! Parses standard PCM WAV files (8-bit unsigned or 16-bit signed,
//! mono or stereo). Returns samples as normalized f32 in [−1, 1].

use crate::error::{Error, Result};

/// Decoded audio data.
#[derive(Clone, Debug)]
pub struct PcmBuffer {
    /// Interleaved f32 samples in [−1, 1].
    pub samples: Vec<f32>,
    /// Number of audio channels (1 = mono, 2 = stereo).
    pub channels: u16,
    /// Samples per second.
    pub sample_rate: u32,
}

impl PcmBuffer {
    /// Duration in seconds.
    pub fn duration(&self) -> f32 {
        if self.channels == 0 || self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / self.channels as f32 / self.sample_rate as f32
    }

    /// Convert stereo to mono by averaging channels.
    pub fn to_mono(&self) -> Self {
        if self.channels == 1 {
            return self.clone();
        }
        // `chunks_exact` drops a trailing odd sample — `chunks` would hand
        // the last frame a 1-element slice and `c[1]` would panic on a
        // stereo buffer with an odd sample count.
        let mono: Vec<f32> = self
            .samples
            .chunks_exact(2)
            .map(|c| (c[0] + c[1]) * 0.5)
            .collect();
        PcmBuffer {
            samples: mono,
            channels: 1,
            sample_rate: self.sample_rate,
        }
    }
}

/// Parse a WAV byte slice. Supports PCM 8-bit or 16-bit, mono or stereo.
pub fn load_wav(data: &[u8]) -> Result<PcmBuffer> {
    let err = |msg: &str| Error::Asset(format!("wav: {msg}"));

    if data.len() < 44 {
        return Err(err("file too short"));
    }
    if &data[0..4] != b"RIFF" {
        return Err(err("not a RIFF file"));
    }
    if &data[8..12] != b"WAVE" {
        return Err(err("not a WAVE file"));
    }

    // Scan for fmt chunk.
    let mut pos = 12usize;
    let mut audio_format = 0u16;
    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut bits_per_sample = 0u16;
    let mut data_start = 0usize;
    let mut data_len = 0usize;

    while pos + 8 <= data.len() {
        let tag = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
            as usize;
        pos += 8;
        if tag == b"fmt " {
            // `size` is the *claimed* size — a truncated file can claim a
            // 16-byte fmt body with only a few real bytes left, and reading
            // the fields below would index out of bounds and panic.
            if size < 16 || pos + 16 > data.len() {
                return Err(err("fmt chunk too small or truncated"));
            }
            audio_format = u16::from_le_bytes([data[pos], data[pos + 1]]);
            channels = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
            sample_rate =
                u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]);
            bits_per_sample = u16::from_le_bytes([data[pos + 14], data[pos + 15]]);
        } else if tag == b"data" {
            data_start = pos;
            data_len = size.min(data.len() - pos);
        }
        // A bogus size must not wrap the cursor (32-bit `usize` overflow
        // would revisit positions and loop forever on crafted input).
        pos = pos.saturating_add(size).saturating_add(size & 1);
    }

    if audio_format != 1 {
        return Err(err("only PCM (format 1) is supported"));
    }
    if channels == 0 || channels > 2 {
        return Err(err("only mono and stereo supported"));
    }
    if data_start == 0 {
        return Err(err("no data chunk found"));
    }

    let raw = &data[data_start..data_start + data_len];
    let samples = match bits_per_sample {
        8 => raw.iter().map(|&b| b as f32 / 127.5 - 1.0).collect(),
        16 => raw
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
            .collect(),
        _ => return Err(err(&format!("{bits_per_sample}-bit not supported"))),
    };

    Ok(PcmBuffer {
        samples,
        channels,
        sample_rate,
    })
}

/// Generate a sine-wave WAV buffer in memory (useful for tests).
pub fn sine_wave(freq: f32, duration: f32, sample_rate: u32) -> PcmBuffer {
    let n = (duration * sample_rate as f32) as usize;
    let samples = (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect();
    PcmBuffer {
        samples,
        channels: 1,
        sample_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wav(bits: u16, samples: &[u8]) -> Vec<u8> {
        let data_len = samples.len() as u32;
        let fmt_size: u32 = 16;
        let riff_size: u32 = 4 + (8 + fmt_size) + (8 + data_len);
        let block_align = bits / 8;
        let byte_rate = 44100u32 * block_align as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&riff_size.to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&fmt_size.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&44100u32.to_le_bytes());
        v.extend_from_slice(&byte_rate.to_le_bytes());
        v.extend_from_slice(&block_align.to_le_bytes());
        v.extend_from_slice(&bits.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.extend_from_slice(samples);
        v
    }

    #[test]
    fn parse_8bit_mono() {
        let wav = minimal_wav(8, &[128u8, 255, 0]);
        let buf = load_wav(&wav).unwrap();
        assert_eq!(buf.channels, 1);
        assert_eq!(buf.sample_rate, 44100);
        assert_eq!(buf.samples.len(), 3);
        assert!((buf.samples[0]).abs() < 0.01); // 128 ≈ silence
    }

    #[test]
    fn parse_16bit_mono() {
        let raw: Vec<u8> = [0i16, i16::MAX, i16::MIN]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        let wav = minimal_wav(16, &raw);
        let buf = load_wav(&wav).unwrap();
        assert_eq!(buf.samples.len(), 3);
        assert!((buf.samples[0]).abs() < 0.001);
        assert!((buf.samples[1] - 1.0).abs() < 0.001);
        assert!((buf.samples[2] + 1.0).abs() < 0.001);
    }

    #[test]
    fn reject_non_riff() {
        assert!(load_wav(b"OGGSxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").is_err());
    }

    #[test]
    fn sine_wave_duration() {
        let buf = sine_wave(440.0, 1.0, 44100);
        assert_eq!(buf.sample_rate, 44100);
        assert_eq!(buf.samples.len(), 44100);
        assert!((buf.duration() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn to_mono_ignores_trailing_odd_sample() {
        // A stereo buffer with an odd sample count: the dangling sample
        // has no partner — drop it rather than panic.
        let stereo = PcmBuffer {
            samples: vec![0.5, -0.5, 0.9],
            channels: 2,
            sample_rate: 44100,
        };
        let mono = stereo.to_mono();
        assert_eq!(mono.samples.len(), 1);
    }

    #[test]
    fn to_mono_averages() {
        let stereo = PcmBuffer {
            samples: vec![0.5, -0.5, 1.0, 0.0],
            channels: 2,
            sample_rate: 44100,
        };
        let mono = stereo.to_mono();
        assert_eq!(mono.channels, 1);
        assert!((mono.samples[0] - 0.0).abs() < 1e-5);
        assert!((mono.samples[1] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn load_wav_never_panics_on_any_truncation_or_single_byte_corruption() {
        // Same contract as the text parser's garbage fuzzers: every prefix
        // length and every single-byte flip of a valid file must be Err or
        // Ok — never panic. The truncated-fmt OOB above is what this sweep
        // would have caught.
        // Two shapes: fmt-first (the common file) and fmt-LAST — only the
        // second can be truncated mid-fmt-payload, which is the OOB family
        // fixed above. A sweep of just the first shape cannot reach it: the
        // 44-byte floor rejects every prefix before the chunk loop.
        let shapes = [minimal_wav(8, &[0, 1, 2, 3, 252, 253, 254, 255]), {
            let mut d = Vec::new();
            d.extend_from_slice(b"RIFF");
            d.extend_from_slice(&64u32.to_le_bytes());
            d.extend_from_slice(b"WAVE");
            d.extend_from_slice(b"JUNK");
            d.extend_from_slice(&12u32.to_le_bytes());
            d.extend_from_slice(&[0; 12]);
            d.extend_from_slice(b"fmt ");
            d.extend_from_slice(&16u32.to_le_bytes());
            d.extend_from_slice(&[0; 16]);
            d
        }];
        for full in &shapes {
            for n in 0..=full.len() {
                let _ = load_wav(&full[..n]);
            }
            for i in 0..full.len() {
                let mut d = full.clone();
                d[i] ^= 0xFF;
                let _ = load_wav(&d);
            }
        }
        let mut g = vec![0xAAu8; 96];
        for seed in 0..512u32 {
            for (i, b) in g.iter_mut().enumerate() {
                *b = seed.wrapping_mul(31).wrapping_add(i as u32) as u8;
            }
            let _ = load_wav(&g);
        }
    }

    #[test]
    fn a_truncated_fmt_chunk_errors_instead_of_panicking() {
        // The claimed chunk size used to be trusted: a file ending in
        // `fmt ` + size=16 + 4 real bytes indexed past the buffer.
        let mut d = Vec::new();
        d.extend_from_slice(b"RIFF");
        d.extend_from_slice(&64u32.to_le_bytes());
        d.extend_from_slice(b"WAVE");
        d.extend_from_slice(b"JUNK");
        d.extend_from_slice(&12u32.to_le_bytes());
        d.extend_from_slice(&[0; 12]);
        d.extend_from_slice(b"fmt ");
        d.extend_from_slice(&16u32.to_le_bytes());
        d.extend_from_slice(&[1, 0, 1, 0]);
        assert_eq!(d.len(), 44);
        assert!(load_wav(&d).is_err());
    }

    #[test]
    fn a_huge_chunk_size_cannot_wrap_the_cursor() {
        // `pos += size` must saturate, not wrap — a wrapped cursor revisits
        // earlier positions and spins forever (wasm32 `usize`).
        let mut d = Vec::new();
        d.extend_from_slice(b"RIFF");
        d.extend_from_slice(&64u32.to_le_bytes());
        d.extend_from_slice(b"WAVE");
        d.extend_from_slice(b"JUNK");
        d.extend_from_slice(&u32::MAX.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        assert!(load_wav(&d).is_err());
    }
}

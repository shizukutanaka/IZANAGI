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
        let mono: Vec<f32> = self
            .samples
            .chunks(2)
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
            if size < 16 {
                return Err(err("fmt chunk too small"));
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
        pos += size + (size & 1); // chunks are word-aligned
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
}

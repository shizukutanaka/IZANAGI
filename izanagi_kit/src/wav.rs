//! RIFF/WAVE codec — PCM `fmt ` + `data` chunks, the audio sibling of
//! [`bmp`](crate::bmp)/[`png`](crate::png). Only integer PCM (format
//! tag 1) is supported: IEEE-float WAV (tag 3) is rejected because the
//! kit has no float types; `WAVEFORMATEXTENSIBLE` (tag `0xFFFE`) with
//! the PCM GUID subtype is accepted.
//!
//! [`parse`] validates chunk bounds and `fmt ` consistency; [`encode`]
//! emits a canonical `RIFF/fmt(16)/data` file. Samples stay as raw
//! interleaved little-endian bytes — channel/interleave semantics are
//! the caller's. Non-PCM chunks (LIST, fact, cue) are skipped.
//!
//! ```
//! use izanagi_kit::wav::{Wav, parse, encode};
//! let w = Wav { channels: 1, sample_rate: 8000, bits: 8, data: vec![128, 255, 0] };
//! assert_eq!(parse(&encode(&w)).unwrap(), w);
//! ```

use std::vec::Vec;

/// A decoded WAVE file: PCM format plus the raw `data` payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wav {
    /// Channel count (1+).
    pub channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Bits per sample: 8, 16, 24, or 32.
    pub bits: u16,
    /// Interleaved sample bytes, little-endian, `block_align`-padded.
    pub data: Vec<u8>,
}

impl Wav {
    /// Bytes per channel-frame (`channels * bits/8`).
    pub fn block_align(&self) -> u16 {
        self.channels.saturating_mul(self.bits / 8)
    }
    /// Whole channel-frames in `data` (trailing partial dropped).
    pub fn frames(&self) -> usize {
        let b = self.block_align() as usize;
        self.data.len().checked_div(b).unwrap_or(0)
    }
    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        self.frames() as u64 * 1000 / self.sample_rate as u64
    }
}

fn le16(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(i)?, *d.get(i + 1)?]))
}
fn le32(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(i)?,
        *d.get(i + 1)?,
        *d.get(i + 2)?,
        *d.get(i + 3)?,
    ]))
}

/// Parse `RIFF…WAVE`; `None` on truncation, non-PCM tags, or a `data`
/// payload that isn't a multiple of `block_align`.
pub fn parse(d: &[u8]) -> Option<Wav> {
    if d.len() < 12 || &d[..4] != b"RIFF" || &d[8..12] != b"WAVE" {
        return None;
    }
    let riff_end = (le32(d, 4)? as usize).saturating_add(8).min(d.len());
    let mut w = Wav {
        channels: 0,
        sample_rate: 0,
        bits: 0,
        data: Vec::new(),
    };
    let mut have_fmt = false;
    let mut have_data = false;
    let mut i = 12;
    while i + 8 <= riff_end {
        let id = &d[i..i + 4];
        let size = le32(d, i + 4)? as usize;
        let body = i + 8;
        if size > riff_end.saturating_sub(body) {
            return None;
        }
        if id == b"fmt " {
            if size < 16 {
                return None;
            }
            let tag = le16(d, body)?;
            // PCM=1; extensible=0xFFFE accepted only with PCM subformat
            // (GUID tail at body+24 = "01000000-0000-0010-8000-00aa00389b71")
            match tag {
                1 => {}
                0xFFFE => {
                    if size < 40 || le16(d, body + 24)? != 1 {
                        return None;
                    }
                }
                _ => return None, // IEEE float / a-law / etc — no float types
            }
            let channels = le16(d, body + 2)?;
            let rate = le32(d, body + 4)?;
            let bits = le16(d, body + 14)?;
            if channels == 0 || !matches!(bits, 8 | 16 | 24 | 32) {
                return None;
            }
            w.channels = channels;
            w.sample_rate = rate;
            w.bits = bits;
            have_fmt = true;
        } else if id == b"data" {
            if !have_fmt {
                return None; // data before fmt
            }
            w.data = d[body..body + size].to_vec();
            have_data = true;
        }
        // chunks are word-aligned
        i = body + size + (size & 1);
    }
    if !have_fmt || !have_data || w.data.len() % w.block_align() as usize != 0 {
        return None;
    }
    Some(w)
}

/// Canonical emission: `RIFF` + `fmt `(16) + `data`.
pub fn encode(w: &Wav) -> Vec<u8> {
    let align = w.block_align() as u32;
    let byte_rate = w.sample_rate.saturating_mul(align);
    let mut out = Vec::with_capacity(44 + w.data.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + w.data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&w.channels.to_le_bytes());
    out.extend_from_slice(&w.sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&(align as u16).to_le_bytes());
    out.extend_from_slice(&w.bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(w.data.len() as u32).to_le_bytes());
    out.extend_from_slice(&w.data);
    out
}

/// 8/16-bit PCM decode to signed 16-bit samples (8-bit unsigned bias
/// 128, higher widths truncated to the top 16 bits). Mono mixes down by
/// arithmetic mean; multi-channel keeps channel 0.
pub fn samples_i16(w: &Wav) -> Vec<i16> {
    let bytes = (w.bits / 8) as usize;
    let align = w.block_align() as usize;
    if bytes == 0 || align == 0 {
        return Vec::new();
    }
    (0..w.frames())
        .map(|f| {
            let i = f * align;
            match w.bits {
                8 => ((w.data[i] as i16) - 128) << 8,
                16 => i16::from_le_bytes([w.data[i], w.data[i + 1]]),
                24 | 32 => i16::from_le_bytes([w.data[i + bytes - 2], w.data[i + bytes - 1]]),
                _ => 0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav8(data: &[u8]) -> Wav {
        Wav {
            channels: 1,
            sample_rate: 8000,
            bits: 8,
            data: data.to_vec(),
        }
    }

    #[test]
    fn roundtrip_canonical() {
        let w = wav8(&[0, 128, 255, 1]);
        let b = encode(&w);
        assert_eq!(&b[..4], b"RIFF");
        assert_eq!(&b[8..12], b"WAVE");
        assert_eq!(b.len(), 44 + 4);
        assert_eq!(parse(&b), Some(w));
        // 16-bit stereo
        let w = Wav {
            channels: 2,
            sample_rate: 44100,
            bits: 16,
            data: vec![1, 0, 2, 0, 3, 0, 4, 0],
        };
        assert_eq!(parse(&encode(&w)), Some(w));
    }

    #[test]
    fn skipped_chunks_and_padding() {
        // fmt + LIST(odd size → pad byte) + data
        let w = wav8(&[0, 128]);
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&[0; 4]);
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&encode(&w)[20..36]);
        b.extend_from_slice(b"LIST");
        b.extend_from_slice(&3u32.to_le_bytes());
        b.extend_from_slice(b"abc");
        b.push(0); // pad
        b.extend_from_slice(b"data");
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&w.data);
        let total = (b.len() - 8) as u32;
        b[4..8].copy_from_slice(&total.to_le_bytes());
        assert_eq!(parse(&b), Some(w));
    }

    #[test]
    fn fields_and_helpers() {
        let w = Wav {
            channels: 2,
            sample_rate: 44100,
            bits: 16,
            data: vec![0; 44100 * 4 / 10],
        };
        assert_eq!(w.block_align(), 4);
        assert_eq!(w.frames(), 4410);
        assert_eq!(w.duration_ms(), 100);
    }

    #[test]
    fn samples_conversion() {
        let w = wav8(&[0, 128, 255]);
        assert_eq!(samples_i16(&w), vec![-32768, 0, 32512]);
        let w16 = Wav {
            channels: 1,
            sample_rate: 8000,
            bits: 16,
            data: vec![0, 128, 255, 127], // -32768, 32767
        };
        assert_eq!(samples_i16(&w16), vec![-32768, 32767]);
    }

    #[test]
    fn malformed_rejected() {
        let good = encode(&wav8(&[0, 1]));
        for i in 0..good.len() {
            let mut t = good.clone();
            t[i] ^= 0xFF;
            // corrupting any single byte must never panic
            let _ = parse(&t);
        }
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&good[..8]), None);
        // IEEE-float tag 3 rejected
        let mut f = encode(&wav8(&[0]));
        f[20] = 3;
        assert_eq!(parse(&f), None);
        // data not multiple of block_align
        let mut w = encode(&Wav {
            channels: 2,
            sample_rate: 8000,
            bits: 16,
            data: vec![0, 0, 0],
        });
        assert_eq!(parse(&w), None);
        w.pop();
        assert_eq!(parse(&w), None);
    }

    #[test]
    fn determinism_twice() {
        let w = wav8(&[1, 2, 3]);
        assert_eq!(encode(&w), encode(&w));
        assert_eq!(parse(&encode(&w)), parse(&encode(&w)));
    }
}

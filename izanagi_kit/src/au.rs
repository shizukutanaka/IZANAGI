//! Sun/NeXT `.au` (and `.snd`) audio file header.
//!
//! Header (24 B, big-endian): `".snd"` magic, data offset, data size
//! (`0xFFFFFFFF` = unknown), encoding id, sample rate, channel count.
//!
//! ```
//! use izanagi_kit::au::{parse, Encoding};
//!
//! let f = [
//!     0x2Eu8, 0x73, 0x6E, 0x64, // ".snd"
//!     0, 0, 0, 24,              // data offset
//!     0xFF, 0xFF, 0xFF, 0xFF,   // data size unknown
//!     0, 0, 0, 3,               // encoding 3 = 16-bit linear PCM
//!     0, 0, 0x1F, 0x40,         // rate 8000
//!     0, 0, 0, 1,               // mono
//! ];
//! let a = parse(&f).unwrap();
//! assert_eq!(a.encoding, Encoding::LinearPcm16);
//! assert_eq!(a.rate, 8000);
//! ```

/// `.au` encoding id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// 1 — 8-bit µ-law.
    MuLaw8,
    /// 2 — 8-bit linear PCM.
    LinearPcm8,
    /// 3 — 16-bit linear PCM.
    LinearPcm16,
    /// 4 — 24-bit linear PCM.
    LinearPcm24,
    /// 5 — 32-bit linear PCM.
    LinearPcm32,
    /// 6 — IEEE float.
    Float32,
    /// 7 — IEEE double.
    Float64,
    /// 27 — 8-bit A-law.
    ALaw8,
    /// Other encoding id.
    Other(u32),
}

/// Parsed `.au` header.
pub struct Au {
    /// Byte offset of the audio data.
    pub data_at: usize,
    /// Declared data size (`None` = 0xFFFFFFFF unknown).
    pub data_size: Option<u32>,
    /// Encoding.
    pub encoding: Encoding,
    /// Raw encoding id.
    pub encoding_id: u32,
    /// Sample rate in Hz.
    pub rate: u32,
    /// Channel count.
    pub channels: u32,
}

/// Parses an `.au` header; magic `.snd` and offset ≥ 24 are required.
/// When `data_size` is known it must fit inside `d`.
pub fn parse(d: &[u8]) -> Option<Au> {
    if d.len() < 24 || &d[..4] != b".snd" {
        return None;
    }
    let u32be = |o: usize| {
        ((d[o] as u32) << 24)
            | ((d[o + 1] as u32) << 16)
            | ((d[o + 2] as u32) << 8)
            | d[o + 3] as u32
    };
    let data_at = u32be(4) as usize;
    if data_at < 24 || data_at > d.len() {
        return None;
    }
    let size = u32be(8);
    if size != 0xFFFF_FFFF && data_at + size as usize > d.len() {
        return None;
    }
    let encoding_id = u32be(12);
    Some(Au {
        data_at,
        data_size: if size == 0xFFFF_FFFF {
            None
        } else {
            Some(size)
        },
        encoding: match encoding_id {
            1 => Encoding::MuLaw8,
            2 => Encoding::LinearPcm8,
            3 => Encoding::LinearPcm16,
            4 => Encoding::LinearPcm24,
            5 => Encoding::LinearPcm32,
            6 => Encoding::Float32,
            7 => Encoding::Float64,
            27 => Encoding::ALaw8,
            o => Encoding::Other(o),
        },
        encoding_id,
        rate: u32be(16),
        channels: u32be(20),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(enc: u32, size: u32) -> Vec<u8> {
        let mut v = vec![0x2E, 0x73, 0x6E, 0x64];
        v.extend_from_slice(&[0, 0, 0, 24]);
        v.extend_from_slice(&[
            (size >> 24) as u8,
            (size >> 16) as u8,
            (size >> 8) as u8,
            size as u8,
        ]);
        v.extend_from_slice(&[
            (enc >> 24) as u8,
            (enc >> 16) as u8,
            (enc >> 8) as u8,
            enc as u8,
        ]);
        v.extend_from_slice(&[0, 0, 0xAC, 0x44]); // 44100
        v.extend_from_slice(&[0, 0, 0, 2]);
        v.extend_from_slice(&[0u8; 8][..8.min(size as usize)]);
        v
    }

    #[test]
    fn parses() {
        let a = parse(&hdr(3, 8)).unwrap();
        assert_eq!(a.encoding, Encoding::LinearPcm16);
        assert_eq!(a.data_size, Some(8));
        assert_eq!(a.rate, 44100);
        assert_eq!(a.channels, 2);
        assert_eq!(a.data_at, 24);
        let a = parse(&hdr(1, u32::MAX)).unwrap();
        assert_eq!(a.encoding, Encoding::MuLaw8);
        assert!(a.data_size.is_none());
        let a = parse(&hdr(6, 8)).unwrap();
        assert_eq!(a.encoding, Encoding::Float32);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RIFF____________________").is_none());
        // offset before header end
        let mut h = hdr(3, 8);
        h[7] = 20;
        assert!(parse(&h).is_none());
        // declared size beyond file
        let h = hdr(3, 0x1000);
        assert!(parse(&h).is_none());
    }
}

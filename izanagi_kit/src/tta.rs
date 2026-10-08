//! True Audio (TTA1) header parsing.
//!
//! Header: `"TTA1"` + audio format u16 + channels u16 + bits u16 +
//! sample rate u32 + data length u32 + CRC32 u32.
//!
//! ```
//! use izanagi_kit::tta::parse;
//!
//! let mut f = vec![0u8; 22];
//! f[..4].copy_from_slice(b"TTA1");
//! f[4..6].copy_from_slice(&[1, 0]);         // format 1 = lossless
//! f[6..8].copy_from_slice(&[2, 0]);         // stereo
//! f[8..10].copy_from_slice(&[16, 0]);       // 16-bit
//! f[10..14].copy_from_slice(&[0x44, 0xAC, 0, 0]); // 44100
//! f[14..18].copy_from_slice(&[0x10, 0, 0, 0]);    // 16 samples
//! // crc field left 0 — parser stores it verbatim
//! let t = parse(&f).unwrap();
//! assert_eq!(t.channels, 2);
//! assert_eq!(t.bits_per_sample, 16);
//! ```

/// Parsed TTA1 header.
#[derive(Debug)]
pub struct Tta {
    /// Audio format (1 = uncompressed lossless).
    pub format: u16,
    /// Channel count.
    pub channels: u16,
    /// Bits per sample.
    pub bits_per_sample: u16,
    /// Sample rate in Hz.
    pub rate: u32,
    /// Total number of samples (per channel).
    pub data_len: u32,
    /// Header CRC-32 as stored.
    pub crc: u32,
}

/// Parses a TTA1 header.
pub fn parse(d: &[u8]) -> Option<Tta> {
    if d.len() < 22 || &d[..4] != b"TTA1" {
        return None;
    }
    let u16le = |o: usize| (d[o] as u16) | ((d[o + 1] as u16) << 8);
    let u32le = |o: usize| {
        (d[o] as u32)
            | ((d[o + 1] as u32) << 8)
            | ((d[o + 2] as u32) << 16)
            | ((d[o + 3] as u32) << 24)
    };
    let format = u16le(4);
    if format != 1 {
        return None;
    }
    let channels = u16le(6);
    let bits = u16le(8);
    if channels == 0 || bits == 0 || bits > 32 {
        return None;
    }
    Some(Tta {
        format,
        channels,
        bits_per_sample: bits,
        rate: u32le(10),
        data_len: u32le(14),
        crc: u32le(18),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut f = vec![0u8; 64];
        f[..4].copy_from_slice(b"TTA1");
        f[4..6].copy_from_slice(&[1, 0]);
        f[6..8].copy_from_slice(&[1, 0]);
        f[8..10].copy_from_slice(&[24, 0]);
        f[10..14].copy_from_slice(&[0x80, 0xBB, 0, 0]); // 48000
        let t = parse(&f).unwrap();
        assert_eq!(t.channels, 1);
        assert_eq!(t.bits_per_sample, 24);
        assert_eq!(t.rate, 48000);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut f = vec![0u8; 22];
        f[..4].copy_from_slice(b"TTA2");
        assert!(parse(&f).is_none());
        f[..4].copy_from_slice(b"TTA1");
        f[4] = 2; // format != 1
        assert!(parse(&f).is_none());
        f[4] = 1;
        f[8] = 40; // bits > 32
        assert!(parse(&f).is_none());
    }
}

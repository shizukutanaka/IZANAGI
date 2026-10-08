//! WavPack (.wv) block header parsing.
//!
//! Block header (32 B): `"wvpk"` + size u32 + version u16 + track/index u8
//! + total_samples u32 + block_index u32 + block_samples u32 + flags u32 + crc u32.
//!
//! ```
//! use izanagi_kit::wv::parse;
//!
//! let mut f = vec![0u8; 32];
//! f[..4].copy_from_slice(b"wvpk");
//! f[4..8].copy_from_slice(&[32, 0, 0, 0]);   // size = 32 (header only)
//! f[8..10].copy_from_slice(&[0x03, 0x04]);   // version 0x403
//! f[12..16].copy_from_slice(&[100, 0, 0, 0]);// total samples
//! f[20..24].copy_from_slice(&[4, 0, 0, 0]);  // block samples 4
//! let w = parse(&f).unwrap();
//! assert_eq!(w.version, 0x403);
//! assert_eq!(w.total_samples, 100);
//! ```

/// Parsed WavPack block header.
#[derive(Debug)]
pub struct Wv {
    /// Declared block byte size (header included); must not exceed input.
    pub block_size: u32,
    /// Format version (e.g. 0x403).
    pub version: u16,
    /// Track number (v4: always 0 — multi-channel handled via blocks).
    pub track: u8,
    /// Total samples in the file (`0xFFFFFFFF` = unknown).
    pub total_samples: u32,
    /// Index of this block's first sample.
    pub block_index: u32,
    /// Samples in this block.
    pub block_samples: u32,
    /// Flags word (mono/stereo, bits-per-sample code, hybrid...).
    pub flags: u32,
    /// Stored CRC of the block.
    pub crc: u32,
}

impl Wv {
    /// Bits per sample from the flag bits 0-1 (0=8, 1=16, 2=24, 3=32).
    pub fn bits_per_sample(&self) -> u16 {
        match self.flags & 3 {
            0 => 8,
            1 => 16,
            2 => 24,
            _ => 32,
        }
    }
    /// True when the mono flag is set (flag bit 2 clear = stereo default;
    /// in WavPack, bit 2 = `MONO_FLAG` actually *clear* means stereo).
    pub fn is_mono(&self) -> bool {
        self.flags & 4 != 0
    }
}

/// Parses a WavPack block header.
pub fn parse(d: &[u8]) -> Option<Wv> {
    if d.len() < 32 || &d[..4] != b"wvpk" {
        return None;
    }
    let u32le = |o: usize| {
        (d[o] as u32)
            | ((d[o + 1] as u32) << 8)
            | ((d[o + 2] as u32) << 16)
            | ((d[o + 3] as u32) << 24)
    };
    let block_size = u32le(4);
    if block_size < 32 || block_size as usize > d.len() {
        return None;
    }
    Some(Wv {
        block_size,
        version: (d[8] as u16) | ((d[9] as u16) << 8),
        track: d[10],
        total_samples: u32le(12),
        block_index: u32le(16),
        block_samples: u32le(20),
        flags: u32le(24),
        crc: u32le(28),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(size: u32) -> Vec<u8> {
        let mut v = vec![0u8; size.max(32) as usize];
        v[..4].copy_from_slice(b"wvpk");
        v[4..8].copy_from_slice(&[(size & 0xFF) as u8, (size >> 8) as u8, 0, 0]);
        v[8..10].copy_from_slice(&[0x03, 0x04]);
        v[24] = 1 | 4; // 16-bit, mono
        v
    }

    #[test]
    fn parses() {
        let w = parse(&block(64)).unwrap();
        assert_eq!(w.block_size, 64);
        assert_eq!(w.version, 0x403);
        assert_eq!(w.bits_per_sample(), 16);
        assert!(w.is_mono());
        let w = parse(&block(32)).unwrap();
        assert_eq!(w.block_samples, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(&block(31)).is_none()); // size < header
        let mut b = block(64);
        b[0] = b'x';
        assert!(parse(&b).is_none());
        let b = block(32); // declares 32 but only 32 bytes present is ok
        assert!(parse(&b).is_some());
        let mut b = block(32);
        b[4] = 0xFF; // huge declared size
        assert!(parse(&b).is_none());
    }
}

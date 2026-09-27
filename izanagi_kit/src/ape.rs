//! Monkey's Audio (.ape / MAC) header parsing.
//!
//! Descriptor (46 B): `"MAC "` + version u16 (>=3980 has descriptor) +
//! descriptor_bytes u16 + header_bytes u16 + seek_table_bytes u16 +
//! wav_header_bytes u16 + ape_frame_data_bytes u32 + ... + terminating u32.
//!
//! Then the APE header (24 B): compression u16, format flags u16,
//! blocks_per_frame u32, final_frame_blocks u32, total_frames u32,
//! bits u16, channels u16, sample_rate u32.
//!
//! ```
//! use izanagi_kit::ape::parse;
//!
//! let mut f = vec![0u8; 76];
//! f[..4].copy_from_slice(b"MAC ");
//! f[4..6].copy_from_slice(&[0x9E, 0x0F]); // version 3998
//! f[6..8].copy_from_slice(&[52, 0]);      // descriptor bytes
//! f[8..10].copy_from_slice(&[24, 0]);     // header bytes
//! let a = parse(&f).unwrap();
//! assert_eq!(a.version, 3998);
//! assert_eq!(a.header_at, 52);
//! ```

/// Parsed MAC descriptor + APE header fields.
pub struct Ape {
    /// Version number ×1000-ish (e.g. 3998 = 3.998).
    pub version: u16,
    /// Byte offset of the APE header (end of descriptor).
    pub header_at: usize,
    /// Declared header byte count.
    pub header_bytes: u16,
    /// Compression level code (1000/2000/3000/4000/5000).
    pub compression: u16,
    /// Total frames in the stream.
    pub total_frames: u32,
    /// Blocks per frame.
    pub blocks_per_frame: u32,
    /// Blocks in the final frame.
    pub final_blocks: u32,
    /// Bits per sample.
    pub bits_per_sample: u16,
    /// Channel count.
    pub channels: u16,
    /// Sample rate.
    pub rate: u32,
}

/// Parses a Monkey's Audio file header. Versions ≥ 3980 carry the
/// descriptor; `header_at` points past it.
pub fn parse(d: &[u8]) -> Option<Ape> {
    if d.len() < 52 || &d[..4] != b"MAC " {
        return None;
    }
    let u16le = |o: usize| (d[o] as u16) | ((d[o + 1] as u16) << 8);
    let version = u16le(4);
    if version < 3980 {
        return None;
    }
    let desc_bytes = u16le(6) as usize;
    if desc_bytes < 52 || desc_bytes + 24 > d.len() {
        return None;
    }
    let h = &d[desc_bytes..];
    Some(Ape {
        version,
        header_at: desc_bytes,
        header_bytes: u16le(8),
        compression: u16le_from(h, 0),
        total_frames: u32le_from(h, 12),
        blocks_per_frame: u32le_from(h, 4),
        final_blocks: u32le_from(h, 8),
        bits_per_sample: u16le_from(h, 16),
        channels: u16le_from(h, 18),
        rate: u32le_from(h, 20),
    })
}

fn u16le_from(d: &[u8], o: usize) -> u16 {
    (d[o] as u16) | ((d[o + 1] as u16) << 8)
}
fn u32le_from(d: &[u8], o: usize) -> u32 {
    (d[o] as u32) | ((d[o + 1] as u32) << 8) | ((d[o + 2] as u32) << 16) | ((d[o + 3] as u32) << 24)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> Vec<u8> {
        let mut f = vec![0u8; 52 + 24];
        f[..4].copy_from_slice(b"MAC ");
        f[4..6].copy_from_slice(&[0x9E, 0x0F]);
        f[6..8].copy_from_slice(&[52, 0]);
        f[8..10].copy_from_slice(&[24, 0]);
        // APE header at 52: comp=2000, flags=0, bpf=73728? use small
        f[52..54].copy_from_slice(&[0xD0, 0x07]); // 2000
        f[56..60].copy_from_slice(&[0, 9, 0, 0]); // blocks/frame 2304
        f[60..64].copy_from_slice(&[0x80, 2, 0, 0]); // final blocks 640
        f[64..68].copy_from_slice(&[0x20, 0, 0, 0]); // total frames 32
        f[68..70].copy_from_slice(&[16, 0]);
        f[70..72].copy_from_slice(&[2, 0]);
        f[72..76].copy_from_slice(&[0x44, 0xAC, 0, 0]);
        f
    }

    #[test]
    fn parses() {
        let a = parse(&file()).unwrap();
        assert_eq!(a.version, 3998);
        assert_eq!(a.header_at, 52);
        assert_eq!(a.compression, 2000);
        assert_eq!(a.blocks_per_frame, 2304);
        assert_eq!(a.final_blocks, 640);
        assert_eq!(a.total_frames, 32);
        assert_eq!(a.bits_per_sample, 16);
        assert_eq!(a.channels, 2);
        assert_eq!(a.rate, 44100);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0; 51]).is_none());
        let mut f = file();
        f[0] = b'x';
        assert!(parse(&f).is_none());
        let mut f = file();
        f[4] = 0x0C; // version 3979 < 3980
        assert!(parse(&f).is_none());
    }
}

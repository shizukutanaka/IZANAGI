//! DSF (DSD Stream File) header parsing.
//!
//! Layout: `DSD ` chunk (28 B: id, size u64, data-size u64, metadata u64)
//! then `fmt ` chunk (52 B), then `data` chunk.
//!
//! ```
//! use izanagi_kit::dsf::parse;
//!
//! let mut f = vec![0u8; 80];
//! f[..4].copy_from_slice(b"DSD ");
//! f[4..12].copy_from_slice(&[28, 0, 0, 0, 0, 0, 0, 0]);
//! f[12..20].copy_from_slice(&[80, 0, 0, 0, 0, 0, 0, 0]); // file size
//! f[28..32].copy_from_slice(b"fmt ");
//! f[32..40].copy_from_slice(&[52, 0, 0, 0, 0, 0, 0, 0]);
//! f[40..44].copy_from_slice(&[1, 0, 0, 0]); // format 1 = DSD raw
//! let d = parse(&f).unwrap();
//! assert_eq!(d.fmt_at, 28);
//! ```

/// Parsed DSF file header.
#[derive(Debug)]
pub struct Dsf {
    /// Declared total file size (0 when unset).
    pub file_size: u64,
    /// ID3 metadata offset (0 = none).
    pub meta_offset: u64,
    /// Offset of the `fmt ` chunk.
    pub fmt_at: usize,
    /// Format version (fmt chunk field).
    pub format_version: u32,
    /// Format id (1 = DSD raw).
    pub format_id: u32,
    /// Channel count.
    pub channels: u32,
    /// Sample rate (2_822_400 for DSD64).
    pub rate: u32,
    /// Bits per sample (1 or 8).
    pub bits_per_sample: u32,
    /// Total sample count.
    pub sample_count: u64,
    /// Offset of the `data` chunk if present in buffer.
    pub data_at: Option<usize>,
}

fn u64le(d: &[u8], o: usize) -> u64 {
    let mut v: u64 = 0;
    for i in 0..8 {
        v |= (d[o + i] as u64) << (i * 8);
    }
    v
}
fn u32le(d: &[u8], o: usize) -> u32 {
    (d[o] as u32) | ((d[o + 1] as u32) << 8) | ((d[o + 2] as u32) << 16) | ((d[o + 3] as u32) << 24)
}

/// Parses the DSD+fmt chunk header of a DSF file.
pub fn parse(d: &[u8]) -> Option<Dsf> {
    if d.len() < 80 || &d[..4] != b"DSD " {
        return None;
    }
    if u64le(d, 4) != 28 {
        return None;
    }
    let fmt_at = 28;
    if &d[fmt_at..fmt_at + 4] != b"fmt " || u64le(d, fmt_at + 4) != 52 {
        return None;
    }
    let fmt = &d[fmt_at + 12..];
    let data_at = if d.len() >= fmt_at + 52 + 12 && &d[fmt_at + 52..fmt_at + 56] == b"data" {
        Some(fmt_at + 52)
    } else {
        None
    };
    Some(Dsf {
        file_size: u64le(d, 12),
        meta_offset: u64le(d, 20),
        fmt_at,
        format_version: u32le(fmt, 0),
        format_id: u32le(fmt, 4),
        channels: u32le(fmt, 8),
        rate: u32le(fmt, 16),
        bits_per_sample: u32le(fmt, 20),
        sample_count: u64le(fmt, 24),
        data_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(with_data: bool) -> Vec<u8> {
        let mut f = vec![0u8; 80];
        f[..4].copy_from_slice(b"DSD ");
        f[4..12].copy_from_slice(&[28, 0, 0, 0, 0, 0, 0, 0]);
        f[12..20].copy_from_slice(&[80, 0, 0, 0, 0, 0, 0, 0]);
        f[28..32].copy_from_slice(b"fmt ");
        f[32..40].copy_from_slice(&[52, 0, 0, 0, 0, 0, 0, 0]);
        f[40..44].copy_from_slice(&[1, 0, 0, 0]);
        f[44..48].copy_from_slice(&[1, 0, 0, 0]);
        f[48..52].copy_from_slice(&[2, 0, 0, 0]);
        f[56..60].copy_from_slice(&[0x00, 0x11, 0x2B, 0x00]); // 2822400
        f[60..64].copy_from_slice(&[1, 0, 0, 0]);
        if with_data {
            f.extend_from_slice(b"data");
            f.extend_from_slice(&[0u8; 8]);
        }
        f
    }

    #[test]
    fn parses() {
        let d = parse(&file(false)).unwrap();
        assert_eq!(d.fmt_at, 28);
        assert_eq!(d.format_version, 1);
        assert_eq!(d.format_id, 1);
        assert_eq!(d.channels, 2);
        assert_eq!(d.rate, 2_822_400);
        assert_eq!(d.bits_per_sample, 1);
        assert!(d.data_at.is_none());
        let d = parse(&file(true)).unwrap();
        assert_eq!(d.data_at, Some(80));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0; 40]).is_none());
        let mut f = file(false);
        f[0] = b'X';
        assert!(parse(&f).is_none());
        let mut f = file(false);
        f[4] = 30;
        assert!(parse(&f).is_none());
        let mut f = file(false);
        f[28] = b'x';
        assert!(parse(&f).is_none());
    }
}

//! Sony Wave64 (w64) — 128-bit GUID chunk ids + `u64le` sizes.
//!
//! ```
//! let mut d = izanagi_kit::w64::RIFF_GUID.to_vec();
//! d.extend_from_slice(&[0x30, 0, 0, 0, 0, 0, 0, 0]);
//! d.extend_from_slice(&izanagi_kit::w64::WAVE_GUID);
//! let w = izanagi_kit::w64::parse(&d).unwrap();
//! assert_eq!(w.size, 0x30);
//! assert!(izanagi_kit::w64::detect(&d));
//! ```

/// `riff` container GUID (first 16 file bytes).
pub const RIFF_GUID: [u8; 16] = [
    b'r', b'i', b'f', b'f', 0x2e, 0x91, 0xcf, 0x11, 0xa5, 0xd6, 0x28, 0xdb, 0x04, 0xc1, 0x00, 0x00,
];
/// `wave` form-type GUID (at offset 24).
pub const WAVE_GUID: [u8; 16] = [
    b'w', b'a', b'v', b'e', 0xf3, 0xac, 0xd3, 0x11, 0x8c, 0xd1, 0x00, 0xc0, 0x4f, 0x8e, 0xdb, 0x8a,
];
/// `fmt ` chunk GUID.
pub const FMT_GUID: [u8; 16] = [
    b'f', b'm', b't', b' ', 0xf3, 0xac, 0xd3, 0x11, 0x8c, 0xd1, 0x00, 0xc0, 0x4f, 0x8e, 0xdb, 0x8a,
];
/// `data` chunk GUID.
pub const DATA_GUID: [u8; 16] = [
    b'd', b'a', b't', b'a', 0xf3, 0xac, 0xd3, 0x11, 0x8c, 0xd1, 0x00, 0xc0, 0x4f, 0x8e, 0xdb, 0x8a,
];

/// Parsed Wave64 summary.
#[derive(Debug, Clone)]
pub struct W64 {
    /// Declared total size from the riff header.
    pub size: u64,
    /// Short names (first 4 GUID bytes when printable) of chunks walked.
    pub chunks: Vec<String>,
    /// Whether a `fmt ` chunk was seen.
    pub has_fmt: bool,
    /// Whether a `data` chunk was seen.
    pub has_data: bool,
    /// Channels decoded from `fmt ` (u16le at +2).
    pub channels: u16,
    /// Sample rate decoded from `fmt ` (u32le at +4).
    pub sample_rate: u32,
}

fn le64(b: &[u8], o: usize) -> u64 {
    let mut v = 0u64;
    for i in 0..8 {
        v |= (b[o + i] as u64) << (8 * i);
    }
    v
}

/// Detects the `riff` GUID + `wave` GUID pair.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 40 && b[..16] == RIFF_GUID && b[24..40] == WAVE_GUID
}

/// Parses a Wave64 stream; `None` without the GUID pair.
#[must_use]
pub fn parse(b: &[u8]) -> Option<W64> {
    if !detect(b) {
        return None;
    }
    let mut f = W64 {
        size: le64(b, 16),
        chunks: Vec::new(),
        has_fmt: false,
        has_data: false,
        channels: 0,
        sample_rate: 0,
    };
    let mut i = 40usize;
    while i + 24 <= b.len() {
        let guid = &b[i..i + 16];
        if !guid[..4].iter().all(|&c| (0x20..0x7f).contains(&c)) {
            break;
        }
        f.chunks
            .push(String::from_utf8_lossy(&guid[..4]).into_owned());
        let len = le64(b, i + 16) as usize;
        if len < 24 || i + len > b.len() {
            break;
        }
        if guid == FMT_GUID {
            f.has_fmt = true;
            if len >= 32 {
                f.channels = (b[i + 26] as u16) | ((b[i + 27] as u16) << 8);
                f.sample_rate = (b[i + 28] as u32)
                    | ((b[i + 29] as u32) << 8)
                    | ((b[i + 30] as u32) << 16)
                    | ((b[i + 31] as u32) << 24);
            }
        }
        if guid == DATA_GUID {
            f.has_data = true;
        }
        i += (len + 7) & !7;
    }
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = RIFF_GUID.to_vec();
        d.extend_from_slice(&[0x68, 0, 0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&WAVE_GUID);
        d.extend_from_slice(&FMT_GUID);
        d.extend_from_slice(&[0x28, 0, 0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&[0x01, 0x00, 0x02, 0x00, 0x44, 0xac, 0x00, 0x00]);
        d.extend_from_slice(&[0u8; 8]);
        d.extend_from_slice(&DATA_GUID);
        d.extend_from_slice(&[0x20, 0, 0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&[0u8; 8]);
        d
    }

    #[test]
    fn parses() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.size, 0x68);
        assert_eq!(f.chunks, vec!["fmt ", "data"]);
        assert!(f.has_fmt && f.has_data);
        assert_eq!(f.channels, 2);
        assert_eq!(f.sample_rate, 44100);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(!detect(&[0u8; 40]));
        let mut d = fixture();
        d[24] = b'X';
        assert!(parse(&d).is_none());
    }
}

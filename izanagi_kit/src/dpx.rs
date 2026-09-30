//! DPX (SMPTE Digital Picture Exchange) image scanner.
//!
//! A DPX file starts with the magic `SDPX` (big-endian layout) or
//! `XPDS` (little-endian), followed in the file's endianness by a
//! `u32` image-data offset, an 8-byte version string (`V2.0`,
//! `V1.0`), and the `u32` total file size.
//!
//! ```
//! let mut d = b"SDPX".to_vec();
//! d.extend_from_slice(&[0, 0, 0x20, 0x00]); // image offset 8192, BE
//! d.extend_from_slice(b"V2\x2e0    ");
//! d.extend_from_slice(&[0, 0, 0x30, 0x00]); // file size, BE
//! d.resize(64, 0);
//! let f = izanagi_kit::dpx::parse(&d).unwrap();
//! assert!(f.big_endian);
//! assert_eq!(f.image_offset, 0x2000);
//! ```
//!
//! Reference: SMPTE ST 268 (DPX) — the `SDPX`/`XPDS` magic that
//! also encodes the file's byte order and the version/file-size
//! words that follow it.

/// Parsed DPX header fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Dpx {
    /// `true` when the file stores multi-byte fields big-endian
    /// (`SDPX` magic); `false` for little-endian (`XPDS`).
    pub big_endian: bool,
    /// Byte offset of the image data block.
    pub image_offset: u32,
    /// Declared file size.
    pub file_size: u32,
    /// Version string, e.g. `V2.0`.
    pub version: Option<String>,
    /// Generic / industry / user header sizes from the
    /// file-information block, when readable (offsets 24/28/32).
    pub header_sizes: Option<(u32, u32, u32)>,
}

fn read_u32(d: &[u8], i: usize, be: bool) -> u32 {
    if be {
        ((d[i] as u32) << 24)
            | ((d[i + 1] as u32) << 16)
            | ((d[i + 2] as u32) << 8)
            | d[i + 3] as u32
    } else {
        (d[i] as u32)
            | ((d[i + 1] as u32) << 8)
            | ((d[i + 2] as u32) << 16)
            | ((d[i + 3] as u32) << 24)
    }
}

/// Parse a DPX header; `None` unless the magic, offset, version,
/// and size fields are readable.
pub fn parse(d: &[u8]) -> Option<Dpx> {
    if d.len() < 28 {
        return None;
    }
    let be = match &d[..4] {
        b"SDPX" => true,
        b"XPDS" => false,
        _ => return None,
    };
    let image_offset = read_u32(d, 4, be);
    let version = {
        let raw = &d[8..16];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(8);
        let s = core::str::from_utf8(&raw[..end])
            .ok()
            .map(str::trim)
            .unwrap_or("");
        if s.starts_with('V') {
            Some(s.to_string())
        } else {
            None
        }
    };
    let file_size = read_u32(d, 16, be);
    let header_sizes = if d.len() >= 36 {
        Some((
            read_u32(d, 24, be),
            read_u32(d, 28, be),
            read_u32(d, 32, be),
        ))
    } else {
        None
    };
    Some(Dpx {
        big_endian: be,
        image_offset,
        file_size,
        version,
        header_sizes,
    })
}

/// `true` if the buffer looks like a DPX image.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w32(v: u32, be: bool) -> [u8; 4] {
        if be {
            [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]
        } else {
            [v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8]
        }
    }

    fn doc(be: bool) -> Vec<u8> {
        let mut d = if be {
            b"SDPX".to_vec()
        } else {
            b"XPDS".to_vec()
        };
        d.extend_from_slice(&w32(0x2000, be));
        d.extend_from_slice(b"V2\x2e0    ");
        d.extend_from_slice(&w32(0x3000, be)); // file size
        d.extend_from_slice(&w32(0, be)); // ditto key
        d.extend_from_slice(&w32(0x600, be)); // generic hdr size
        d.extend_from_slice(&w32(0x100, be)); // industry hdr size
        d.extend_from_slice(&w32(0, be)); // user hdr size
        d.resize(64, 0);
        d
    }

    #[test]
    fn parses_be() {
        let f = parse(&doc(true)).unwrap();
        assert!(f.big_endian);
        assert_eq!(f.image_offset, 0x2000);
        assert_eq!(f.version.as_deref(), Some("V2\x2e0"));
        assert_eq!(f.file_size, 0x3000);
        assert_eq!(f.header_sizes, Some((0x600, 0x100, 0)));
    }

    #[test]
    fn parses_le() {
        let f = parse(&doc(false)).unwrap();
        assert!(!f.big_endian);
        assert_eq!(f.image_offset, 0x2000);
        assert_eq!(f.file_size, 0x3000);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"SDPX").is_none()); // too short
        assert!(parse(b"XPXDSXXX").is_none());
        assert!(parse(&[0xff; 32]).is_none());
    }

    #[test]
    fn detects() {
        assert!(detect(&doc(true)));
        assert!(detect(&doc(false)));
        assert!(!detect(b"DPX!"));
    }
}

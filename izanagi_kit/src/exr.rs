//! OpenEXR image file scanner.
//!
//! An EXR file starts with the LE magic `0x01312F76` (bytes
//! `76 2F 31 01`) and a LE `u32` version+flags word: low byte is
//! the version (2), then bit 9 = tiled, bit 10 = long names,
//! bit 11 = non-image (deep), bit 12 = multipart. The header is a
//! `name\0 type\0 size\le value` attribute list terminated by a
//! zero byte.
//!
//! ```
//! let mut d = vec![0x76, 0x2F, 0x31, 0x01, 0x02, 0, 0, 0];
//! d.extend_from_slice(b"channels\0chlist\0");
//! d.extend_from_slice(&[1, 0, 0, 0, 0]); // size 1 + payload + terminator
//! d.push(0);
//! let e = izanagi_kit::exr::parse(&d).unwrap();
//! assert_eq!(e.version, 2);
//! assert_eq!(e.attributes, 1);
//! ```
//!
//! Reference: OpenEXR file layout (Academy Software Foundation /
//! ILM Technical Document) — magic `20000630` and the version-flag
//! word bit assignments.

/// Parsed EXR header fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Exr {
    /// File format version (normally 2).
    pub version: u8,
    /// Tiled single-image flag (version word bit 9).
    pub tiled: bool,
    /// Long attribute names flag (bit 10).
    pub long_names: bool,
    /// Non-image / deep data flag (bit 11).
    pub non_image: bool,
    /// Multipart flag (bit 12).
    pub multipart: bool,
    /// Header attribute count.
    pub attributes: usize,
    /// First few attribute names.
    pub names: Vec<String>,
}

fn u32le(d: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

/// Parse an EXR header; `None` unless the magic, version, and a
/// well-formed NUL-terminated attribute list are present.
pub fn parse(d: &[u8]) -> Option<Exr> {
    if d.len() < 9 || d[..4] != [0x76, 0x2F, 0x31, 0x01] {
        return None;
    }
    let vf = u32le(d, 4);
    let version = (vf & 0xFF) as u8;
    if version == 0 {
        return None;
    }
    let mut attributes = 0usize;
    let mut names = Vec::new();
    let mut i = 8usize;
    loop {
        if i >= d.len() {
            return None;
        }
        if d[i] == 0 {
            break;
        }
        let name_end = i + d[i..].iter().position(|&b| b == 0)?;
        let name = core::str::from_utf8(&d[i..name_end]).ok()?.to_string();
        let type_start = name_end + 1;
        let type_end = type_start + d[type_start..].iter().position(|&b| b == 0)?;
        if type_end + 4 > d.len() {
            return None;
        }
        let size = u32le(d, type_end + 1) as usize;
        i = type_end + 5 + size;
        if i > d.len() {
            return None;
        }
        attributes += 1;
        if names.len() < 8 {
            names.push(name);
        }
    }
    Some(Exr {
        version,
        tiled: vf & 0x200 != 0,
        long_names: vf & 0x400 != 0,
        non_image: vf & 0x800 != 0,
        multipart: vf & 0x1000 != 0,
        attributes,
        names,
    })
}

/// `true` if the buffer looks like an EXR file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Vec<u8> {
        let mut d = vec![0x76, 0x2F, 0x31, 0x01];
        d.extend_from_slice(&[0x02, 0x02, 0, 0]); // v2 + tiled
        d.extend_from_slice(b"channels\0chlist\0");
        d.extend_from_slice(&[1, 0, 0, 0]);
        d.push(0);
        d.extend_from_slice(b"dataWindow\0box2i\0");
        d.extend_from_slice(&[16, 0, 0, 0]);
        d.extend_from_slice(&[0; 16]);
        d.push(0);
        d
    }

    #[test]
    fn parses() {
        let e = parse(&doc()).unwrap();
        assert_eq!(e.version, 2);
        assert!(e.tiled);
        assert!(!e.multipart);
        assert_eq!(e.attributes, 2);
        assert_eq!(e.names[0], "channels");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x76, 0x2F, 0x31, 0x01]).is_none()); // no terminator
        assert!(parse(&[0x76, 0x2F, 0x31, 0x01, 0, 0, 0, 0, 0]).is_none()); // version 0
        assert!(parse(&[0xff; 16]).is_none());
    }

    #[test]
    fn detects() {
        assert!(detect(&doc()));
        assert!(!detect(b"EXR"));
    }
}

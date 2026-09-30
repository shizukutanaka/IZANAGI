//! JPEG XR (HD Photo) image scanner.
//!
//! A JXR file starts with the little-endian signature
//! `II \xBC \x01` followed by a LE `u32` offset of the first IFD
//! (normally 8). The IFD is a `u16` tag count plus `N×12`-byte
//! tag records (`tag`,`type`,`count`,`value/offset`) and a `u32`
//! next-IFD offset.
//!
//! ```
//! let mut d = b"II\xBC\x01".to_vec();
//! d.extend_from_slice(&[8, 0, 0, 0]);   // ifd at 8
//! d.extend_from_slice(&[1, 0]);         // 1 tag
//! d.extend_from_slice(&[0x01, 0xBC, 1, 0]); // tag 0xBC01, type 1
//! d.extend_from_slice(&[4, 0, 0, 0, 0, 0, 0, 0]);
//! d.extend_from_slice(&[0, 0, 0, 0]);   // next ifd = 0
//! let j = izanagi_kit::jxr::parse(&d).unwrap();
//! assert_eq!(j.tags, 1);
//! ```
//!
//! Reference: JPEG XR / ITU-T T.832 — `II 0xBC 0x01` signature and
//! the TIFF-style IFD directory layout.

/// Parsed JXR fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Jxr {
    /// Offset of the first IFD.
    pub ifd_offset: u32,
    /// Tag count in the first IFD.
    pub tags: u16,
    /// Next-IFD offset (0 when single).
    pub next_ifd: u32,
    /// Tag ids present in the first IFD (capped at 32).
    pub tag_ids: Vec<u16>,
}

fn u16le(d: &[u8], i: usize) -> u16 {
    (d[i] as u16) | ((d[i + 1] as u16) << 8)
}

fn u32le(d: &[u8], i: usize) -> u32 {
    (d[i] as u32) | ((d[i + 1] as u32) << 8) | ((d[i + 2] as u32) << 16) | ((d[i + 3] as u32) << 24)
}

/// Parse a JXR header; `None` unless the `II \xBC \x01` signature
/// and a readable first IFD are present.
pub fn parse(d: &[u8]) -> Option<Jxr> {
    if d.len() < 8 || d[..4] != [0x49, 0x49, 0xBC, 0x01] {
        return None;
    }
    let ifd_offset = u32le(d, 4);
    let i = ifd_offset as usize;
    if i + 2 > d.len() {
        return None;
    }
    let tags = u16le(d, i);
    let table_end = i + 2 + tags as usize * 12;
    if table_end + 4 > d.len() {
        return None;
    }
    let mut tag_ids = Vec::new();
    for t in 0..tags as usize {
        if tag_ids.len() < 32 {
            tag_ids.push(u16le(d, i + 2 + t * 12));
        }
    }
    Some(Jxr {
        ifd_offset,
        tags,
        next_ifd: u32le(d, table_end),
        tag_ids,
    })
}

/// `true` if the buffer looks like a JXR image.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Vec<u8> {
        let mut d = b"II\xBC\x01".to_vec();
        d.extend_from_slice(&[8, 0, 0, 0]);
        d.extend_from_slice(&[2, 0]);
        for tag in [0xBC01u16, 0xBC02] {
            d.extend_from_slice(&tag.to_le_bytes());
            d.extend_from_slice(&[1, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
        }
        d.extend_from_slice(&[0, 0, 0, 0]);
        d
    }

    #[test]
    fn parses() {
        let j = parse(&doc()).unwrap();
        assert_eq!(j.ifd_offset, 8);
        assert_eq!(j.tags, 2);
        assert_eq!(j.tag_ids, [0xBC01, 0xBC02]);
        assert_eq!(j.next_ifd, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"II\xBC\x02\x08\0\0\0").is_none());
        assert!(parse(b"II\xBC\x01\x08\0\0\0\xff").is_none()); // ifd past end
    }

    #[test]
    fn detects() {
        assert!(detect(&doc()));
        assert!(!detect(b"II"));
    }
}

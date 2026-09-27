//! Zstandard — the RFC 8878 frame header. A frame opens with the u32
//! magic `0xFD2F_B528` (little-endian bytes `28 B5 2F FD`), then a
//! Frame_Header_Descriptor byte: bits 7–6 pick the frame-content-size
//! field width (0/1→1B or 2B, 2→4B, 3→8B), bit 5 is
//! `single_segment` (no window descriptor follows), bit 2 is the
//! content-checksum flag, and bits 1–0 pick the dictionary-ID width.
//! Skippable frames use magics `0x184D2A50`..=`0x184D2A5F` followed by
//! a u32 payload length.
//!
//! ```
//! use izanagi_kit::zstd::{parse, header_bytes};
//! let mut d = vec![0x28, 0xB5, 0x2F, 0xFD];
//! d.push(0x80);            // FCS flag 10b (4 bytes), not single-segment
//! d.push(0x01);            // window descriptor
//! d.extend_from_slice(&64u32.to_le_bytes()); // content size
//! let z = parse(&d).unwrap();
//! assert_eq!(z.fcs_bytes, 4);
//! assert_eq!(header_bytes(&z), 4 + 1 + 1 + 4);
//! ```

/// Little-endian frame magic.
pub const MAGIC: u32 = 0xFD2F_B528;
/// First skippable-frame magic (`0x184D2A50`..`0x184D2A5F`).
pub const SKIPPABLE_MIN: u32 = 0x184D_2A50;
/// Last skippable-frame magic.
pub const SKIPPABLE_MAX: u32 = 0x184D_2A5F;

/// A parsed frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zstd {
    /// True for a skippable frame (`fcs_bytes`/`dict_id_bytes` are 0).
    pub skippable: bool,
    /// `single_segment` flag.
    pub single_segment: bool,
    /// Content-checksum flag.
    pub checksum: bool,
    /// Dictionary-ID field width in bytes (0/1/2/4).
    pub dict_id_bytes: u8,
    /// Frame-content-size field width in bytes (0/1/2/4/8).
    pub fcs_bytes: u8,
    /// Window descriptor byte when present.
    pub window: Option<u8>,
    /// Declared content size when the field is present.
    pub content_size: Option<u64>,
    /// For skippable frames: declared payload length.
    pub payload_len: Option<u32>,
}

fn u64l(d: &[u8], at: usize, n: usize) -> Option<u64> {
    let mut v = 0u64;
    for (i, &c) in d.get(at..at + n)?.iter().enumerate() {
        v |= u64::from(c) << (i * 8);
    }
    Some(v)
}

/// Parse the frame header; `None` without either magic.
pub fn parse(d: &[u8]) -> Option<Zstd> {
    let magic = u32::from_le_bytes(d.get(..4)?.try_into().ok()?);
    if (SKIPPABLE_MIN..=SKIPPABLE_MAX).contains(&magic) {
        return Some(Zstd {
            skippable: true,
            single_segment: false,
            checksum: false,
            dict_id_bytes: 0,
            fcs_bytes: 0,
            window: None,
            content_size: None,
            payload_len: Some(u32::from_le_bytes(d.get(4..8)?.try_into().ok()?)),
        });
    }
    if magic != MAGIC {
        return None;
    }
    let dscr = *d.get(4)?;
    let fcs_flag = dscr >> 6;
    let single_segment = dscr & 0x20 != 0;
    let checksum = dscr & 0x04 != 0;
    let dict_id_bytes = [0u8, 1, 2, 4][usize::from(dscr & 0x03)];
    let fcs_bytes = match fcs_flag {
        0 if single_segment => 1,
        0 => 0,
        1 => 2,
        2 => 4,
        _ => 8,
    };
    let mut at = 5usize;
    let window = if single_segment {
        None
    } else {
        let w = *d.get(at)?;
        at += 1;
        Some(w)
    };
    at = at.checked_add(usize::from(dict_id_bytes))?;
    let content_size = if fcs_bytes > 0 {
        Some(u64l(d, at, usize::from(fcs_bytes))?)
    } else {
        None
    };
    at = at.checked_add(usize::from(fcs_bytes))?;
    d.get(..at)?; // full header must be present
    Some(Zstd {
        skippable: false,
        single_segment,
        checksum,
        dict_id_bytes,
        fcs_bytes,
        window,
        content_size,
        payload_len: None,
    })
}

/// Total bytes the header occupies (magic through content size).
pub fn header_bytes(z: &Zstd) -> usize {
    if z.skippable {
        return 8;
    }
    4 + 1
        + usize::from(z.window.is_some())
        + usize::from(z.dict_id_bytes)
        + usize::from(z.fcs_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_le_bytes().to_vec();
        d.push(0x80); // fcs flag 2 (4B), not single segment
        d.push(0x7F); // window descriptor
        d.extend_from_slice(&64u32.to_le_bytes());
        d.extend_from_slice(b"body");
        d
    }

    #[test]
    fn parses_header() {
        let d = fixture();
        let z = parse(&d).unwrap();
        assert!(!z.skippable && !z.single_segment && !z.checksum);
        assert_eq!(z.fcs_bytes, 4);
        assert_eq!(z.dict_id_bytes, 0);
        assert_eq!(z.window, Some(0x7F));
        assert_eq!(z.content_size, Some(64));
        assert_eq!(header_bytes(&z), 10);
    }

    #[test]
    fn single_segment_and_skippable() {
        // single segment → FCS always present (1B for flag 0)
        let mut d = MAGIC.to_le_bytes().to_vec();
        d.push(0x20);
        d.push(200); // content size
        let z = parse(&d).unwrap();
        assert!(z.single_segment);
        assert_eq!(z.fcs_bytes, 1);
        assert_eq!(z.content_size, Some(200));
        assert_eq!(header_bytes(&z), 6);
        // skippable
        let mut s = SKIPPABLE_MIN.to_le_bytes().to_vec();
        s.extend_from_slice(&7u32.to_le_bytes());
        s.extend_from_slice(&[0; 7]);
        let zs = parse(&s).unwrap();
        assert!(zs.skippable);
        assert_eq!(zs.payload_len, Some(7));
        assert_eq!(header_bytes(&zs), 8);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x28, 0xB5, 0x2F, 0xFE]).is_none());
        // header declared past EOF
        let mut d = MAGIC.to_le_bytes().to_vec();
        d.push(0xC0); // fcs flag 3 → needs 8 more bytes
        assert!(parse(&d).is_none());
    }
}

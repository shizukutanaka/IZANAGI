//! Olympus ORF (Olympus RAW Format) header census.
//!
//! ORF is little-endian TIFF with an Olympus signature at offset 8:
//! `IIRO` (most models) or `IIRS` (C-7070/E-1 generation). Bytes 12..16
//! hold `0x0008` and IFD0 starts at offset 16.
//!
//! ```text
//! 00 "II*\x00"  04 u32le = 8  08 "IIRO"|"IIRS"  0C u32le = 8  10 IFD0
//! ```
//!
//! ```
//! let mut d = b"II*\x00\x08\x00\x00\x00IIRO\x08\x00\x00\x00".to_vec();
//! d.extend([1, 0, 0x0F, 0x01, 2, 0, 8, 0, 0, 0, 0x40, 0, 0, 0, 0, 0, 0, 0]);
//! let o = izanagi_kit::orf::parse(&d).unwrap();
//! assert_eq!(o.signature, *b"IIRO");
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orf {
    /// Signature at offset 8: `IIRO` or `IIRS`.
    pub signature: [u8; 4],
    /// `IIRS` variant (older C-7070/E-1 style) vs `IIRO`.
    pub iirs: bool,
    /// `u32le` at offset 12 (always 8 — IFD0-relative marker).
    pub marker: u32,
    /// IFD0 entry count at offset 16 when in bounds.
    pub entries: u16,
    /// `MakerNote` (0x927C) tag seen in IFD0.
    pub maker_tag: bool,
}

fn u16le(b: &[u8], i: usize) -> u16 {
    (b[i] as u16) | ((b[i + 1] as u16) << 8)
}

fn u32le(b: &[u8], i: usize) -> u32 {
    (b[i] as u32) | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on `II*\x00` + `IIRO`/`IIRS` at offset 8.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 16 && &b[0..4] == b"II*\x00" && (&b[8..12] == b"IIRO" || &b[8..12] == b"IIRS")
}

/// Census; `None` on non-ORF.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Orf> {
    if !detect(b) {
        return None;
    }
    let mut signature = [0u8; 4];
    signature.copy_from_slice(&b[8..12]);
    let entries = if b.len() >= 18 { u16le(b, 16) } else { 0 };
    let mut maker_tag = false;
    for i in 0..usize::from(entries).min(512) {
        let e = 18 + i * 12;
        if e + 2 > b.len() {
            break;
        }
        if u16le(b, e) == 0x927C {
            maker_tag = true;
        }
    }
    Some(Orf {
        signature,
        iirs: &b[8..12] == b"IIRS",
        marker: u32le(b, 12),
        entries,
        maker_tag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"II*\x00\x08\x00\x00\x00IIRO\x08\x00\x00\x00".to_vec();
        d.extend([
            1, 0, 0x7C, 0x92, 7, 0, 4, 0, 0, 0, 0x60, 0, 0, 0, 0, 0, 0, 0,
        ]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(detect(b"II*\x00\x08\x00\x00\x00IIRS\x08\x00\x00\x00"));
        assert!(!detect(
            b"II*\x00\x08\x00\x00\x00CR\x02\x00\x00\x10\x00\x00"
        ));
    }

    #[test]
    fn parses() {
        let o = parse(&fixture()).unwrap();
        assert_eq!(o.signature, *b"IIRO");
        assert!(!o.iirs);
        assert_eq!(o.marker, 8);
        assert_eq!(o.entries, 1);
        assert!(o.maker_tag);
    }

    #[test]
    fn iirs_variant() {
        let o = parse(b"II*\x00\x08\x00\x00\x00IIRS\x08\x00\x00\x00\x00\x00").unwrap();
        assert!(o.iirs);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"IIU\x00\x08\x00\x00\x00").is_none());
    }
}

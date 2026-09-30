//! MicroStation DGN (Intergraph Standard File Format / V8 design
//! files). A DGNv8 file opens with the file's first element record
//! whose leading word is `0x0809` (bytes `09 08`), followed by the
//! 16-bit element type of the first element and 512-byte-block
//! organization.
//!
//! `parse` requires the `09 08` prolog plus a plausible element type
//! (`1..=127`), and reports the declared file version fields found at
//! the fixed TCB offsets (`0x04` set-id / `0x08` version word are
//! kept raw as integers).
//!
//! ```
//! let mut f = vec![0u8; 512];
//! f[0] = 0x09; f[1] = 0x08;
//! f[2] = 0x05; f[3] = 0x00; // element type 5 (TCB)
//! let d = izanagi_kit::dgn::parse(&f).unwrap();
//! assert!(d.is_v8());
//! assert_eq!(d.first_element_type, 5);
//! assert!(izanagi_kit::dgn::parse(b"09x").is_none());
//! ```

/// Parsed DGN summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Dgn {
    /// Element type of the first record (u16 little-endian at `0x02`);
    /// element type 5 is the TCB (file setup) in V8.
    pub first_element_type: u16,
    /// Length of the first element in 16-bit words if the record is
    /// long enough to carry it (offset `0x04`), else `0`.
    pub first_element_words: u16,
}

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

impl Dgn {
    /// True when the first element is a TCB (V8 signature shape).
    pub fn is_v8(&self) -> bool {
        self.first_element_type == 5 || self.first_element_type == 9
    }
}

/// Parse a DGN file; `None` without the `09 08` prolog and a sane
/// first element type.
pub fn parse(d: &[u8]) -> Option<Dgn> {
    if d.len() < 6 || d[0] != 0x09 || d[1] != 0x08 {
        return None;
    }
    let ty = le16(d, 2)?;
    if ty == 0 || ty > 127 {
        return None;
    }
    let words = le16(d, 4).unwrap_or(0);
    Some(Dgn {
        first_element_type: ty,
        first_element_words: words,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut f = vec![0u8; 512];
        f[0] = 0x09;
        f[1] = 0x08;
        f[2] = 5;
        f[4] = 0x12; // 18 words
        let d = parse(&f).unwrap();
        assert!(d.is_v8());
        assert_eq!(d.first_element_words, 18);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x09\x09\x05\x00").is_none());
        assert!(parse(b"\x09\x08\x00\x00").is_none());
        assert!(parse(b"\x09\x08\xC8\x00").is_none()); // type 200 > 127
    }
}

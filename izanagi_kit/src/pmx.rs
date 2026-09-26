//! PMX — the successor of `PMD`. Byte 0 carries `PMX ` (note the
//! space), a four-byte IEEE-754 version word kept as raw `u32`
//! (`2.0` = `0x4000_0000`, `2.1` = `0x4000_0001`), a u8 header size
//! (always 8) and the 8-byte settings header: byte 0 selects the
//! text encoding (0 = UTF-16LE, 1 = UTF-8) and bytes 2..8 are the
//! index widths for vertices, textures, materials, bones, morphs and
//! rigid bodies (1, 2 or 4).
//!
//! ```
//! use izanagi_kit::pmx::{parse, VERSION_2_0};
//! let mut d = b"PMX ".to_vec();
//! d.extend_from_slice(&VERSION_2_0.to_le_bytes());
//! d.push(8);
//! d.extend_from_slice(&[0, 0, 4, 1, 1, 1, 1, 1]);
//! let p = parse(&d).unwrap();
//! assert_eq!(p.version, VERSION_2_0);
//! assert_eq!(p.index_size(2), Some(4)); // vertex index width
//! ```

/// `f32` 2.0 as raw little-endian bits.
pub const VERSION_2_0: u32 = 0x4000_0000;
/// `f32` 2.1 as raw little-endian bits (the `+1` bump is real).
pub const VERSION_2_1: u32 = 0x4000_0001;
/// The fixed settings-header width.
pub const HEADER_SIZE: u8 = 8;

/// A parsed PMX head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pmx {
    /// Raw version bits (`VERSION_2_0` or `VERSION_2_1`).
    pub version: u32,
    /// The eight settings bytes verbatim.
    pub header: [u8; HEADER_SIZE as usize],
}

impl Pmx {
    /// True when strings are UTF-8 (byte 0 == 1); UTF-16LE otherwise.
    pub fn utf8(&self) -> bool {
        self.header[0] == 1
    }

    /// Index width for slot `i` (2 = vertex, 3 = texture, 4 = material,
    /// 5 = bone, 6 = morph, 7 = rigid body); `None` out of range or a
    /// width other than 1/2/4.
    pub fn index_size(&self, i: usize) -> Option<u8> {
        match self.header.get(i).copied()? {
            w @ (1 | 2 | 4) => Some(w),
            _ => None,
        }
    }
}

/// Parse the head; `None` without `PMX ` or a short header.
pub fn parse(d: &[u8]) -> Option<Pmx> {
    if d.get(..4)? != b"PMX " {
        return None;
    }
    let version = u32::from_le_bytes(d.get(4..8)?.try_into().ok()?);
    let size = *d.get(8)?;
    let header: [u8; HEADER_SIZE as usize] = d
        .get(9..9 + usize::from(size.min(HEADER_SIZE)))?
        .try_into()
        .ok()?;
    if size < HEADER_SIZE {
        return None;
    }
    Some(Pmx { version, header })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"PMX ".to_vec();
        d.extend_from_slice(&VERSION_2_1.to_le_bytes());
        d.push(HEADER_SIZE);
        d.extend_from_slice(&[1, 0, 4, 2, 1, 1, 1, 1]);
        d
    }

    #[test]
    fn parses_head() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, VERSION_2_1);
        assert!(p.utf8());
        assert_eq!(p.index_size(2), Some(4));
        assert_eq!(p.index_size(3), Some(2));
        assert_eq!(p.index_size(9), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        // short settings header
        let mut d2 = fixture();
        d2[8] = 4;
        assert!(parse(&d2).is_none());
    }
}

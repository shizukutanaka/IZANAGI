//! PMD — the MikuMikuDance polygon model. Byte 0 carries `Pmd` and a
//! four-byte IEEE-754 version word (kept as its raw `u32` — `1.0` is
//! `0x3F80_0000`); a 20-byte Shift-JIS model name and a 256-byte
//! comment follow, then a u32LE vertex count.
//!
//! ```
//! use izanagi_kit::pmd::{parse, VERSION_1_0};
//! let mut d = b"Pmd".to_vec();
//! d.extend_from_slice(&VERSION_1_0.to_le_bytes());
//! let mut name = b"Model".to_vec();
//! name.resize(20, 0);
//! d.extend_from_slice(&name);
//! d.extend(std::iter::repeat(0).take(256));
//! d.extend_from_slice(&42u32.to_le_bytes());
//! let p = parse(&d).unwrap();
//! assert_eq!(p.version, VERSION_1_0);
//! assert_eq!(p.name, b"Model");
//! assert_eq!(p.vertices, 42);
//! ```

/// `f32` 1.0 as raw little-endian bits — the only version PMD defines.
pub const VERSION_1_0: u32 = 0x3F80_0000;
/// Model name field width.
pub const NAME_LEN: usize = 20;
/// Comment field width.
pub const COMMENT_LEN: usize = 256;

/// A parsed PMD head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pmd {
    /// Raw version bits (`VERSION_1_0` for every PMD in the wild).
    pub version: u32,
    /// Shift-JIS model name, trimmed at the first NUL.
    pub name: Vec<u8>,
    /// Comment field, trimmed at the first NUL.
    pub comment: Vec<u8>,
    /// Declared vertex count.
    pub vertices: u32,
}

fn trim(field: &[u8]) -> Vec<u8> {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    field[..end].to_vec()
}

/// Parse the fixed head; `None` without `Pmd` or on a short buffer.
pub fn parse(d: &[u8]) -> Option<Pmd> {
    if d.get(..3)? != b"Pmd" {
        return None;
    }
    let version = u32::from_le_bytes(d.get(3..7)?.try_into().ok()?);
    let name = trim(d.get(7..7 + NAME_LEN)?);
    let comment_at = 7 + NAME_LEN;
    let comment = trim(d.get(comment_at..comment_at + COMMENT_LEN)?);
    let v_at = comment_at + COMMENT_LEN;
    let vertices = u32::from_le_bytes(d.get(v_at..v_at + 4)?.try_into().ok()?);
    Some(Pmd {
        version,
        name,
        comment,
        vertices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"Pmd".to_vec();
        d.extend_from_slice(&VERSION_1_0.to_le_bytes());
        let mut name = b"Miku".to_vec();
        name.resize(NAME_LEN, 0);
        d.extend_from_slice(&name);
        let mut c = b"test model".to_vec();
        c.resize(COMMENT_LEN, 0);
        d.extend_from_slice(&c);
        d.extend_from_slice(&7u32.to_le_bytes());
        d
    }

    #[test]
    fn parses_head() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, VERSION_1_0);
        assert_eq!(p.name, b"Miku");
        assert_eq!(p.comment, b"test model");
        assert_eq!(p.vertices, 7);
    }

    #[test]
    fn full_fields_no_nul() {
        let mut d = b"Pmd".to_vec();
        d.extend_from_slice(&VERSION_1_0.to_le_bytes());
        d.extend(std::iter::repeat(b'a').take(NAME_LEN));
        d.extend(std::iter::repeat(b'b').take(COMMENT_LEN));
        d.extend_from_slice(&0u32.to_le_bytes());
        let p = parse(&d).unwrap();
        assert_eq!(p.name.len(), NAME_LEN);
        assert_eq!(p.comment.len(), COMMENT_LEN);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        assert!(parse(&d[..100]).is_none());
    }
}

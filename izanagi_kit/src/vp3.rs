//! Pfaff/Viking VP3 embroidery file: starts with the signature
//! `%vsm%` + `\x00`, a `version` u16 BE, then a `\x00`-padded block,
//! the `produced by` NUL-terminated vendor string and section markers
//! (`%header%`, `%comments%`, …).
//!
//! ```
//! let mut d = b"%vsm%\x00\x00\x00stuff\x00".to_vec();
//! d.extend_from_slice(b"%header%");
//! let v = izanagi_kit::vp3::parse(&d).unwrap();
//! assert_eq!(v.magic_ok, true);
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed VP3 file header.
#[derive(Clone, Debug)]
pub struct Vp3 {
    /// `true` when the `%vsm%` signature matched.
    pub magic_ok: bool,
    /// File-format version (`u16` after the magic + NUL pad).
    pub version: u16,
    /// `produced by` / vendor comment bytes as text (up to the first
    /// section marker).
    pub comment: String,
    /// Offsets of `%name%`-style section markers found.
    pub sections: Vec<usize>,
}

/// Parse a VP3 file; `None` without the `%vsm%` signature.
pub fn parse(d: &[u8]) -> Option<Vp3> {
    if !d.starts_with(b"%vsm%") {
        return None;
    }
    // Layout: "%vsm%" + one NUL pad byte + u16 BE version.
    if d.len() < 8 || d[5] != 0 {
        return None;
    }
    let version = (u16::from(d[6]) << 8) | u16::from(d[7]);
    let mut i = 8;
    while i < d.len() && d[i] == 0 {
        i += 1;
    }
    // Collect `%section%` markers.
    let mut sections = Vec::new();
    for (p, w) in d.windows(2).enumerate() {
        // `%name%`-style section markers; 'v' is the `%vsm%` magic.
        if w[0] == b'%' && w[1].is_ascii_lowercase() && w[1] != b'v' {
            sections.push(p);
        }
    }
    let comment = {
        let region = &d[i..d.len().min(i + 64)];
        let end = region
            .iter()
            .position(|&b| b == 0 || b == b'%')
            .unwrap_or(region.len());
        String::from_utf8_lossy(&region[..end]).to_string()
    };
    Some(Vp3 {
        magic_ok: true,
        version,
        comment,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut d = b"%vsm%\x00\x00\x01".to_vec(); // version = 1 BE
        d.extend_from_slice(b"vendor\x00");
        d.extend_from_slice(b"%header%data");
        let v = parse(&d).unwrap();
        assert!(v.magic_ok);
        assert_eq!(v.version, 1);
        assert!(!v.sections.is_empty());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"%vp3%\x00").is_none());
    }
}

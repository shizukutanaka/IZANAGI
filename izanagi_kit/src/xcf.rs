//! GIMP XCF image header parsing.
//!
//! Header: `gimp xcf ` + version string (`file`, `v001`–`v023`,
//! or `001`+…) NUL-terminated, then `width u32BE`, `height u32BE`,
//! `precision u32BE` (150/300/600/650/700 for 8/16/32 int/float
//! variants in XCF 11+).
//!
//! ```
//! use izanagi_kit::xcf;
//! let mut d = b"gimp xcf v001".to_vec();
//! d.push(0);
//! d.extend_from_slice(&256u32.to_be_bytes()); // w
//! d.extend_from_slice(&256u32.to_be_bytes()); // h
//! d.extend_from_slice(&150u32.to_be_bytes()); // precision
//! let x = xcf::parse(&d).unwrap();
//! assert_eq!(x.version.as_str(), "v001");
//! ```

use std::string::String;

/// A parsed XCF header.
#[derive(Clone, Debug, PartialEq)]
pub struct Xcf {
    /// Version string after `gimp xcf ` (`file` = 0, `vNNN` = N).
    pub version: String,
    /// Canvas width.
    pub width: u32,
    /// Canvas height.
    pub height: u32,
    /// Precision code (150 = 8-bit int, 300 = 16-bit int, …).
    pub precision: u32,
    /// Byte offset where image properties begin.
    pub props_at: usize,
}

/// Parses an XCF header: `gimp xcf ` magic, NUL-terminated version
/// string, then width/height/precision big-endian u32s.
pub fn parse(d: &[u8]) -> Option<Xcf> {
    let magic = b"gimp xcf ";
    if !d.starts_with(magic) {
        return None;
    }
    let vstart = magic.len();
    let nul = d.get(vstart..)?.iter().position(|&b| b == 0)?;
    let ver_b = d.get(vstart..vstart + nul)?;
    if ver_b.is_empty() || ver_b.len() > 8 {
        return None;
    }
    let version = String::from_utf8_lossy(ver_b).into_owned();
    if !(version == "file"
        || (version.len() == 4
            && version.starts_with('v')
            && version.bytes().skip(1).all(|b| b.is_ascii_digit())))
    {
        return None;
    }
    let mut at = vstart + nul + 1;
    let be32 = |at: usize| -> Option<u32> {
        let s = d.get(at..at.checked_add(4)?)?;
        Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
    };
    let width = be32(at)?;
    let height = be32(at + 4)?;
    let precision = be32(at + 8)?;
    at += 12;
    if width == 0 || height == 0 {
        return None;
    }
    Some(Xcf {
        version,
        width,
        height,
        precision,
        props_at: at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(ver: &[u8]) -> Vec<u8> {
        let mut d = b"gimp xcf ".to_vec();
        d.extend_from_slice(ver);
        d.push(0);
        d.extend_from_slice(&640u32.to_be_bytes());
        d.extend_from_slice(&480u32.to_be_bytes());
        d.extend_from_slice(&150u32.to_be_bytes());
        d
    }

    #[test]
    fn parses() {
        let x = parse(&fixture(b"v004")).unwrap();
        assert_eq!(x.version, "v004");
        assert_eq!((x.width, x.height), (640, 480));
        assert_eq!(x.precision, 150);
        let x0 = parse(&fixture(b"file")).unwrap();
        assert_eq!(x0.version, "file");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"gimp xcv v001\x00").is_none());
        assert!(parse(b"gimp xcf ").is_none()); // no NUL/version
        assert!(parse(&fixture(b"vXYZ")).is_none());
        assert!(parse(&fixture(b"v001")[..10]).is_none());
    }
}

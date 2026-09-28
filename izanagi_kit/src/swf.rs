//! SWF (Shockwave Flash / Adobe Flash, SWF spec): signature `FWS`
//! (raw) / `CWS` (zlib) / `ZWS` (LZMA) + `u8` version + `u32le`
//! declared (uncompressed) file length.
//!
//! ```
//! use izanagi_kit::swf::{detect, parse};
//!
//! let d = b"CWS\x0A\x38\x00\x00\x00rest-of-file";
//! assert!(detect(d));
//! let s = parse(d).unwrap();
//! assert_eq!(s.compression, "zlib");
//! assert_eq!(s.version, 10);
//! assert_eq!(s.declared_len, 56);
//! ```

fn le32(b: &[u8]) -> u32 {
    u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16 | u32::from(b[3]) << 24
}

/// Parsed SWF header.
#[derive(Debug, Clone, PartialEq)]
pub struct Swf {
    /// `"none"` (FWS), `"zlib"` (CWS) or `"lzma"` (ZWS).
    pub compression: &'static str,
    /// Format version byte.
    pub version: u8,
    /// Declared uncompressed file length (`u32le` at offset 4).
    pub declared_len: u32,
    /// `declared_len == actual buffer length` (only meaningful for
    /// `FWS`, where the declared length is the on-disk length).
    pub length_matches: bool,
}

/// `true` on `FWS`/`CWS`/`ZWS` + room for the length field.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8 && matches!(b[0], b'F' | b'C' | b'Z') && b[1] == b'W' && b[2] == b'S'
}

/// Parses the 8-byte header; `None` without a signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Swf> {
    if !detect(b) {
        return None;
    }
    let compression = match b[0] {
        b'C' => "zlib",
        b'Z' => "lzma",
        _ => "none",
    };
    let declared_len = le32(&b[4..8]);
    Some(Swf {
        compression,
        version: b[3],
        declared_len,
        length_matches: declared_len as usize == b.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_works() {
        assert!(detect(b"FWS\x09\x08\x00\x00\x00"));
        assert!(detect(b"CWS\x0A\x38\x00\x00\x00x"));
        assert!(detect(b"ZWS\x0D\x38\x00\x00\x00x"));
        assert!(!detect(b"GWS\x0A\x38\x00\x00\x00"));
        assert!(!detect(b"FWS"));
    }

    #[test]
    fn parses() {
        let s = parse(b"FWS\x09\x08\x00\x00\x00").unwrap();
        assert_eq!(s.compression, "none");
        assert_eq!(s.version, 9);
        assert_eq!(s.declared_len, 8);
        assert!(s.length_matches);
        let c = parse(b"CWS\x0A\x38\x00\x00\x00rest-of-file").unwrap();
        assert_eq!(c.compression, "zlib");
        assert!(!c.length_matches);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"FWS!").is_none());
    }
}

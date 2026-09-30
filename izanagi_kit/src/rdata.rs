//! R `.rds` / `.RData` — serialized workspace. Raw serialization begins
//! `RDX<ver>\n` then a format line `A`/`B`/`X` (ascii/binary/native).
//! Files are typically gzip/xz/zstd wrapped; this parser reports the
//! compression and validates the inner header only when stored raw.
//!
//! ```
//! let d = b"RDX3\nB\n";
//! let r = izanagi_kit::rdata::parse(d).unwrap();
//! assert_eq!(r.format, b'B');
//! assert!(!r.gzip_wrapped());
//! ```

/// Parsed R serialization prolog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rdata {
    /// `RDX` digit (`'2'`, `'3'`, `'4'`) or 0 for compressed input.
    pub rdx: u8,
    /// Format code: `'A'` ascii, `'B'` binary, `'X'` xdr, 0 unknown.
    pub format: u8,
    /// Outer compression: `"none"`, `"gzip"`, `"xz"`, or `"zstd"`.
    pub compression: &'static str,
    /// R writer version triplet when the 3rd ASCII line is present
    /// (`v.major.minor` kept as the raw text after the format line).
    pub version_line: Option<String>,
}

impl Rdata {
    /// True when the payload is gzip-wrapped (`0x1F8B` member).
    pub fn gzip_wrapped(&self) -> bool {
        self.compression == "gzip"
    }
}

/// Parse the prolog; `None` on garbage.
pub fn parse(d: &[u8]) -> Option<Rdata> {
    if d.starts_with(&[0x1F, 0x8B]) {
        return Some(Rdata {
            rdx: 0,
            format: 0,
            compression: "gzip",
            version_line: None,
        });
    }
    if d.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A]) {
        return Some(Rdata {
            rdx: 0,
            format: 0,
            compression: "xz",
            version_line: None,
        });
    }
    if d.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        return Some(Rdata {
            rdx: 0,
            format: 0,
            compression: "zstd",
            version_line: None,
        });
    }
    if !d.starts_with(b"RDX") || !matches!(d.get(3), Some(b'2'..=b'4')) {
        return None;
    }
    if d.get(4)? != &b'\n' {
        return None;
    }
    let format = *d.get(5)?;
    if !matches!(format, b'A' | b'B' | b'X') {
        return None;
    }
    if d.get(6)? != &b'\n' {
        return None;
    }
    let version_line = d.get(7..).and_then(|rest| {
        let end = rest.iter().position(|&b| b == b'\n')?;
        std::str::from_utf8(&rest[..end]).ok().map(str::to_string)
    });
    Some(Rdata {
        rdx: d[3] - b'0',
        format,
        compression: "none",
        version_line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_binary() {
        let r = parse(b"RDX3\nB\n7 4 5 0\n").unwrap();
        assert_eq!((r.rdx, r.format, r.compression), (3, b'B', "none"));
        assert_eq!(r.version_line.as_deref(), Some("7 4 5 0"));
    }

    #[test]
    fn compressed() {
        let r = parse(&[0x1F, 0x8B, 8, 0]).unwrap();
        assert!(r.gzip_wrapped());
        assert_eq!(
            parse(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0])
                .unwrap()
                .compression,
            "xz"
        );
        assert_eq!(
            parse(&[0x28, 0xB5, 0x2F, 0xFD, 0]).unwrap().compression,
            "zstd"
        );
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RDX5\nB\n").is_none()); // rdx version out of range
        assert!(parse(b"RDX3\nQ\n").is_none()); // bad format char
        assert!(parse(b"RDX3").is_none()); // truncated
    }
}

//! MDict `.mdx` — a `u32` header length followed by a `key="value"`
//! attribute header: UTF-16LE for engine v2, UTF-8 for v3. Known
//! keys include `Title`, `Encoding`, `Format`, `KeyCaseSensitive`,
//! `CreatedByVersion`, `RequiredEngineVersion`, `Description`.
//! A 4-byte Adler32 checksum follows the header text.
//!
//! ```
//! use izanagi_kit::mdx::{detect, parse};
//!
//! let h = "Title=\"Oxford\" Encoding=\"UTF-8\" Format=\"Html\" CreatedByVersion=\"2\x2e0\x2e0\"";
//! let mut d = (h.len() as u32).to_be_bytes().to_vec();
//! d.extend_from_slice(h.as_bytes());
//! d.extend_from_slice(&[0, 0, 0, 0]);
//! assert!(detect(&d));
//! let m = parse(&d).unwrap();
//! assert_eq!(m.title.as_deref(), Some("Oxford"));
//! assert!(!m.utf16);
//! ```

/// Parsed MDict `.mdx` header census.
#[derive(Debug, Clone, PartialEq)]
pub struct Mdx {
    /// Header byte length as declared (excludes the 4B prefix + 4B checksum).
    pub header_bytes: u32,
    /// `true` when the header decoded as UTF-16LE (engine v2 layout).
    pub utf16: bool,
    /// `Title` attribute.
    pub title: Option<String>,
    /// `Encoding` attribute.
    pub encoding: Option<String>,
    /// `Format` attribute (`Html` / `Text`).
    pub format: Option<String>,
    /// `CreatedByVersion` / `RequiredEngineVersion`.
    pub engine_version: Option<String>,
    /// `Description` attribute.
    pub description: Option<String>,
    /// Total `key="value"` attributes.
    pub attributes: u32,
}

fn le32(b: &[u8], off: usize) -> Option<u32> {
    Some(
        u32::from(*b.get(off)?)
            | u32::from(*b.get(off + 1)?) << 8
            | u32::from(*b.get(off + 2)?) << 16
            | u32::from(*b.get(off + 3)?) << 24,
    )
}

fn be32(b: &[u8], off: usize) -> Option<u32> {
    Some(
        u32::from(*b.get(off)?) << 24
            | u32::from(*b.get(off + 1)?) << 16
            | u32::from(*b.get(off + 2)?) << 8
            | u32::from(*b.get(off + 3)?),
    )
}

fn utf16le_ascii(b: &[u8]) -> Option<String> {
    if b.len() % 2 != 0 {
        return None;
    }
    let mut s = String::with_capacity(b.len() / 2);
    for w in b.chunks_exact(2) {
        let u = u16::from(w[0]) | u16::from(w[1]) << 8;
        if u == 0 || u > 0x7f {
            return None;
        }
        s.push(char::from(u as u8));
    }
    Some(s)
}

fn header_text(b: &[u8]) -> Option<(String, u32, bool)> {
    if b.len() < 8 {
        return None;
    }
    for endian in [true, false] {
        let n = if endian { be32(b, 0)? } else { le32(b, 0)? };
        let n = usize::try_from(n).ok()?;
        if n == 0 || n > 1_048_576 || 4 + n > b.len() {
            continue;
        }
        let region = &b[4..4 + n];
        if let Some(s) = utf16le_ascii(region) {
            if s.contains("=\"") {
                return Some((s, u32::try_from(n).ok()?, true));
            }
        }
        if let Ok(s) = core::str::from_utf8(region) {
            if s.contains("=\"") {
                return Some((s.to_string(), u32::try_from(n).ok()?, false));
            }
        }
    }
    None
}

fn attr<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    for m in s.match_indices(key) {
        let i = m.0;
        let start_ok = i == 0 || s.as_bytes()[i - 1].is_ascii_whitespace();
        if !start_ok {
            continue;
        }
        let rest = &s[i + key.len()..];
        let Some(val) = rest.strip_prefix("=\"") else {
            continue;
        };
        return val.split('"').next();
    }
    None
}

/// `true` when a plausible `key="value"` header follows the length prefix.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    header_text(b).is_some_and(|(s, _, _)| {
        [
            "Title",
            "Encoding",
            "Format",
            "CreatedByVersion",
            "RequiredEngineVersion",
        ]
        .iter()
        .any(|k| s.contains(k))
    })
}

/// Census; `None` without a decodable header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mdx> {
    if !detect(b) {
        return None;
    }
    let (s, header_bytes, utf16) = header_text(b)?;
    let mut m = Mdx {
        header_bytes,
        utf16,
        title: attr(&s, "Title").map(str::to_string),
        encoding: attr(&s, "Encoding").map(str::to_string),
        format: attr(&s, "Format").map(str::to_string),
        engine_version: attr(&s, "CreatedByVersion")
            .or_else(|| attr(&s, "RequiredEngineVersion"))
            .map(str::to_string),
        description: attr(&s, "Description").map(str::to_string),
        attributes: 0,
    };
    let mut rest = s.as_str();
    while let Some(p) = rest.find("=\"") {
        m.attributes += 1;
        rest = &rest[p + 2..];
        match rest.find('"') {
            Some(q) => rest = &rest[q + 1..],
            None => break,
        }
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(h: &str) -> Vec<u8> {
        let mut d = (h.len() as u32).to_be_bytes().to_vec();
        d.extend_from_slice(h.as_bytes());
        d.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        d
    }

    fn build_utf16(h: &str) -> Vec<u8> {
        let mut body = Vec::new();
        for c in h.bytes() {
            body.push(c);
            body.push(0);
        }
        let mut d = (body.len() as u32).to_be_bytes().to_vec();
        d.extend_from_slice(&body);
        d.extend_from_slice(&[0, 0, 0, 0]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&build("Title=\"A\" CreatedByVersion=\"2\x2e0\"")));
        assert!(detect(&build_utf16("Title=\"A\" Format=\"Html\"")));
        assert!(!detect(b"plain"));
        assert!(!detect(&[0, 0, 0, 4, b'a', b'b', b'c', b'd']));
    }

    #[test]
    fn parses() {
        let m = parse(&build(
            "Title=\"Oxford\" Encoding=\"UTF-8\" Format=\"Html\" CreatedByVersion=\"2\x2e0\x2e0\" Description=\"D\"",
        ))
        .unwrap();
        assert!(!m.utf16);
        assert_eq!(m.title.as_deref(), Some("Oxford"));
        assert_eq!(m.encoding.as_deref(), Some("UTF-8"));
        assert_eq!(m.format.as_deref(), Some("Html"));
        assert_eq!(m.engine_version.as_deref(), Some("2.0.0"));
        assert_eq!(m.description.as_deref(), Some("D"));
        assert_eq!(m.attributes, 5);
        assert!(m.header_bytes > 0);
    }

    #[test]
    fn parses_utf16() {
        let m = parse(&build_utf16("Title=\"L\" Encoding=\"UTF-16\"")).unwrap();
        assert!(m.utf16);
        assert_eq!(m.title.as_deref(), Some("L"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

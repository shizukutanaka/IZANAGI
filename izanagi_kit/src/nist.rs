//! NIST SPHERE audio headers — 1024-byte ASCII `NIST_1A` header ending in `end_head`.
//!
//! ```
//! let h = b"NIST_1A\n   1024\nchannel_count -i 1\nsample_rate -i 8000\nsample_n_bytes -i 2\nend_head\n";
//! let n = izanagi_kit::nist::parse(h).unwrap();
//! assert_eq!(n.sample_rate, Some(8000));
//! assert_eq!(izanagi_kit::nist::detect(h), true);
//! ```

/// Parsed NIST SPHERE header fields.
#[derive(Debug, Clone)]
pub struct Nist {
    /// Header size declared on line 2 (usually 1024).
    pub header_size: u32,
    /// `channel_count`.
    pub channels: Option<u32>,
    /// `sample_rate` Hz.
    pub sample_rate: Option<u32>,
    /// `sample_n_bytes` (1 or 2 typical).
    pub sample_bytes: Option<u32>,
    /// `sample_count`.
    pub sample_count: Option<u32>,
    /// `sample_byte_format` (`01` LE / `10` BE).
    pub byte_format: Option<String>,
    /// `sample_coding` (`pcm`, `ulaw`, `alaw` …).
    pub coding: Option<String>,
    /// `end_head` marker seen.
    pub complete: bool,
}

/// Detects `NIST_1A`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"NIST_1A")
}

fn field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    for l in s.lines() {
        let l = l.trim();
        if let Some(rest) = l.strip_prefix(key) {
            // "key -i value" / "key -s3 value": the value is the last token
            return rest.split_whitespace().last();
        }
    }
    None
}

fn num(s: &str, key: &str) -> Option<u32> {
    field(s, key).and_then(|v| v.parse().ok())
}

/// Parses a SPHERE header; `None` without `NIST_1A`.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nist> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut it = s.lines();
    it.next();
    let header_size: u32 = it.next()?.trim().parse().ok()?;
    let f = Nist {
        header_size,
        channels: num(s, "channel_count"),
        sample_rate: num(s, "sample_rate"),
        sample_bytes: num(s, "sample_n_bytes"),
        sample_count: num(s, "sample_count"),
        byte_format: field(s, "sample_byte_format").map(String::from),
        coding: field(s, "sample_coding").map(String::from),
        complete: s.lines().any(|l| l.trim() == "end_head"),
    };
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"NIST_1A\n   1024\nchannel_count -i 1\nsample_rate -i 16000\nsample_n_bytes -i 2\nsample_count -i 320\nsample_byte_format -s2 01\nsample_coding -s3 pcm\nend_head\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.header_size, 1024);
        assert_eq!(f.channels, Some(1));
        assert_eq!(f.sample_rate, Some(16000));
        assert_eq!(f.sample_bytes, Some(2));
        assert_eq!(f.sample_count, Some(320));
        assert_eq!(f.byte_format.as_deref(), Some("01"));
        assert_eq!(f.coding.as_deref(), Some("pcm"));
        assert!(f.complete);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"NIST_1A\n").is_none());
        assert!(!detect(b"NIST_1B\n"));
        assert!(parse(&[0xff; 64]).is_none());
    }

    #[test]
    fn incomplete_header() {
        let f = parse(b"NIST_1A\n   1024\nsample_rate -i 8000\n").unwrap();
        assert!(!f.complete);
        assert_eq!(f.sample_rate, Some(8000));
    }
}

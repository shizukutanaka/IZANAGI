//! asciinema `.cast` recordings — a JSON header line
//! (`{"version":N,"width":W,"height":H,…}`), then one JSON array per
//! event line `[t,"o","data"]` for v2, or an embedded `"stdout"` record
//! list for v1.
//!
//! ```
//! let d = b"{\"version\":2,\"width\":80,\"height\":24,\"title\":\"t\"}\n\
//! [0\x2e1,\"o\",\"hi\"]\n[0\x2e2,\"i\",\"x\"]\n[1\x2e0,\"m\",\"\"]\n";
//! let c = izanagi_kit::asciicast::parse(d).unwrap();
//! assert_eq!(c.version, 2);
//! assert_eq!(c.events, 3);
//! assert_eq!(c.output_events, 1);
//! assert!(izanagi_kit::asciicast::detect(d));
//! ```

/// Census of an asciinema `.cast` file.
#[derive(Debug, Clone)]
pub struct Asciicast {
    /// Format version (1 or 2).
    pub version: u32,
    /// `width` from the header.
    pub width: Option<u32>,
    /// `height` from the header.
    pub height: Option<u32>,
    /// Event lines after the header (v2).
    pub events: usize,
    /// `"o"` output events.
    pub output_events: usize,
    /// `"i"` input events.
    pub input_events: usize,
    /// `"m"` marker events.
    pub markers: usize,
    /// Header carries `"title"`.
    pub title: bool,
    /// Header carries `"env"`.
    pub env: bool,
    /// v1-style embedded `"stdout"` record list.
    pub v1_stdout: bool,
}

fn num_after(line: &str, key: &str) -> Option<u32> {
    let at = line.find(key)? + key.len();
    let rest = line.get(at..)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn scan(b: &[u8]) -> Option<Asciicast> {
    let t = std::str::from_utf8(b).ok()?;
    let mut lines = t.lines();
    let head = lines.next()?.trim();
    if !head.starts_with('{') || !head.contains("\"version\"") {
        return None;
    }
    let version = num_after(head, "\"version\":")?;
    if version != 1 && version != 2 {
        return None;
    }
    let mut c = Asciicast {
        version,
        width: num_after(head, "\"width\":"),
        height: num_after(head, "\"height\":"),
        events: 0,
        output_events: 0,
        input_events: 0,
        markers: 0,
        title: head.contains("\"title\""),
        env: head.contains("\"env\""),
        v1_stdout: head.contains("\"stdout\""),
    };
    for raw in lines {
        let l = raw.trim();
        if l.is_empty() {
            continue;
        }
        c.events += 1;
        if l.contains(",\"o\",") {
            c.output_events += 1;
        } else if l.contains(",\"i\",") {
            c.input_events += 1;
        } else if l.contains(",\"m\",") {
            c.markers += 1;
        }
    }
    // v1 embeds the record list; v2 needs a size header or event lines
    if c.v1_stdout || c.events > 0 || (c.width.is_some() && c.height.is_some()) {
        return Some(c);
    }
    None
}

/// Detects a `.cast`: a `{"version":1|2,…}` header with terminal size or events.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    scan(b).is_some()
}

/// Parses a `.cast`; `None` on non-JSON first line or wrong version.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Asciicast> {
    scan(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"{\"version\":2,\"width\":80,\"height\":24,\"title\":\"x\",\"env\":{\"TERM\":\"vt\"}}\n[0\x2e01,\"o\",\"hi\"]\n[0\x2e5,\"o\",\"yo\"]\n[1\x2e0,\"i\",\"q\"]\n[2\x2e0,\"m\",\"\"]\n";

    #[test]
    fn parses() {
        let c = parse(D).unwrap();
        assert_eq!(c.version, 2);
        assert_eq!(c.width, Some(80));
        assert_eq!(c.height, Some(24));
        assert_eq!(c.events, 4);
        assert_eq!(c.output_events, 2);
        assert_eq!(c.input_events, 1);
        assert_eq!(c.markers, 1);
        assert!(c.title && c.env);
        assert!(!c.v1_stdout);
    }

    #[test]
    fn v1_form() {
        let c = parse(b"{\"version\":1,\"width\":80,\"height\":24,\"stdout\":[]}\n").unwrap();
        assert_eq!(c.version, 1);
        assert!(c.v1_stdout);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"{\"version\":3}"));
        assert!(!detect(b"{}"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"{\"version\":2}").is_none()); // no size, no events
        assert!(parse(b"not json").is_none());
    }
}

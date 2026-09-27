//! OpenTSDB telnet-style `put` lines:
//! `put metric timestamp value tagk=tagv ...` (opentsdb HTTP/telnet API).
//!
//! ```
//! use izanagi_kit::opentsdb::parse_line;
//!
//! let p = parse_line(b"put sys.cpu.user 1700000000 42 host=web1 dc=east").unwrap();
//! assert_eq!(p.metric, "sys.cpu.user");
//! assert_eq!(p.tags.len(), 2);
//! ```

/// One `put` datapoint.
#[derive(Debug, Clone)]
pub struct Put {
    /// Metric name (letters, digits, `-`, `_`, `.`, `/`).
    pub metric: String,
    /// Unix seconds or milliseconds, verbatim.
    pub timestamp: String,
    /// Value verbatim.
    pub value: String,
    /// Tag pairs; at least one is required.
    pub tags: Vec<(String, String)>,
}

fn name_ok(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/'))
}

/// Parse one `put` line (the leading `put ` may be omitted in bulk files).
pub fn parse_line(line: &[u8]) -> Option<Put> {
    let text = std::str::from_utf8(line).ok()?;
    let body = text.strip_prefix("put ").unwrap_or(text);
    let f: Vec<&str> = body.split_whitespace().collect();
    if f.len() < 4 || !name_ok(f[0]) {
        return None;
    }
    let mut tags = Vec::new();
    for t in &f[3..] {
        let (k, v) = t.split_once('=')?;
        if k.is_empty() || v.is_empty() || !name_ok(k) {
            return None;
        }
        tags.push((k.to_string(), v.to_string()));
    }
    Some(Put {
        metric: f[0].to_string(),
        timestamp: f[1].to_string(),
        value: f[2].to_string(),
        tags,
    })
}

/// Parse a batch of `put` lines; blank lines skipped.
pub fn parse(data: &[u8]) -> Option<Vec<Put>> {
    let text = std::str::from_utf8(data).ok()?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        out.push(parse_line(line.as_bytes())?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let p = parse_line(b"put m.lat 1700 5 host=a\n").unwrap();
        assert_eq!(p.tags[0], ("host".to_string(), "a".to_string()));
        let q = parse_line(b"m.lat 1700 5 host=a").unwrap(); // no `put` prefix
        assert_eq!(q.metric, "m.lat");
    }

    #[test]
    fn rejects() {
        assert!(parse_line(b"put m 1 5\n").is_none()); // no tags
        assert!(parse_line(b"put m 1 5 badtag\n").is_none());
        assert!(parse_line(b"put m!bad 1 5 t=v\n").is_none());
    }
}

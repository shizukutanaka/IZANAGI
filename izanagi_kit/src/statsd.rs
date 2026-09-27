//! StatsD datagram/text lines: `name:value|type[|@rate][|#tag:v,...]`
//! (etsy statsd / DogStatsD).
//!
//! ```
//! use izanagi_kit::statsd::parse_line;
//!
//! let m = parse_line(b"reqs:5|c|@0.1|#env:prod").unwrap();
//! assert_eq!(m.name, "reqs");
//! assert_eq!(m.metric_type, "c");
//! assert_eq!(m.sample_rate.as_deref(), Some("0.1"));
//! assert_eq!(m.tags, vec![("env".to_string(), "prod".to_string())]);
//! ```

/// One StatsD metric.
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric name.
    pub name: String,
    /// Value verbatim (ints/floats both legal).
    pub value: String,
    /// `c` counter, `g` gauge, `ms`/`h` timer, `s` set — kept verbatim.
    pub metric_type: String,
    /// Optional `@rate` verbatim.
    pub sample_rate: Option<String>,
    /// DogStatsD `#` tags as pairs (`k:v` split on first `:`).
    pub tags: Vec<(String, String)>,
}

/// Parse one metric line or datagram.
pub fn parse_line(line: &[u8]) -> Option<Metric> {
    let text = std::str::from_utf8(line).ok()?;
    let mut parts = text.split('|');
    let first = parts.next()?;
    let (name, value) = first.split_once(':')?;
    if name.is_empty() || value.is_empty() {
        return None;
    }
    let metric_type = parts.next()?.to_string();
    if metric_type.is_empty() {
        return None;
    }
    let mut sample_rate = None;
    let mut tags = Vec::new();
    for p in parts {
        if let Some(r) = p.strip_prefix('@') {
            sample_rate = Some(r.to_string());
            continue;
        }
        if let Some(t) = p.strip_prefix('#') {
            for piece in t.split(',') {
                let (k, v) = piece.split_once(':').unwrap_or((piece, ""));
                if k.is_empty() {
                    return None;
                }
                tags.push((k.to_string(), v.to_string()));
            }
            continue;
        }
        return None; // unknown extension
    }
    Some(Metric {
        name: name.to_string(),
        value: value.to_string(),
        metric_type,
        sample_rate,
        tags,
    })
}

/// Parse a multi-line batch (UDP datagrams are usually one per datagram,
/// but newline-batched forms exist).
pub fn parse(data: &[u8]) -> Option<Vec<Metric>> {
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
        let m = parse_line(b"g.users:100|g").unwrap();
        assert_eq!(m.metric_type, "g");
        assert!(m.tags.is_empty());
        let d = parse_line(b"page.view:1|c|#a").unwrap();
        assert_eq!(d.tags[0], ("a".to_string(), String::new()));
    }

    #[test]
    fn rejects() {
        assert!(parse_line(b"nocontent").is_none());
        assert!(parse_line(b"n:1|").is_none()); // empty type
        assert!(parse_line(b"n:1|c|bogus").is_none());
    }
}

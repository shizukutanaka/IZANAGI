//! InfluxDB line protocol: `measurement,tag=v field=v[,...] [timestamp]`.
//! Field values stay verbatim (`i`/`u` integer suffixes, floats, `t/f`,
//! quoted strings all pass through).
//!
//! ```
//! use izanagi_kit::influx::parse_line;
//!
//! let l = parse_line(b"cpu,host=a,region=w idle=64i,user=12 1700000000000000000").unwrap();
//! assert_eq!(l.measurement, "cpu");
//! assert_eq!(l.tags, vec![("host".to_string(), "a".to_string()), ("region".to_string(), "w".to_string())]);
//! assert_eq!(l.fields[0].0, "idle");
//! ```

/// One line-protocol record.
#[derive(Debug, Clone)]
pub struct Line {
    /// Measurement name (commas/spaces unescaped here).
    pub measurement: String,
    /// Tag set in written order.
    pub tags: Vec<(String, String)>,
    /// Field set in written order (key, raw value incl. `i`/`u` suffix).
    pub fields: Vec<(String, String)>,
    /// Optional timestamp verbatim.
    pub timestamp: Option<String>,
}

fn split_escaped(s: &str, sep: u8) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut esc = false;
    for c in s.chars() {
        if esc {
            cur.push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == sep as char {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

fn kv(s: &str) -> Option<(String, String)> {
    let (k, v) = s.split_once('=')?;
    if k.is_empty() || v.is_empty() {
        return None;
    }
    Some((k.to_string(), v.to_string()))
}

/// Parse one line.
pub fn parse_line(line: &[u8]) -> Option<Line> {
    let text = std::str::from_utf8(line).ok()?;
    let mut it = text.splitn(2, ' ');
    let first = it.next()?;
    let rest = it.next().unwrap_or("");
    let mut parts = first.splitn(2, ',');
    let measurement = parts.next()?.to_string();
    if measurement.is_empty() {
        return None;
    }
    let mut tags = Vec::new();
    if let Some(t) = parts.next() {
        for p in split_escaped(t, b',') {
            tags.push(kv(&p)?);
        }
    }
    let mut rest_it = rest.rsplitn(2, ' ');
    let (fields_part, ts) = {
        let tail = rest_it.next().unwrap_or("");
        let head = rest_it.next().unwrap_or("");
        // `head tail`: if tail is all digits it is a timestamp
        if head.is_empty() {
            (tail, None)
        } else if !tail.is_empty()
            && tail.bytes().all(|b| b.is_ascii_digit() || b == b'-')
            && head.contains('=')
        {
            (head, Some(tail.to_string()))
        } else {
            (rest.trim(), None)
        }
    };
    let mut fields = Vec::new();
    for p in split_escaped(fields_part, b',') {
        fields.push(kv(&p)?);
    }
    if fields.is_empty() {
        return None;
    }
    Some(Line {
        measurement,
        tags,
        fields,
        timestamp: ts,
    })
}

/// Parse a multi-line buffer; blank lines and `#` comments are skipped.
pub fn parse(data: &[u8]) -> Option<Vec<Line>> {
    let text = std::str::from_utf8(data).ok()?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
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
        let l = parse_line(b"m,t1=a\\,b f=1i,g=x 99").unwrap();
        assert_eq!(l.tags[0], ("t1".to_string(), "a,b".to_string()));
        assert_eq!(l.fields.len(), 2);
        assert_eq!(l.timestamp.as_deref(), Some("99"));
        let n = parse_line(b"m f=1").unwrap();
        assert_eq!(n.timestamp, None);
    }

    #[test]
    fn rejects() {
        assert!(parse_line(b"").is_none());
        assert!(parse_line(b"m").is_none()); // no fields
        assert!(parse_line(b"m,t=x ").is_none());
    }
}

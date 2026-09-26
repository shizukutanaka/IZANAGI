//! Graphite plaintext protocol: `path value timestamp` per line
//! (carbon-cache feed format).
//!
//! ```
//! use izanagi_kit::graphite::parse;
//!
//! let d = b"servers.web.cpu 42 1700000000\napp.req.count 7 1700000001\n";
//! let g = parse(d).unwrap();
//! assert_eq!(g.len(), 2);
//! assert_eq!(g[0].path, "servers.web.cpu");
//! assert_eq!(g[0].timestamp, 1700000000);
//! ```

/// One graphite datapoint. `value` is kept verbatim (graphite accepts floats).
#[derive(Debug, Clone)]
pub struct Point {
    /// Dotted metric path.
    pub path: String,
    /// Value text.
    pub value: String,
    /// Unix timestamp seconds.
    pub timestamp: u64,
}

fn num(s: &str) -> Option<u64> {
    let mut v: u64 = 0;
    let mut any = false;
    for &b in s.as_bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        any = true;
        v = v.checked_mul(10)?.checked_add(u64::from(b - b'0'))?;
    }
    if any {
        Some(v)
    } else {
        None
    }
}

/// Parse every `path value ts` line; blank lines are skipped.
pub fn parse(data: &[u8]) -> Option<Vec<Point>> {
    let text = std::str::from_utf8(data).ok()?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 3 || f[0].is_empty() {
            return None;
        }
        out.push(Point {
            path: f[0].to_string(),
            value: f[1].to_string(),
            timestamp: num(f[2])?,
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = parse(b"a.b.c 1 100\n\nx.y 2.5 200\n").unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d[1].value, "2.5");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"only two\n").is_none());
        assert!(parse(b"a 1 x\n").is_none()); // non-numeric ts
    }
}

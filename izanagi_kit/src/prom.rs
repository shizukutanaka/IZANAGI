//! Prometheus text exposition format: `# HELP`/`# TYPE` comments and
//! `name{label="v",...} value [timestamp]` sample lines. Sample values are
//! kept verbatim (Prometheus allows `NaN`/`+Inf` and arbitrary floats — the
//! sim constraint keeps them textual).
//!
//! ```
//! use izanagi_kit::prom::parse;
//!
//! let d = b"# HELP up alive\n# TYPE up gauge\nup{job=\"x\"} 1 1700000000000\n";
//! let p = parse(d).unwrap();
//! assert_eq!(p.samples.len(), 1);
//! assert_eq!(p.samples[0].name, "up");
//! assert_eq!(p.samples[0].labels, vec![("job".to_string(), "x".to_string())]);
//! ```

/// A `# HELP` or `# TYPE` record.
#[derive(Debug, Clone)]
pub struct Meta {
    /// `HELP` or `TYPE`.
    pub kind: String,
    /// Metric name.
    pub name: String,
    /// Remaining text (doc string or type name).
    pub rest: String,
}

/// One sample line.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Metric name.
    pub name: String,
    /// Label pairs in written order.
    pub labels: Vec<(String, String)>,
    /// Value verbatim.
    pub value: String,
    /// Optional ms timestamp verbatim.
    pub timestamp: Option<String>,
}

/// Parsed exposition.
#[derive(Debug, Clone)]
pub struct Exposition {
    /// Metadata lines.
    pub metas: Vec<Meta>,
    /// Samples.
    pub samples: Vec<Sample>,
}

fn ident(s: &str) -> bool {
    let mut it = s.bytes();
    matches!(it.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_' || b == b':')
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

fn label_list(s: &str) -> Option<Vec<(String, String)>> {
    // s is inside { ... }; entries: k="v" or k="v",k2="v2"
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let eq = rest.find('=')?;
        let k = rest[..eq].trim();
        if !ident(k) {
            return None;
        }
        let rest2 = &rest[eq + 1..];
        if !rest2.starts_with('"') {
            return None;
        }
        let mut i = 1;
        let mut val = String::new();
        loop {
            let c = *rest2.as_bytes().get(i)?;
            if c == b'"' {
                break;
            }
            if c == b'\\' {
                i += 1;
                let e = *rest2.as_bytes().get(i)?;
                val.push(e as char);
            } else {
                val.push(c as char);
            }
            i += 1;
        }
        out.push((k.to_string(), val));
        rest = rest2.get(i + 1..)?;
        if rest.starts_with(',') {
            rest = &rest[1..];
        } else if !rest.is_empty() {
            return None;
        }
    }
    Some(out)
}

/// Parse the whole exposition buffer.
pub fn parse(data: &[u8]) -> Option<Exposition> {
    let text = std::str::from_utf8(data).ok()?;
    let mut metas = Vec::new();
    let mut samples = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            let mut it = rest.splitn(3, ' ');
            let kind = it.next()?;
            if kind != "HELP" && kind != "TYPE" {
                continue; // e.g. "# EOF" or other comments
            }
            let name = it.next().unwrap_or("");
            let rest = it.next().unwrap_or("");
            metas.push(Meta {
                kind: kind.to_string(),
                name: name.to_string(),
                rest: rest.to_string(),
            });
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        // sample: name[{labels}] value [ts]
        let (name, after_name) = match line.find('{') {
            Some(b) => {
                let e = line.rfind('}')?;
                (&line[..b], Some(&line[b + 1..e]))
            }
            None => {
                let sp = line.find(' ')?;
                (&line[..sp], None)
            }
        };
        if !ident(name) {
            return None;
        }
        let labels = match after_name {
            Some(inner) => label_list(inner)?,
            None => Vec::new(),
        };
        let tail_at = line.find('}').map(|e| e + 1).unwrap_or_else(|| name.len());
        let tail = line.get(tail_at..)?.trim();
        let mut parts = tail.split_whitespace();
        let value = parts.next()?.to_string();
        let timestamp = parts.next().map(str::to_string);
        samples.push(Sample {
            name: name.to_string(),
            labels,
            value,
            timestamp,
        });
    }
    Some(Exposition { metas, samples })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"# TYPE c counter\nc_total{a=\"1\",b=\"x\\\"y\"} 5\nbare -2\n";
        let p = parse(d).unwrap();
        assert_eq!(p.metas.len(), 1);
        assert_eq!(p.samples.len(), 2);
        assert_eq!(
            p.samples[0].labels[1],
            ("b".to_string(), "x\"y".to_string())
        );
        assert_eq!(p.samples[1].value, "-2");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"9bad 1\n").is_none());
        assert!(parse(b"ok{bad} 1\n").is_none());
    }
}

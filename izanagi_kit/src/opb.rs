//! OPB — the pseudo-Boolean constraint file: `* #variable= N
//! #constraint= M` header comment, `min:`/`max:` objective line,
//! then constraints as `+w*x1 -w*x2 >= k ;` / `= k ;` sums.
//!
//! ```
//! use izanagi_kit::opb::{detect, parse};
//!
//! let d = b"* #variable= 3 #constraint= 2\n\
//! min: +1*x1 +2*x2 ;\n+1*x1 +1*x2 >= 1 ;\n-1*x1 = 0 ;\n";
//! assert!(detect(d));
//! let o = parse(d).unwrap();
//! assert_eq!(o.declared_vars, Some(3));
//! assert_eq!(o.constraints, 2);
//! ```

/// Parsed OPB census.
#[derive(Debug, Clone, PartialEq)]
pub struct Opb {
    /// `#variable=` header value.
    pub declared_vars: Option<u32>,
    /// `#constraint=` header value.
    pub declared_constraints: Option<u32>,
    /// `min:`/`max:` objective keyword.
    pub objective: Option<String>,
    /// Constraint lines (`;`-terminated, with sense op).
    pub constraints: u32,
    /// `>=` senses.
    pub ge: u32,
    /// `<=` senses.
    pub le: u32,
    /// `=` senses.
    pub eq: u32,
    /// `x…` variable tokens seen (max index).
    pub max_var: u32,
    /// `*` comment lines.
    pub comments: u32,
    /// Soft `soft` pseudo-boolean markers.
    pub soft: u32,
}

fn hdr_num(s: &str, key: &str) -> Option<u32> {
    let i = s.find(key)? + key.len();
    s[i..]
        .split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
}

/// `true` on `#variable=` header or `min:`/`max:` + `>=`/`= ...;`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("#variable=") || ((s.contains("min:") || s.contains("max:")) && s.contains(";"))
}

/// Census; `None` without OPB markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Opb> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut o = Opb {
        declared_vars: hdr_num(s, "#variable="),
        declared_constraints: hdr_num(s, "#constraint="),
        objective: None,
        constraints: 0,
        ge: 0,
        le: 0,
        eq: 0,
        max_var: 0,
        comments: 0,
        soft: 0,
    };
    for line in s.lines() {
        let t = line.trim();
        if t.starts_with('*') {
            o.comments += 1;
            continue;
        }
        if t.starts_with("min:") {
            o.objective = Some("min".to_string());
        } else if t.starts_with("max:") {
            o.objective = Some("max".to_string());
        }
        if t.contains("soft") {
            o.soft += 1;
        }
        for tok in t.split_whitespace() {
            let p = tok.rsplit('*').next().unwrap_or(tok);
            let p = p.strip_prefix('~').unwrap_or(p);
            if let Some(v) = p.strip_prefix('x') {
                if let Ok(n) = v.trim_end_matches(';').parse::<u32>() {
                    o.max_var = o.max_var.max(n);
                }
            }
        }
        if (t.contains(">=") || t.contains("<=") || t.contains('='))
            && !t.starts_with("min")
            && !t.starts_with("max")
        {
            o.constraints += 1;
            if t.contains(">=") {
                o.ge += 1;
            } else if t.contains("<=") {
                o.le += 1;
            } else {
                o.eq += 1;
            }
        }
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"* #variable= 3 #constraint= 3\n\
min: +1*x1 +2*x2 ;\n+1*x1 +1*x2 >= 1 ;\n-1*x3 <= 0 ;\n+1*x1 = 1 ;\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"min: x1 ;\n+1*x1 >= 1 ;"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn parses() {
        let o = parse(D).unwrap();
        assert_eq!(o.declared_vars, Some(3));
        assert_eq!(o.declared_constraints, Some(3));
        assert_eq!(o.objective.as_deref(), Some("min"));
        assert_eq!(o.constraints, 3);
        assert_eq!(o.ge, 1);
        assert_eq!(o.le, 1);
        assert_eq!(o.eq, 1);
        assert_eq!(o.max_var, 3);
        assert_eq!(o.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }

    #[test]
    fn negated_vars() {
        let o = parse(b"* #variable= 4 #constraint= 1\n+1*~x4 >= 1 ;\n").unwrap();
        assert_eq!(o.max_var, 4);
    }
}

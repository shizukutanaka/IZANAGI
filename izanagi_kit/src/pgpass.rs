//! PostgreSQL `.pgpass` password file format.
//!
//! `.pgpass` lines are `host:port:database:user:password` with `\\`
//! escaping, `*` wildcards in the first four fields, `#` comments.
//!
//! ```
//! let b = concat!(
//!     "host1:5432:db:alice:secret\n",
//!     "*:*:*:bob:p\\:ss\n",
//!     "# comment\n",
//!     "*.example.com:5432:db:svc:s3cr3t\n"
//! ).as_bytes();
//! assert!(izanagi_kit::pgpass::detect(b));
//! let c = izanagi_kit::pgpass::Pgpass::parse(b).unwrap();
//! assert_eq!(c.entries, 3);
//! assert_eq!(c.wildcards, 3);
//! ```

/// Parsed .pgpass summary.
#[derive(Debug, Clone)]
pub struct Pgpass {
    /// Valid `a:b:c:d:e` lines.
    pub entries: usize,
    /// `*` wildcard fields.
    pub wildcards: usize,
    /// `\\`/`\\:` escaped characters.
    pub escapes: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// Malformed lines (fewer than 5 fields).
    pub malformed: usize,
}

fn fields(l: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = l.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            cur.push('\\');
            if let Some(n) = chars.next() {
                cur.push(n);
            }
        } else if ch == ':' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    out.push(cur);
    out
}

/// Whether the buffer looks like .pgpass.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let tr = l.trim();
        !tr.is_empty() && !tr.starts_with('#') && fields(tr).len() >= 5
    })
}

impl Pgpass {
    /// Parses a .pgpass summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            wildcards: 0,
            escapes: 0,
            comments: 0,
            malformed: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let fs = fields(tr);
            if fs.len() >= 5 {
                c.entries += 1;
                c.wildcards += fs[..4].iter().filter(|f| f.as_str() == "*").count();
                c.escapes += tr.matches("\\\\").count() + tr.matches("\\:").count();
            } else {
                c.malformed += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pgpass() {
        let b = concat!(
            "host1:5432:db:alice:secret\n",
            "*:*:*:bob:p\\:ss\n",
            "# comment\n",
            "*.example.com:5432:db:svc:s3cr3t\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Pgpass::parse(b).unwrap();
        assert_eq!(c.entries, 3);
        assert_eq!(c.wildcards, 3);
        assert_eq!(c.escapes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"a:b\n"));
        assert!(Pgpass::parse(b"x").is_none());
    }
}

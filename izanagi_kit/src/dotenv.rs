//! dotenv `.env` file format.
//!
//! `.env` files declare `KEY=value` pairs, optionally prefixed with
//! `export `, values possibly single/double quoted or empty, `#`
//! comments, and bare `KEY` flags.
//!
//! ```
//! let b = concat!(
//!     "DB_HOST=localhost\n",
//!     "export API_KEY=\"abc123\"\n",
//!     "# comment\n",
//!     "DEBUG=\n",
//!     "QUOTED='single'\n"
//! ).as_bytes();
//! assert!(izanagi_kit::dotenv::detect(b));
//! let c = izanagi_kit::dotenv::Dotenv::parse(b).unwrap();
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.exports, 1);
//! ```

/// Parsed .env summary.
#[derive(Debug, Clone)]
pub struct Dotenv {
    /// `KEY=value` / `KEY=` assignments.
    pub entries: usize,
    /// `export KEY=...` prefixed lines.
    pub exports: usize,
    /// Values wrapped in `'…'` or `"…"`.
    pub quoted: usize,
    /// `KEY=` with empty value.
    pub empty_values: usize,
    /// Bare `KEY` lines with no `=`.
    pub bare: usize,
    /// `#` comment lines.
    pub comments: usize,
}

fn is_key(k: &str) -> bool {
    !k.is_empty()
        && k.chars()
            .enumerate()
            .all(|(i, c)| c == '_' || (i > 0 && c.is_ascii_digit()) || c.is_ascii_alphabetic())
}

/// Whether the buffer looks like a .env file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        let body = tr.strip_prefix("export ").unwrap_or(tr);
        if let Some((k, _)) = body.split_once('=') {
            if is_key(k.trim()) {
                score += 1;
            }
        }
    }
    score >= 1
}

impl Dotenv {
    /// Parses a .env file summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            exports: 0,
            quoted: 0,
            empty_values: 0,
            bare: 0,
            comments: 0,
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
            let (body, exported) = match tr.strip_prefix("export ") {
                Some(r) => (r, true),
                None => (tr, false),
            };
            match body.split_once('=') {
                Some((k, v)) if is_key(k.trim()) => {
                    c.entries += 1;
                    if exported {
                        c.exports += 1;
                    }
                    let v = v.trim();
                    if v.is_empty() {
                        c.empty_values += 1;
                    } else if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2)
                        || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2)
                    {
                        c.quoted += 1;
                    }
                }
                _ => {
                    if is_key(body.trim()) {
                        c.bare += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv() {
        let b = concat!(
            "DB_HOST=localhost\n",
            "export API_KEY=\"abc123\"\n",
            "# comment\n",
            "DEBUG=\n",
            "QUOTED='single'\n",
            "FLAG_ONLY\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Dotenv::parse(b).unwrap();
        assert_eq!(c.entries, 4);
        assert_eq!(c.exports, 1);
        assert_eq!(c.quoted, 2);
        assert_eq!(c.empty_values, 1);
        assert_eq!(c.bare, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"not a key value pair"));
        assert!(Dotenv::parse(b"1=x").is_none());
    }
}

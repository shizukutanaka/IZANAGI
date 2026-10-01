//! Go `go.sum` checksum database format.
//!
//! go.sum lines are `module version h1:hash=` or
//! `module version/go.mod h1:hash=` entries.
//!
//! ```
//! let b = concat!(
//!     "example.com/a v100 h1:AAAA=\n",
//!     "example.com/a v100/go.mod h1:BBBB=\n",
//!     "example.com/b v200 h1:CCCC=\n"
//! ).as_bytes();
//! assert!(izanagi_kit::gosum::detect(b));
//! let c = izanagi_kit::gosum::Gosum::parse(b).unwrap();
//! assert_eq!(c.entries, 3);
//! assert_eq!(c.mod_only, 1);
//! ```

/// Parsed go.sum summary.
#[derive(Debug, Clone)]
pub struct Gosum {
    /// Module entries.
    pub entries: usize,
    /// `/go.mod` hash-only entries.
    pub mod_only: usize,
    /// Distinct module paths.
    pub modules: usize,
    /// `h1:`-hashed entries.
    pub h1: usize,
}

fn is_entry(l: &str) -> bool {
    // `path version hash` — path non-empty, hash `h1:`/`h2:` style.
    let parts: Vec<&str> = l.split_whitespace().collect();
    parts.len() == 3
        && !parts[0].is_empty()
        && parts[2].starts_with("h")
        && parts[2].contains(':')
        && parts[2].ends_with('=')
}

/// Whether the buffer looks like go.sum.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| is_entry(l.trim()))
}

impl Gosum {
    /// Parses a go.sum summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            entries: 0,
            mod_only: 0,
            modules: 0,
            h1: 0,
        };
        let mut seen = Vec::new();
        for l in t.lines() {
            let tr = l.trim();
            if !is_entry(tr) {
                continue;
            }
            c.entries += 1;
            let mut parts = tr.split_whitespace();
            let path = parts.next().unwrap_or("");
            let ver = parts.next().unwrap_or("");
            if ver.ends_with("/go.mod") {
                c.mod_only += 1;
            }
            if tr.contains("h1:") {
                c.h1 += 1;
            }
            if !seen.contains(&path) {
                seen.push(path);
                c.modules += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gosum() {
        let b = concat!(
            "example.com/a v100 h1:AAAA=\n",
            "example.com/a v100/go.mod h1:BBBB=\n",
            "example.com/b v200 h1:CCCC=\n",
            "example.com/b v200/go.mod h1:DDDD=\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Gosum::parse(b).unwrap();
        assert_eq!(c.entries, 4);
        assert_eq!(c.mod_only, 2);
        assert_eq!(c.modules, 2);
        assert_eq!(c.h1, 4);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"not a checksum\n"));
        assert!(Gosum::parse(b"x").is_none());
    }
}

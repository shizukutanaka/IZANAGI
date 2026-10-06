//! Ory Keto `keto.yaml`/`keto.yml` config census.
//!
//! Keto top-level keys: `serve`, `namespaces`, `limit`,
//! `dsn`, `log`, `tracing`, `metrics`, `profiling`.
//!
//! ```rust
//! let k = b"serve:\n  read:\n    port: 4466\ndsn: postgres://x\nnamespaces:\n  location: file://namespaces\n";
//! assert!(izanagi_kit::keto::detect(k));
//! ```

/// Keto config census.
#[derive(Debug, Clone)]
pub struct Keto {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &["namespaces", "dsn"];

const WEAK: &[&str] = &["serve", "limit", "log", "tracing", "metrics", "profiling"];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect a Keto config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Keto {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
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
    fn detects() {
        let b = b"serve:\n  read:\n    port: 4466\ndsn: postgres://x\nnamespaces:\n  location: file://namespaces\n";
        assert!(detect(b));
        let c = Keto::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"serve:\n  read:\n    port: 1\n"));
        assert!(!detect(b"namespaces: []\n"));
        assert!(!detect(b"# namespaces:\n# dsn: x\n"));
    }
}

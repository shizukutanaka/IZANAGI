//! Permissions-Policy header (W3C): comma-separated directives
//! `feature=()`, `feature=(*)`, `feature=(self "https://e.x")` —
//! plus the legacy bare `feature` (allowlist `*`). Quoted origins keep
//! their quotes stripped; `'self'`/`'src'`/`'none'` markers are kept.
//!
//! ```
//! let d = izanagi_kit::permissions::parse("geolocation=(), camera=(self \"https://e.x\"), fullscreen=*").unwrap();
//! assert_eq!(d.get("geolocation").unwrap(), Vec::<String>::new());
//! assert_eq!(d.get("camera").unwrap(), ["self", "https://e\x2ex"]);
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed Permissions-Policy header value.
#[derive(Clone, Debug)]
pub struct Permissions {
    /// `(feature, allowlist)` pairs; bare `feature` is `["*"]`,
    /// `feature=()` is `[]`.
    pub policies: Vec<(String, Vec<String>)>,
}

impl Permissions {
    /// Allowlist for `feature`, when present.
    pub fn get(&self, feature: &str) -> Option<&[String]> {
        self.policies
            .iter()
            .find(|(k, _)| k == feature)
            .map(|(_, v)| v.as_slice())
    }

    /// True when `feature` is present and not empty (`()`).
    pub fn allowed(&self, feature: &str) -> bool {
        self.get(feature).is_some_and(|a| !a.is_empty())
    }
}

/// Parse a Permissions-Policy header; `None` when empty or malformed
/// (unbalanced `()`, feature name with bad characters).
pub fn parse(s: &str) -> Option<Permissions> {
    let mut policies = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, list) = if let Some((n, rest)) = part.split_once('=') {
            let rest = rest.trim();
            let list: Vec<String> = if rest == "*" {
                vec!["*".to_string()]
            } else {
                let inner = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')'))?;
                inner
                    .split_whitespace()
                    .map(|t| t.trim_matches('"').trim_matches('\'').to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            };
            (n.trim(), list)
        } else {
            (part, vec!["*".to_string()]) // legacy bare name = allow all
        };
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return None;
        }
        policies.push((name.to_string(), list));
    }
    if policies.is_empty() {
        return None;
    }
    Some(Permissions { policies })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = parse(
            "geolocation=(), camera=(self), usb=(*), fullscreen, payment=(\"https://a\" 'src')",
        )
        .unwrap();
        assert_eq!(d.policies.len(), 5);
        assert_eq!(d.get("geolocation").unwrap(), Vec::<String>::new());
        assert_eq!(d.get("camera").unwrap(), ["self"]);
        assert_eq!(d.get("usb").unwrap(), ["*"]);
        assert_eq!(d.get("fullscreen").unwrap(), ["*"]);
        assert_eq!(d.get("payment").unwrap(), ["https://a", "src"]);
        assert!(!d.allowed("geolocation"));
        assert!(d.allowed("camera"));
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("feature=noparens").is_none());
        assert!(parse("bad name=()").is_none()); // space in name
        assert!(parse(",,,").is_none());
    }
}

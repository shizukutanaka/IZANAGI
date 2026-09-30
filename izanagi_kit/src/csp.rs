//! Content-Security-Policy header (W3C CSP Level 3):
//! `directive-name source…; directive-name source…` — semicolon
//! separated directive list, sources space separated. Known directive
//! names are classified by [`directive_kind`]; unknown ones are kept.
//!
//! ```
//! let d = izanagi_kit::csp::parse("default-src 'self'; script-src 'nonce-x' https:").unwrap();
//! assert_eq!(d.get("script-src").unwrap(), ["'nonce-x'", "https:"]);
//! assert!(d.has("default-src"));
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed CSP header value.
#[derive(Clone, Debug)]
pub struct Csp {
    /// `(directive-name, sources)` pairs in order.
    pub directives: Vec<(String, Vec<String>)>,
}

impl Csp {
    /// Sources of the first directive named `name` (lowercase match).
    pub fn get(&self, name: &str) -> Option<&[String]> {
        self.directives
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_slice())
    }

    /// True when directive `name` is present.
    pub fn has(&self, name: &str) -> bool {
        self.directives.iter().any(|(k, _)| k == name)
    }

    /// True when `default-src` is absent (the spec's most common hole).
    pub fn missing_default_src(&self) -> bool {
        !self.has("default-src")
    }
}

/// Classify `name` against the CSP3 directive list; `"other"` for
/// unknown/deprecated names.
pub fn directive_kind(name: &str) -> &'static str {
    match name {
        "default-src" | "script-src" | "style-src" | "img-src" | "font-src" | "connect-src"
        | "media-src" | "object-src" | "frame-src" | "child-src" | "worker-src"
        | "manifest-src" | "prefetch-src" => "fetch",
        "frame-ancestors" | "base-uri" | "form-action" => "navigation",
        "upgrade-insecure-requests" | "block-all-mixed-content" => "inline",
        "require-trusted-types-for" | "trusted-types" => "trusted-types",
        "sandbox" | "report-uri" | "report-to" | "plugin-types" => "other",
        _ => "other",
    }
}

/// Parse a CSP header; `None` when empty or a directive has no name.
pub fn parse(s: &str) -> Option<Csp> {
    let mut directives = Vec::new();
    for part in s.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut it = part.split_whitespace();
        let name = it.next()?;
        let name = name.to_lowercase();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return None;
        }
        directives.push((name, it.map(ToString::to_string).collect()));
    }
    if directives.is_empty() {
        return None;
    }
    Some(Csp { directives })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = parse("default-src 'self'; img-src * data:; frame-ancestors 'none'; report-uri /r")
            .unwrap();
        assert_eq!(d.directives.len(), 4);
        assert_eq!(d.get("img-src").unwrap(), ["*", "data:"]);
        assert!(!d.missing_default_src());
        assert_eq!(directive_kind("img-src"), "fetch");
        assert_eq!(directive_kind("frame-ancestors"), "navigation");
        assert_eq!(directive_kind("weird-src"), "other");
    }

    #[test]
    fn empty_and_junk() {
        assert!(parse("").is_none());
        assert!(parse(";;;").is_none());
        assert!(parse("bad name here").is_some()); // "bad" is a valid name token
        assert!(parse("'self'").is_none()); // 'source' alone is not a directive name
    }

    #[test]
    fn missing_default() {
        let d = parse("script-src 'self'").unwrap();
        assert!(d.missing_default_src());
    }
}

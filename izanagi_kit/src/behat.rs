//! Behat `behat.yml` / `behat.yml.dist` census.
//!
//! Profile-first YAML: `default:`/`test:`/`prod:` profile keys,
//! `suites:`/`extensions:`/`gherkin:`/`testwork:`/`formatters:`/
//! `translations:`/`import:`/`calls:` sections, plus
//! `Behat\Vendor\Extension`-style `Vendor\Extension:` class keys and
//! context/service references.
//!
//! ```rust
//! let y = b"default:\n    suites:\n        default:\n            contexts:\n                - FeatureContext\n    extensions:\n        Behat\\MinkExtension: ~\n    gherkin: ~\n";
//! assert!(izanagi_kit::behat::detect(y));
//! ```

/// behat.yml census.
#[derive(Debug, Clone)]
pub struct Behat {
    /// `key:`/`key: ~` lines matching known behat keys.
    pub keys: usize,
    /// `Vendor\Extension:` namespaced class keys.
    pub ext_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Top-level/structural behat keys.
const KEYS: &[&str] = &[
    "autoload",
    "bootstrap",
    "cached",
    "calls",
    "contexts",
    "default",
    "definitions",
    "extensions",
    "filters",
    "formatters",
    "gherkin",
    "import",
    "paths",
    "profile",
    "snippets",
    "strict",
    "suites",
    "test",
    "testwork",
    "translations",
];

/// Context/service keys frequently inside suites.
const INNER_KEYS: &[&str] = &[
    "base_url",
    "browser_name",
    "cache",
    "context",
    "contexts",
    "env",
    "filters",
    "formatters",
    "javascript_session",
    "parameter",
    "parameters",
    "paths",
    "services",
    "session",
    "sessions",
    "suites",
    "tags",
    "type",
    "wd_host",
];

fn ykey(l: &str) -> Option<&str> {
    let t = l.trim_start();
    if t.is_empty() || t.starts_with('#') || t.starts_with('-') {
        return None;
    }
    let end = t.find(':')?;
    let k = &t[..end];
    if k.is_empty() {
        return None;
    }
    Some(k.trim())
}

fn is_class_key(k: &str) -> bool {
    k.contains('\\')
}

/// Detect a `behat.yml` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut classes = 0usize;
    for l in t.lines() {
        if let Some(k) = ykey(l) {
            if is_class_key(k) {
                classes += 1;
            } else if KEYS.contains(&k) || INNER_KEYS.contains(&k) {
                keys += 1;
            }
        }
    }
    (keys >= 2 && (classes >= 1 || t.contains("suites:"))) || keys >= 4
}

impl Behat {
    /// Count keys. Returns `None` when the input does not look like a
    /// `behat.yml`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            ext_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = ykey(l) {
                if is_class_key(k) {
                    c.ext_keys += 1;
                } else if KEYS.contains(&k) || INNER_KEYS.contains(&k) {
                    c.keys += 1;
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
        let b = b"# behat.yml\ndefault:\n    autoload: [ features/bootstrap ]\n    suites:\n        web:\n            paths: [ %paths.base%/features ]\n            contexts: [ FeatureContext, MinkContext ]\n            filters:\n                tags: \"@web\"\n        api:\n            contexts: [ ApiContext ]\n    extensions:\n        Behat\\MinkExtension:\n            base_url: http://localhost/\n            sessions:\n                default:\n                    goutte: ~\n                javascript:\n                    selenium2: ~\n        FriendsOfBehat\\SymfonyExtension: ~\n    gherkin:\n        cache: ~\n";
        assert!(detect(b));
        let c = Behat::parse(b).unwrap();
        assert_eq!(c.ext_keys, 2);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"name: x\nversion: 1\n"));
        assert!(!detect(b"key=value\n"));
        assert!(Behat::parse(b"").is_none());
    }
}

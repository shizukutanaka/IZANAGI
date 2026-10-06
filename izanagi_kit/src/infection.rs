//! Infection `infection.json` / `infection.json5` mutation-testing
//! config census.
//!
//! JSON keys: `$schema`, `source.directories`, `timeout`,
//! `logs` (`text`/`html`/`summary`/`json`/`gitlab`/`badge`/`stryker`),
//! `tmpDir`, `phpUnit`/`customPath`/`configDir`, `initialTestsPhpOptions`,
//! `testFramework`, `testFrameworkOptions`, `bootstrap`,
//! `initialTestsRunner`, `mutators`, `minMsi`, `minCoveredMsi`,
//! `msiPrecision`, `ignoreMsiWithNoMutations`, `onlyCovered`,
//! `restrictions`, `uncovered` …
//!
//! ```rust
//! let i = br#"{
//!     "source": {"directories": ["src"]},
//!     "timeout": 10,
//!     "logs": {"text": "infection.log"},
//!     "mutators": {"@default": true},
//!     "minMsi": 60
//! }"#;
//! assert!(izanagi_kit::infection::detect(i));
//! ```

/// infection.json census.
#[derive(Debug, Clone)]
pub struct Infection {
    /// `"key":` lines matching a known infection key.
    pub keys: usize,
    /// `#`/`//` comment lines (json5).
    pub comments: usize,
}

/// Infection config keys (top and second level).
const KEYS: &[&str] = &[
    "$schema",
    "badge",
    "bootstrap",
    "command",
    "configDir",
    "coverage",
    "customPath",
    "debug",
    "directories",
    "excludes",
    "export",
    "gitLab",
    "gitlab",
    "html",
    "ignoreMsiWithNoMutations",
    "ignoreSourceCodeMutationsMap",
    "initialTestsPhpOptions",
    "initialTestsRunner",
    "interceptors",
    "junit",
    "logs",
    "logVerbosity",
    "minCoveredMsi",
    "minMsi",
    "msiPrecision",
    "mutators",
    "onlyCovered",
    "parameters",
    "perMutatorThresholds",
    "phpUnit",
    "phpSpec",
    "profiles",
    "restrictions",
    "skipInitialTests",
    "source",
    "stryker",
    "summary",
    "summaryJson",
    "testFramework",
    "testFrameworkOptions",
    "text",
    "thresholds",
    "timeout",
    "tmpDir",
    "uncovered",
];

fn jkey(l: &str) -> Option<&str> {
    let t = l.trim_start();
    let t = t.strip_prefix('"')?;
    let end = t.find('"')?;
    let key = &t[..end];
    if key.is_empty() {
        return None;
    }
    let rest = t[end + 1..].trim_start();
    if rest.starts_with(':') {
        Some(key)
    } else {
        None
    }
}

/// Detect an `infection.json` config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut n = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Infection {
    /// Count keys. Returns `None` when the input does not look like an
    /// `infection.json`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if let Some(k) = jkey(l) {
                if KEYS.contains(&k) {
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
        let b = br#"{
    "$schema": "vendor/infection/infection/resources/schema.json",
    "source": {
        "directories": [
            "src"
        ],
        "excludes": [
            "src/Generated"
        ]
    },
    "timeout": 10,
    "logs": {
        "text": "infection.log",
        "summary": "summary.log",
        "gitlab": "gitlab.log"
    },
    "mutators": {
        "@default": true,
        "MBString": {
            "settings": {
                "mb_substr": false
            }
        }
    },
    "testFramework": "phpunit",
    "minMsi": 60,
    "minCoveredMsi": 70,
    "ignoreMsiWithNoMutations": true
}
"#;
        assert!(detect(b));
        let c = Infection::parse(b).unwrap();
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(br#"{"name":"x","dependencies":{}}"#));
        assert!(!detect(b"key=value\n"));
        assert!(Infection::parse(b"").is_none());
    }
}

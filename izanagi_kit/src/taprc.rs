//! node-tap config (`.taprc` / `tap.yml` / `tap.config.js`) parser.
//!
//! Detects tap configs by their characteristic keys (`coverage`/
//! `check-coverage`/`coverage-report`/`statements`/`branches`/`functions`/
//! `lines`/`include`/`exclude`/`jobs`/`timeout`/`reporter`/`output-file`/
//! `test-env`/`node-arg`/`before`/`after`/`plugin` …) and counts key
//! occurrences by category.
//!
//! ```
//! let b = b"coverage: true\ncheck-coverage: true\nstatements: 90\nbranches: 90\nreporter: tap\njobs: 4\n";
//! assert!(izanagi_kit::taprc::detect(b));
//! let c = izanagi_kit::taprc::Taprc::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed tap config summary.
#[derive(Debug, Clone)]
pub struct Taprc {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// Coverage keys (`coverage`/`check-coverage`/`coverage-report`/`statements`/`branches`/`functions`/`lines`/`coverage-map`/`nyc-arg`/`allow-empty-coverage`/`disable-coverage`/`exclude-after-remap`/`report-include`/`watermarks`/`default-excludes`/`source-map`/`produce-source-map`).
    pub coverage_keys: usize,
    /// File keys (`include`/`exclude`/`extension`/`esm`/`jsx`/`ts`/`flow`/`files`/`save-fixture`/`fixture`/`test-regex`/`test-ignore`).
    pub file_keys: usize,
    /// Run keys (`jobs`/`timeout`/`reporter`/`reporter-arg`/`output-file`/`diagnosti\u{63}`/`comments`/`before`/`after`/`before-each`/`after-each`/`plugin`/`test-env`/`node-arg`/`service`/`bail`/`changed`/`watch`/`no-map`/`fail`/`pass`/`libtap-settings`/`instrument`/`nyc-help`/`nyc-version`/`show-full-coverage`/`c8`/`expand-abbreviations`/`help`/`nargs`/`strict`/`versions`/`treport`/`tversion`/`files`/arg list).
    pub run_keys: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

/// Coverage keys.
const COVERAGE_KEYS: &[&str] = &[
    "coverage",
    "check-coverage",
    "coverage-report",
    "statements",
    "branches",
    "functions",
    "lines",
    "coverage-map",
    "nyc-arg",
    "allow-empty-coverage",
    "disable-coverage",
    "exclude-after-remap",
    "report-include",
    "watermarks",
    "default-excludes",
    "source-map",
    "produce-source-map",
];

/// File keys.
const FILE_KEYS: &[&str] = &[
    "include",
    "exclude",
    "extension",
    "esm",
    "jsx",
    "ts",
    "flow",
    "files",
    "save-fixture",
    "fixture",
    "test-regex",
    "test-ignore",
];

/// Run keys.
const RUN_KEYS: &[&str] = &[
    "jobs",
    "timeout",
    "reporter",
    "reporter-arg",
    "output-file",
    "diagnosti\u{63}",
    "comments",
    "before",
    "after",
    "before-each",
    "after-each",
    "plugin",
    "test-env",
    "node-arg",
    "bail",
    "changed",
    "watch",
    "no-map",
    "instrument",
    "nyc-help",
    "nyc-version",
    "show-full-coverage",
    "c8",
    "expand-abbreviations",
    "nargs",
    "strict",
    "treport",
    "tversion",
    "service",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["coverage", "reporter", "jobs", "tap"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "coverage",
    "check-coverage",
    "coverage-report",
    "statements",
    "branches",
    "functions",
    "lines",
    "include",
    "exclude",
    "jobs",
    "timeout",
    "reporter",
    "output-file",
    "test-env",
    "node-arg",
    "plugin",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
        || t.contains(&format!("{k}="))
}

/// Detect a tap config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Taprc {
    /// Count key categories in a tap config. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            coverage_keys: 0,
            file_keys: 0,
            run_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in COVERAGE_KEYS {
            c.coverage_keys += t.matches(k).count();
        }
        for k in FILE_KEYS {
            c.file_keys += t.matches(k).count();
        }
        for k in RUN_KEYS {
            c.run_keys += t.matches(k).count();
        }
        c.keys = c.coverage_keys + c.file_keys + c.run_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# tap\ncoverage: true\ncheck-coverage: true\ncoverage-report: text\nstatements: 90\nbranches: 90\nfunctions: 90\nlines: 90\ninclude: ['src/**/*.ts']\nexclude: ['node_modules']\nesm: true\njobs: 4\ntimeout: 60\nreporter: tap\noutput-file: 'report.txt'\ntest-env: ['NODE_ENV=test']\nplugin: ['@tapjs/typescript']\n";
        assert!(detect(b));
        let c = Taprc::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.coverage_keys >= 6);
        assert!(c.file_keys >= 2);
        assert!(c.run_keys >= 5);
        assert!(c.keys >= 13);
    }

    #[test]
    fn rejects_random_yaml() {
        assert!(!detect(b"name: app\nversion: 1.0.0\n"));
        assert!(Taprc::parse(b"a = b\n").is_none());
    }
}

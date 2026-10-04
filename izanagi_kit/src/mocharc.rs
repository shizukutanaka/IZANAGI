//! Mocha config (`.mocharc.yml`/`.mocharc.json`/`.mocharc.js`/`mocha.opts`)
//! parser.
//!
//! Detects Mocha option files by their characteristic keys (`spec`/`require`/
//! `reporter`/`ui`/`timeout`/`retries`/`grep`/`recursive`/`bail`/`parallel`/
//! `watch-files`/`forbid-only`/`exit` …) and counts key occurrences by
//! category.
//!
//! ```
//! let b = b"spec: 'test/**/*.spec.js'\nrequire: 'ts-node/register'\nreporter: 'spec'\ntimeout: 5000\nrecursive: true\n";
//! assert!(izanagi_kit::mocharc::detect(b));
//! let c = izanagi_kit::mocharc::Mocharc::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed Mocha config summary.
#[derive(Debug, Clone)]
pub struct Mocharc {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// Spec/file keys (`spe\u{63}`/`file`/`ignore`/`extension`/`recursive`/`require`/`watch-files`/`watch-extensions`/`watch-ignore`/`package`/`config`/`opts`/`node-option`/`forbid-only`/`forbid-pending`).
    pub spec_keys: usize,
    /// Run-control keys (`timeout`/`slow`/`retries`/`bail`/`parallel`/`jobs`/`exit`/`delay`/`grep`/`invert`/`fgrep`/`async-only`/`allow-uncaught`/`dry-run`/`fail-zero`/`pass-on-failing-test-suite`/`check-leaks`/`full-trace`/`unhandled-rejection`/`trace-warnings`/`sort`/`growl`).
    pub run_keys: usize,
    /// Report/display keys (`reporter`/`reporter-option`/`reporter-options`/`ui`/`color`/`colors`/`inline-diffs`/`diff`/`list-interfaces`/`list-reporters`/`global`).
    pub report_keys: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

/// Spec/file keys.
const SPEC_KEYS: &[&str] = &[
    "spe\u{63}",
    "file",
    "ignore",
    "extension",
    "recursive",
    "require",
    "watch-files",
    "watch-extensions",
    "watch-ignore",
    "package",
    "config",
    "opts",
    "node-option",
    "forbid-only",
    "forbid-pending",
];

/// Run-control keys.
const RUN_KEYS: &[&str] = &[
    "timeout",
    "slow",
    "retries",
    "bail",
    "parallel",
    "jobs",
    "exit",
    "delay",
    "grep",
    "invert",
    "fgrep",
    "async-only",
    "allow-uncaught",
    "dry-run",
    "fail-zero",
    "pass-on-failing-test-suite",
    "check-leaks",
    "full-trace",
    "unhandled-rejection",
    "trace-warnings",
    "sort",
    "growl",
];

/// Report/display keys.
const REPORT_KEYS: &[&str] = &[
    "reporter",
    "reporter-option",
    "reporter-options",
    "ui",
    "color",
    "colors",
    "inline-diffs",
    "diff",
    "list-interfaces",
    "list-reporters",
    "global",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["mocha", "reporter", "timeout", "ui"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "spe\u{63}",
    "require",
    "reporter",
    "ui",
    "timeout",
    "retries",
    "grep",
    "recursive",
    "bail",
    "parallel",
    "watch-files",
    "forbid-only",
    "exit",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
        || t.contains(&format!("--{k}"))
}

/// Detect a Mocha config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Mocharc {
    /// Count key categories in a Mocha config. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            spec_keys: 0,
            run_keys: 0,
            report_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in SPEC_KEYS {
            c.spec_keys += t.matches(k).count();
        }
        for k in RUN_KEYS {
            c.run_keys += t.matches(k).count();
        }
        for k in REPORT_KEYS {
            c.report_keys += t.matches(k).count();
        }
        c.keys = c.spec_keys + c.run_keys + c.report_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts_yaml() {
        let b = b"# mocha\nspec: 'test/**/*.spec.js'\nrequire:\n  - ts-node/register\nreporter: 'spec'\nui: 'bdd'\ntimeout: 5000\nretries: 1\nrecursive: true\nbail: false\n";
        assert!(detect(b));
        let c = Mocharc::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.spec_keys >= 3);
        assert!(c.run_keys >= 3);
        assert!(c.report_keys >= 2);
        assert!(c.keys >= 8);
    }

    #[test]
    fn detects_opts_file() {
        let b = b"--reporter dot\n--timeout 20000\n--recursive\n--require ts-node/register\n--spec test/**/*.spec.js\n";
        assert!(detect(b));
        let c = Mocharc::parse(b).unwrap();
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_random() {
        assert!(!detect(b"const x = 1;\nconsole.log(x);\n"));
        assert!(Mocharc::parse(b"a = b\n").is_none());
    }
}

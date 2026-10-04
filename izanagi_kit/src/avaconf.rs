//! AVA config (`ava.config.js` / `package.json` `"ava"` block) parser.
//!
//! Detects AVA configs by their characteristic keys (`files`/`sources`/
//! `match`/`cache`/`concurrency`/`failFast`/`tap`/`verbose`/`snapshotDir`/
//! `babel`/`typescript`/`extensions`/`require`/`nodeArguments`/
//! `environmentVariables`/`watchMode`/`ignoredByWatcher` …) and counts
//! key occurrences by category.
//!
//! ```
//! let b = b"export default {\n  files: ['test/**/*.js'],\n  sources: ['src/**/*.js'],\n  cache: true,\n  concurrency: 5,\n  failFast: false,\n  tap: true,\n};\n";
//! assert!(izanagi_kit::avaconf::detect(b));
//! let c = izanagi_kit::avaconf::Ava::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed AVA config summary.
#[derive(Debug, Clone)]
pub struct Ava {
    /// Recognized config key occurrences.
    pub keys: usize,
    /// File/source keys (`files`/`sources`/`match`/`ignoredByWatcher`/`snapshotDir`/`nonSemVerExperiments`).
    pub file_keys: usize,
    /// Run-control keys (`concurrency`/`failFast`/`serial`/`timeout`/`workerArgv`/`environmentVariables`/`utilizeParallelBuilds`/`nodeArguments`/`watchMode`/`ignoreChanges`).
    pub run_keys: usize,
    /// Output/transpile keys (`tap`/`verbose`/`babel`/`compileEnhancements`/`extensions`/`require`/`typescript`/`provider`/`commonJS`/`flow`/`enhancements`).
    pub out_keys: usize,
    /// `//`/`/*`/`#` comment lines.
    pub comments: usize,
}

/// File/source keys.
const FILE_KEYS: &[&str] = &[
    "files",
    "sources",
    "match",
    "ignoredByWatcher",
    "snapshotDir",
    "nonSemVerExperiments",
];

/// Run-control keys.
const RUN_KEYS: &[&str] = &[
    "concurrency",
    "failFast",
    "serial",
    "timeout",
    "workerArgv",
    "environmentVariables",
    "utilizeParallelBuilds",
    "nodeArguments",
    "watchMode",
    "ignoreChanges",
];

/// Output/transpile keys.
const OUT_KEYS: &[&str] = &[
    "tap",
    "verbose",
    "babel",
    "compileEnhancements",
    "extensions",
    "require",
    "typescript",
    "provider",
    "commonJS",
    "flow",
    "enhancements",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["ava", "files", "sources", "cache"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "files",
    "sources",
    "match",
    "cache",
    "concurrency",
    "failFast",
    "tap",
    "verbose",
    "snapshotDir",
    "babel",
    "typescript",
    "extensions",
    "nodeArguments",
    "watchMode",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("\"{k}\""))
        || t.contains(&format!("{k}:"))
        || t.contains(&format!("{k} :"))
        || t.contains(&format!("{k} ="))
}

/// Detect an AVA config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Ava {
    /// Count key categories in an AVA config. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            file_keys: 0,
            run_keys: 0,
            out_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("//") || tr.starts_with("/*") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in FILE_KEYS {
            c.file_keys += t.matches(k).count();
        }
        for k in RUN_KEYS {
            c.run_keys += t.matches(k).count();
        }
        for k in OUT_KEYS {
            c.out_keys += t.matches(k).count();
        }
        c.keys = c.file_keys + c.run_keys + c.out_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"// ava\nexport default {\n  files: ['test/**/*.test.js'],\n  sources: ['src/**/*.js'],\n  ignoredByWatcher: ['tmp'],\n  cache: true,\n  concurrency: 5,\n  failFast: true,\n  serial: false,\n  timeout: '30s',\n  tap: true,\n  verbose: true,\n  babel: false,\n  extensions: ['js', 'ts'],\n  require: ['ts-node/register'],\n  nodeArguments: ['--trace-warnings'],\n};\n";
        assert!(detect(b));
        let c = Ava::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.file_keys >= 2);
        assert!(c.run_keys >= 3);
        assert!(c.out_keys >= 4);
        assert!(c.keys >= 9);
    }

    #[test]
    fn detects_package_ava() {
        let b = br#"{"ava": {"files": ["t/*.js"], "tap": true, "concurrency": 2, "typescript": {"compile": false}}}"#;
        assert!(detect(b));
        let c = Ava::parse(b).unwrap();
        assert!(c.keys >= 3);
    }

    #[test]
    fn rejects_random_js() {
        assert!(!detect(b"const x = 1;\nconsole.log(x);\n"));
        assert!(Ava::parse(b"a = b\n").is_none());
    }
}

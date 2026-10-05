//! Bandit `.bandit` / `bandit.yaml` / `[bandit]` parser.
//!
//! Detects Bandit security-linter config by `[bandit]` / `tests:` /
//! `skips:` / `exclude_dirs:` / `profile` keys plus `B1xx`–`B7xx` test
//! IDs, and counts structure.
//!
//! ```
//! let b = b"[bandit]\nskips = B101,B102\nexclude_dirs = tests\n[bandit.plugins]\nB101: assert_used\n";
//! assert!(izanagi_kit::banditconf::detect(b));
//! let c = izanagi_kit::banditconf::Bandit::parse(b).unwrap();
//! assert!(c.codes >= 2);
//! ```

/// Parsed .bandit summary.
#[derive(Debug, Clone)]
pub struct Bandit {
    /// Recognized key occurrences.
    pub keys: usize,
    /// `B1xx`–`B7xx` test-ID occurrences.
    pub codes: usize,
    /// `[bandit]`/`[bandit.*]`/`tests:`/`skips:` section markers.
    pub sections: usize,
    /// `key = value`/`key: value` lines.
    pub assignments: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Known option keys.
const KEYS: &[&str] = &[
    "tests",
    "skips",
    "exclude_dirs",
    "include",
    "exclude",
    "profile",
    "output_format",
    "recursive",
    "targets",
    "severity",
    "confidence",
    "number",
    "agg_lines",
    "context_lines",
    "exit_zero",
    "verbose",
    "quiet",
    "ignore-nose\u{63}",
    "baseline",
    "ini-path",
    "stdin-fd",
    "hardcoded_password_string",
    "hardcoded_password_funcarg",
    "hardcoded_password_default",
    "subprocess_popen_with_shell",
    "exec_used",
    "assert_used",
    "sql_statements",
    "request_with_no_cert_validation",
    "request_with_hardcoded_password",
    "yaml_load",
    "blacklist_imports",
    "blacklist_import_fun\u{63}",
    "blacklist_calls",
    "ssh_no_host_key_verification",
    "os_exe\u{63}",
    "set_permissions_file",
    "mark_secure",
    "extension",
    "B101",
    "B102",
    "B103",
    "B104",
    "B105",
    "B106",
    "B107",
    "B108",
    "B109",
    "B110",
    "B111",
    "B112",
    "B113",
    "B201",
    "B301",
    "B302",
    "B303",
    "B304",
    "B305",
    "B306",
    "B307",
    "B308",
    "B310",
    "B311",
    "B312",
    "B313",
    "B314",
    "B315",
    "B316",
    "B317",
    "B318",
    "B319",
    "B320",
    "B321",
    "B401",
    "B402",
    "B403",
    "B404",
    "B405",
    "B406",
    "B407",
    "B408",
    "B409",
    "B410",
    "B411",
    "B412",
    "B413",
    "B501",
    "B502",
    "B503",
    "B504",
    "B505",
    "B506",
    "B507",
    "B601",
    "B602",
    "B603",
    "B604",
    "B605",
    "B606",
    "B607",
    "B608",
    "B609",
    "B610",
    "B611",
    "B701",
    "B702",
    "B703",
    "B704",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Bandit config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if t.contains("[bandit]") || t.contains("[bandit.") {
        return true;
    }
    let hits = KEYS.iter().filter(|k| key_present(t, k)).count();
    hits >= 3
}

impl Bandit {
    /// Count categories. Returns `None` when the input does not look like
    /// a Bandit config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            codes: 0,
            sections: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
            } else if (tr.starts_with('[') && tr.ends_with(']'))
                || (tr.ends_with(':') && !tr.starts_with('-'))
            {
                c.sections += 1;
            } else if tr.contains('=') || tr.contains(':') {
                c.assignments += 1;
            }
        }
        for k in KEYS {
            c.keys += t.matches(k).count();
        }
        let bytes = t.as_bytes();
        let mut i = 0;
        while i + 3 < bytes.len() {
            if bytes[i] == b'B' && bytes[i + 1].is_ascii_digit() {
                let d1 = bytes[i + 1];
                let d2 = bytes[i + 2];
                let d3 = bytes[i + 3];
                if (b'1'..=b'7').contains(&d1) && d2.is_ascii_digit() && d3.is_ascii_digit() {
                    let boundary =
                        i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
                    if boundary {
                        c.codes += 1;
                        i += 4;
                        continue;
                    }
                }
            }
            i += 1;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[bandit]\nskips = B101,B102\nexclude_dirs = tests,fixtures\n[bandit.plugins]\nB101: assert_used\nB105: hardcoded_password_string\n";
        assert!(detect(b));
        let c = Bandit::parse(b).unwrap();
        assert!(c.sections >= 2);
        assert!(c.codes >= 4);
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[main]\nfoo = bar\n"));
        assert!(Bandit::parse(b"").is_none());
    }
}

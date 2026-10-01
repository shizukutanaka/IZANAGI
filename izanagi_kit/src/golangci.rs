//! golangci-lint `.golangci.yml` parser.
//!
//! Detects the tool's characteristic top-level sections (`linters:`/
//! `linters-settings:`/`run:`/`issues:`/`output:`/`formatters:`) and counts
//! enabled/disabled linter entries plus `linters-settings` keys.
//!
//! ```
//! let b = b"run:\n  timeout: 5m\nlinters:\n  enable:\n    - govet\n    - staticcheck\n  disable:\n    - errcheck\nlinters-settings:\n  govet:\n    enable-all: true\n";
//! assert!(izanagi_kit::golangci::detect(b));
//! let c = izanagi_kit::golangci::Golangci::parse(b).unwrap();
//! assert_eq!(c.enabled, 2);
//! assert_eq!(c.disabled, 1);
//! ```

/// Parsed .golangci.yml summary.
#[derive(Debug, Clone)]
pub struct Golangci {
    /// Top-level sections present.
    pub sections: usize,
    /// `- name` entries under `linters:` → `enable:`.
    pub enabled: usize,
    /// `- name` entries under `linters:` → `disable:`.
    pub disabled: usize,
    /// Keys under `linters-settings:`.
    pub settings: usize,
    /// Keys under `issues:`/`output:`/`run:`/`formatters:` (non-linters top sections).
    pub misc_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Known top-level sections.
const SECTIONS: &[&str] = &[
    "run",
    "issues",
    "output",
    "linters",
    "linters-settings",
    "severity",
    "service",
    "formatters",
    "formatters-settings",
    "version",
];

/// Detect a `.golangci.yml`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("linters-settings:")
        || t.contains("golangci")
        || (t.contains("linters:")
            && (t.contains("enable:") || t.contains("disable:") || t.contains("presets:"))
            && (t.contains("run:") || t.contains("issues:") || t.contains("output:")))
}

impl Golangci {
    /// Count sections/linter entries in a `.golangci.yml`. Returns `None`
    /// when the input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            enabled: 0,
            disabled: 0,
            settings: 0,
            misc_keys: 0,
            comments: 0,
        };
        let mut section = "";
        let mut subsection = "";
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = l.len() - tr.len();
            if indent == 0 {
                if let Some(key) = tr.strip_suffix(':') {
                    section = key.trim();
                    subsection = "";
                    if SECTIONS.contains(&section) {
                        c.sections += 1;
                    }
                    continue;
                }
                if tr.ends_with('{') || tr.contains(':') {
                    c.misc_keys += 1;
                }
                continue;
            }
            if tr.starts_with("- ") {
                let item = tr.trim_start_matches('-').trim();
                if section == "linters" && subsection == "enable" && !item.is_empty() {
                    c.enabled += 1;
                } else if section == "linters" && subsection == "disable" && !item.is_empty() {
                    c.disabled += 1;
                }
                continue;
            }
            if let Some(key) = tr.strip_suffix(':') {
                let k = key.trim();
                if section == "linters" {
                    subsection = k;
                } else if section == "linters-settings" {
                    c.settings += 1;
                    subsection = k;
                } else {
                    subsection = k;
                    c.misc_keys += 1;
                }
                continue;
            }
            if tr.contains(':') {
                if section == "linters-settings" {
                    c.settings += 1;
                } else if section == "linters" {
                    // fast: true / presets handled as keys
                } else {
                    c.misc_keys += 1;
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
    fn detects_and_counts() {
        let b = b"# golangci\nrun:\n  timeout: 5m\n  go: \"1\"\n  concurrency: 4\nlinters:\n  enable-all: true\n  disable-all: false\n  enable:\n    - govet\n    - staticcheck\n    - revive\n  disable:\n    - errcheck\n    - gochecknoinits\n  presets:\n    - bugs\nlinters-settings:\n  govet:\n    enable-all: true\n  revive:\n    min-confidence: 0\n  staticcheck:\n    checks: [\"all\"]\nissues:\n  exclude-use-default: false\n  max-issues-per-linter: 0\noutput:\n  formats:\n    - format: text\n";
        assert!(detect(b));
        let c = Golangci::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert_eq!(c.enabled, 3);
        assert_eq!(c.disabled, 2);
        assert_eq!(c.settings, 6);
        assert!(c.sections >= 4);
        assert!(c.misc_keys >= 5);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(!detect(b"kind: ConfigMap\ndata:\n"));
        assert!(Golangci::parse(b"a: 1\n").is_none());
    }
}

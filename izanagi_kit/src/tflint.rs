//! TFLint `.tflint.hcl` census.
//!
//! `plugin "<name>" { enabled/source/version }` blocks,
//! `rule "<name>" { enabled }` blocks, and `config {` settings:
//! `module_inspect_mode`, `call_module_type`, `force`,
//! `disabled_by_default`, `plugin_dir`, `ignore_rule`,
//! `exclude_paths`, `variables`.
//!
//! ```rust
//! let k = b"plugin \"aws\" {\n  enabled = true\n  version = \"0.30.0\"\n  source  = \"github.com/terraform-linters/tflint-ruleset-aws\"\n}\n";
//! assert!(izanagi_kit::tflint::detect(k));
//! ```

/// .tflint.hcl census.
#[derive(Debug, Clone)]
pub struct Tflint {
    /// `plugin` blocks.
    pub plugins: usize,
    /// `rule` blocks.
    pub rules: usize,
    /// `config`/`ignore_*`/`exclude_*` blocks.
    pub blocks: usize,
    /// `key = value` assignments.
    pub assignments: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const STRONG_KEYS: &[&str] = &[
    "call_module_type",
    "module_inspect_mode",
    "disabled_by_default",
    "plugin_dir",
    "ignore_rule",
    "exclude_paths",
    "only_mode",
    "force",
];

fn code_of(s: &str) -> &str {
    let mut s = s;
    if let Some(i) = s.find("//") {
        s = &s[..i];
    }
    if let Some(i) = s.find('#') {
        s = &s[..i];
    }
    s.trim()
}

/// Detect .tflint.hcl content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut plugin_or_rule = false;
    let mut strong = false;
    for line in t.lines() {
        let c = code_of(line);
        if c.is_empty() {
            continue;
        }
        if c.starts_with("plugin \"") || c.starts_with("rule \"") {
            plugin_or_rule = true;
        }
        if STRONG_KEYS.iter().any(|k| c.starts_with(k)) {
            strong = true;
        }
    }
    plugin_or_rule || strong
}

impl Tflint {
    /// Census a .tflint.hcl buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            plugins: 0,
            rules: 0,
            blocks: 0,
            assignments: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') || s.starts_with("/*") {
                c.comments += 1;
                continue;
            }
            let code = code_of(s);
            if code.is_empty() {
                continue;
            }
            if code.starts_with("plugin \"") {
                c.plugins += 1;
            } else if code.starts_with("rule \"") {
                c.rules += 1;
            } else if code.ends_with('{')
                && (code.starts_with("config")
                    || code.starts_with("ignore_")
                    || code.starts_with("exclude_"))
            {
                c.blocks += 1;
            }
            if code.contains('=') && !code.ends_with('{') {
                c.assignments += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tflint() {
        let b = b"plugin \"aws\" {\n  enabled = true\n  version = \"0.30.0\"\n  source  = \"github.com/terraform-linters/tflint-ruleset-aws\"\n}\nrule \"terraform_deprecated_syntax\" {\n  enabled = true\n}\n";
        assert!(detect(b));
        let c = Tflint::parse(b).unwrap();
        assert_eq!(c.plugins, 1);
        assert_eq!(c.rules, 1);
    }

    #[test]
    fn detects_config_only() {
        let b = b"config {\n  call_module_type = \"all\"\n  disabled_by_default = false\n}\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_plain_hcl() {
        assert!(!detect(
            b"terraform {\n  required_version = \">= 1.0\"\n}\nvariable \"x\" {}\n"
        ));
        // Comment-only mention.
        assert!(!detect(b"# plugin \"aws\" {}\nvariable \"x\" {}\n"));
    }
}

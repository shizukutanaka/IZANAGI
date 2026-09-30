//! ktlint `.editorconfig`-style properties census.
//!
//! ktlint is configured through `.editorconfig`: `[*.{kt,kts}]`
//! sections plus `ktlint_*` properties (`ktlint_code_style`,
//! `ktlint_standard_*`, `ktlint_experimental`,
//! `ktlint_function_signature_*`, `ktlint_ignore_back_ticked_identifier`,
//! `ktlint_disabled_rules`/`disabled_rules`), IntelliJ `ij_kotlin_*`/
//! `ij_kt_*` keys, and generic editorconfig keys (`indent_size`,
//! `max_line_length`, `insert_final_newline`…).
//!
//! ```rust
//! let c = izanagi_kit::ktlint::Ktlint::parse(b"[*.{kt,kts}]\nktlint_code_style = ktlint_official\n").unwrap();
//! assert_eq!(c.ktlint_props, 1);
//! ```

/// ktlint editorconfig census.
#[derive(Debug, Clone)]
pub struct Ktlint {
    /// `[...]` section headers (`[*.{kt,kts}]` etc).
    pub sections: usize,
    /// `ktlint_*`/`ktlint-*` properties.
    pub ktlint_props: usize,
    /// `ij_*` IntelliJ properties.
    pub intellij_props: usize,
    /// Other `key = value` editorconfig properties.
    pub editorconfig_props: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like a ktlint-flavoured editorconfig file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if let Some((k, _)) = s.split_once('=') {
            let k = k.trim();
            if k.starts_with("ktlint_")
                || k.starts_with("ktlint-")
                || k.starts_with("ij_kotlin")
                || k.starts_with("ij_kt")
                || k == "disabled_rules"
            {
                hits += 1;
            }
        }
    }
    hits >= 1
}

impl Ktlint {
    /// Parse an editorconfig-style ktlint config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            ktlint_props: 0,
            intellij_props: 0,
            editorconfig_props: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                continue;
            }
            if let Some((k, _)) = s.split_once('=') {
                let k = k.trim();
                if k.starts_with("ktlint_") || k.starts_with("ktlint-") || k == "disabled_rules" {
                    c.ktlint_props += 1;
                } else if k.starts_with("ij_") {
                    c.intellij_props += 1;
                } else {
                    c.editorconfig_props += 1;
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
    fn parses_ktlint_editorconfig() {
        let b = concat!(
            "root = true\n",
            "# ktlint\n",
            "[*.{kt,kts}]\n",
            "indent_size = 4\n",
            "max_line_length = 140\n",
            "insert_final_newline = true\n",
            "ktlint_code_style = ktlint_official\n",
            "ktlint_standard_no-wildcard-imports = enabled\n",
            "ktlint_function_signature_rule_force_multiline_when_parameter_count_greater_or_equal_than = 2\n",
            "ktlint_experimental = enabled\n",
            "ij_kotlin_align_multiline_parameters = true\n",
            "ij_kotlin_allow_trailing_comma = true\n",
        );
        let c = Ktlint::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.ktlint_props, 4);
        assert_eq!(c.intellij_props, 2);
        assert_eq!(c.editorconfig_props, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Ktlint::parse(b"[*]\nindent_size = 2").is_none());
    }
}

//! RuboCop `.rubocop.yml` / `.rubocop_todo.yml` census.
//!
//! RuboCop config is YAML: `require:`/`plugins:`/`inherit_from:`/
//! `inherit_gem:`/`inherit_mode:`/`AllCops:` plus per-cop
//! `Department/CopName:` blocks (`Style/FrozenStringLiteralComment:`)
//! holding `Enabled:`/`Exclude:`/`Include:`/`Severity:`/
//! `AllowedMethods:`/`Max:`/`EnforcedStyle:` entries.
//!
//! ```rust
//! let c = izanagi_kit::rubocop::Rubocop::parse(b"AllCops:\n  NewCops: enable\nStyle/FrozenStringLiteralComment:\n  Enabled: false\n").unwrap();
//! assert_eq!(c.cops, 1);
//! ```

/// `.rubocop.yml` census.
#[derive(Debug, Clone)]
pub struct Rubocop {
    /// `Department/Cop:` section headers.
    pub cops: usize,
    /// Other `key:`/`key: value` settings.
    pub settings: usize,
    /// `- ` items inside `Exclude:` blocks.
    pub excludes: usize,
    /// `require`/`plugins`/`inherit_*` entries.
    pub requires: usize,
    /// Other `- ` list items (`Include:`/`AllowedMethods:`…).
    pub listitems: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const REQUIRE_KEYS: &[&str] = &[
    "require",
    "plugins",
    "inherit_from",
    "inherit_gem",
    "inherit_mode",
];

fn is_cop(k: &str) -> bool {
    let mut it = k.split('/');
    let mut n = 0usize;
    for seg in &mut it {
        if seg.is_empty() || !seg.bytes().next().unwrap_or(0).is_ascii_uppercase() {
            return false;
        }
        if !seg.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_') {
            return false;
        }
        n += 1;
    }
    n >= 2
}

/// Whether the buffer looks like a RuboCop config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if let Some((k, _)) = s.split_once(':') {
            let k = k.trim();
            if k == "AllCops" || REQUIRE_KEYS.contains(&k) || is_cop(k) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

impl Rubocop {
    /// Parse a `.rubocop.yml` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            cops: 0,
            settings: 0,
            excludes: 0,
            requires: 0,
            listitems: 0,
            comments: 0,
        };
        let mut excl_indent: Option<usize> = None;
        let mut req_indent: Option<usize> = None;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let ind = l.len() - l.trim_start().len();
            if s.strip_prefix("- ").is_some() || s == "-" {
                match (excl_indent, req_indent) {
                    (Some(ei), _) if ind > ei => c.excludes += 1,
                    (_, Some(ri)) if ind > ri => c.requires += 1,
                    _ => c.listitems += 1,
                }
                continue;
            }
            if excl_indent.is_some_and(|ei| ind <= ei) {
                excl_indent = None;
            }
            if req_indent.is_some_and(|ri| ind <= ri) {
                req_indent = None;
            }
            if let Some((k, v)) = s.split_once(':') {
                let k = k.trim();
                let v = v.trim();
                if is_cop(k) {
                    c.cops += 1;
                } else if REQUIRE_KEYS.contains(&k) {
                    c.settings += 1;
                    if v.is_empty() {
                        req_indent = Some(ind);
                    } else {
                        c.requires += 1;
                    }
                } else {
                    c.settings += 1;
                    if k == "Exclude" && v.is_empty() {
                        excl_indent = Some(ind);
                    }
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
    fn parses_rubocop() {
        let b = concat!(
            "# rubocop\n",
            "require: rubocop-rspec\n",
            "inherit_from: .rubocop_todo.yml\n",
            "AllCops:\n",
            "  NewCops: enable\n",
            "  TargetRubyVersion: 3.2\n",
            "  Exclude:\n",
            "    - 'vendor/**/*'\n",
            "    - 'db/schema.rb'\n",
            "Style/FrozenStringLiteralComment:\n",
            "  Enabled: false\n",
            "Metrics/BlockLength:\n",
            "  Exclude:\n",
            "    - 'spec/**/*'\n",
            "  AllowedMethods:\n",
            "    - describe\n",
            "Layout/LineLength:\n",
            "  Max: 120\n",
        );
        let c = Rubocop::parse(b.as_bytes()).unwrap();
        assert_eq!(c.cops, 3);
        assert_eq!(c.requires, 2);
        assert_eq!(c.excludes, 3);
        assert_eq!(c.listitems, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Rubocop::parse(b"foo: bar\nbaz: qux").is_none());
    }
}

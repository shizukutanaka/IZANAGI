//! Junos configuration census — `set`/`delete` command form and
//! hierarchical `{ ... ; }` form.
//!
//! The set-format uses `set <hierarchy> <statement>`, plus `delete`,
//! `deactivate`, `rename`, `replace`, `edit`, `top` and comments in
//! `/* ... */` or `# ...`. The hierarchical form uses `name {` blocks
//! terminated by `}`/`}`-with-`;` and `key value;` leaves. `parse` counts
//! verbs, distinct top-level hierarchies and statement leaves.
//!
//! ```rust
//! let j = concat!(
//!     "set system host-name r1\n",
//!     "set interfaces ge-0/0/0 unit 0 family inet address 10/24\n",
//!     "set protocols bgp group ebgp peer-as 65001\n",
//!     "deactivate protocols isis\n",
//! );
//! let c = izanagi_kit::junos::Junos::parse(j.as_bytes()).unwrap();
//! assert_eq!(c.sets, 3);
//! assert_eq!(c.top_levels, 3);
//! ```

/// Junos configuration census.
#[derive(Debug, Clone)]
pub struct Junos {
    /// `set ` lines.
    pub sets: usize,
    /// `delete`/`deactivate`/`rename`/`replace`/`edit`/`top`/`annotate` lines.
    pub other_verbs: usize,
    /// Distinct top-level hierarchy names (`system`, `interfaces`, …).
    pub top_levels: usize,
    /// Leaf statements (`key value;` lines in braced form, or set lines).
    pub leaves: usize,
    /// `{`/`}` block opens in hierarchical form.
    pub blocks: usize,
    /// `/* ... */` or `#` comment lines.
    pub comments: usize,
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a Junos configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut sets = 0usize;
    let mut brace_pair = false;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("set ") {
            sets += 1;
        }
        if s.ends_with('{') && t.contains('}') {
            brace_pair = true;
        }
    }
    sets >= 2 || (brace_pair && (t.contains("system") || t.contains("interfaces")))
}

impl Junos {
    /// Parse a Junos configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sets: 0,
            other_verbs: 0,
            top_levels: 0,
            leaves: 0,
            blocks: 0,
            comments: 0,
        };
        let mut tops: Vec<&str> = Vec::new();
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with("/*") || s.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if let Some(rest) = s.strip_prefix("set ") {
                c.sets += 1;
                c.leaves += 1;
                if let Some(top) = rest.split_whitespace().next() {
                    if !tops.contains(&top) {
                        tops.push(top);
                    }
                }
                continue;
            }
            for verb in [
                "delete ",
                "deactivate ",
                "rename ",
                "replace:",
                "replace ",
                "edit ",
                "top",
                "annotate ",
                "activate ",
                "insert ",
                "copy ",
            ] {
                if s.starts_with(verb) {
                    c.other_verbs += 1;
                    break;
                }
            }
            if s.ends_with('{') {
                c.blocks += 1;
            }
            if s.ends_with(';') {
                c.leaves += 1;
            }
        }
        c.top_levels = tops.len();
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_form() {
        let b = concat!(
            "set system host-name r1\n",
            "set system services ssh\n",
            "set interfaces ge-0/0/0 unit 0 family inet address 10/24\n",
            "deactivate interfaces ge-0/0/1\n",
            "# comment\n",
        );
        let c = Junos::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sets, 3);
        assert_eq!(c.top_levels, 2);
        assert_eq!(c.other_verbs, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn parses_braced_form() {
        let b = concat!(
            "system {\n",
            "    host-name r1;\n",
            "    services {\n",
            "        ssh;\n",
            "    }\n",
            "}\n",
            "interfaces {\n",
            "    ge-0/0/0 {\n",
            "        unit 0;\n",
            "    }\n",
            "}\n",
        );
        let c = Junos::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sets, 0);
        assert_eq!(c.blocks, 4);
        assert_eq!(c.leaves, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Junos::parse(b"hello").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

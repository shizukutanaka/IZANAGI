//! Carvel `ytt` template census.
//!
//! ytt annotates YAML with `#@` Starlark directives (`#@data/values`,
//! `#@ load(...)`, `#@ if`, `#@ for`, `#@ end`, `#@ template`,
//! `#@ def`, `@ytt:` references). Detected by two-or-more `#@`
//! directives including a definitive ytt marker.
//!
//! ```rust
//! let k = b"#@data/values\n---\n#@ if data.values.enabled:\nname: x\n#@ end\n";
//! assert!(izanagi_kit::ytt::detect(k));
//! ```

/// ytt template census.
#[derive(Debug, Clone)]
pub struct Ytt {
    /// `#@` directive lines.
    pub directives: usize,
    /// `data/values` or `@ytt:data.values` usages.
    pub values: usize,
    /// `load(...)` imports.
    pub loads: usize,
    /// `---` document separators.
    pub documents: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comments that are not directives.
    pub comments: usize,
}

fn is_directive(s: &str) -> bool {
    s.starts_with("#@") || s.starts_with("@")
}

fn has_strong_marker(t: &str) -> bool {
    t.lines().any(|l| {
        let s = l.trim_start();
        (s.starts_with("#@")
            && (s.contains("data/values")
                || s.contains("load(")
                || s.contains("template")
                || s.contains("text/")))
            || s.contains("@ytt:")
    })
}

/// Detect ytt-annotated content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let directives = t
        .lines()
        .filter(|l| {
            let s = l.trim_start();
            is_directive(s) && !s.starts_with("# @")
        })
        .count();
    directives >= 2 && has_strong_marker(t)
}

impl Ytt {
    /// Census an ytt buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            directives: 0,
            values: 0,
            loads: 0,
            documents: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim_start();
            let tr = s.trim_end();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("#@") || tr.starts_with('@') {
                c.directives += 1;
                if tr.contains("data/values") || tr.contains("data.values") {
                    c.values += 1;
                }
                if tr.contains("load(") {
                    c.loads += 1;
                }
                continue;
            }
            if tr == "---" || tr.starts_with("--- ") {
                c.documents += 1;
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.contains('@') && tr.contains("@ytt:") {
                c.values += 1;
            }
            if tr.contains(':') {
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_template() {
        let b = b"#@data/values\n---\n#@ if data.values.enabled:\nname: x\n#@ end\n";
        assert!(detect(b));
        let c = Ytt::parse(b).unwrap();
        assert_eq!(c.directives, 3);
        assert_eq!(c.documents, 1);
    }

    #[test]
    fn detects_overlay() {
        let b = b"#@ load(\"@ytt:overlay\", \"overlay\")\n#@overlay/match by=overlay.all\n---\nmetadata:\n  name: x\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(!detect(b"key: value\nlist:\n- a\n"));
        // A single stray '#@' comment with no marker.
        assert!(!detect(b"#@ note\nkey: value\n"));
    }
}

//! Magefile (`mage`) Go build target census.
//!
//! A Magefile is a Go file guarded by the `mage` build tag
//! (`//go:build mage` / `// +build mage`) and/or importing
//! `github.com/magefile/mage/mg`. Exported `func` declarations in
//! `package main` become `mage` targets.
//!
//! ```rust
//! let k = b"//go:build mage\n\npackage main\n\nimport \"fmt\"\n\n// Build builds it.\nfunc Build() { fmt.Println(1) }\n";
//! assert!(izanagi_kit::magefile::detect(k));
//! ```

use crate::textutil::strip_bom;
/// Magefile census.
#[derive(Debug, Clone)]
pub struct Magefile {
    /// Top-level `func` declarations (potential targets).
    pub funcs: usize,
    /// `mg.Deps`/`mg.SerialDeps` invocations.
    pub mg_calls: usize,
    /// `import` lines.
    pub imports: usize,
    /// `//` comment lines.
    pub comments: usize,
    /// Whether a `Default` target exists.
    pub has_default: bool,
}

fn strip_line_comment(s: &str) -> &str {
    match s.find("//") {
        Some(i) => s[..i].trim_end(),
        None => s,
    }
}

/// Detect a Magefile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut has_tag = false;
    let mut has_mg = false;
    let mut has_main = false;
    for line in t.lines() {
        let s = line.trim();
        if s.starts_with("//go:build mage") || s == "// +build mage" {
            has_tag = true;
        }
        if s.contains("github.com/magefile/mage/") {
            has_mg = true;
        }
        if s == "package main" {
            has_main = true;
        }
    }
    has_main && (has_tag || has_mg)
}

impl Magefile {
    /// Census a Magefile buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            funcs: 0,
            mg_calls: 0,
            imports: 0,
            comments: 0,
            has_default: false,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if s.starts_with("import") || s.starts_with('"') && s.contains("github.com/") {
                c.imports += 1;
            }
            if let Some(rest) = s.strip_prefix("func ") {
                c.funcs += 1;
                let name: String = rest
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                    .collect();
                if name == "Default" {
                    c.has_default = true;
                }
            }
            if strip_line_comment(s).contains("mg.Deps") || s.contains("mg.SerialDeps") {
                c.mg_calls += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_tag() {
        let b = b"//go:build mage\n\npackage main\n\nfunc Build() {}\nfunc Test() {}\n";
        assert!(detect(b));
        let c = Magefile::parse(b).unwrap();
        assert_eq!(c.funcs, 2);
    }

    #[test]
    fn detects_by_import() {
        let b = b"package main\n\nimport (\n\t\"github.com/magefile/mage/mg\"\n)\n\nfunc Install() { mg.Deps(Build) }\nfunc Build() {}\n";
        assert!(detect(b));
        let c = Magefile::parse(b).unwrap();
        assert_eq!(c.mg_calls, 1);
    }

    #[test]
    fn rejects_plain_go() {
        assert!(!detect(b"package main\n\nfunc main() {}\n"));
        assert!(!detect(b"//go:build linux\n\npackage main\n"));
        assert!(!detect(b"package lib\n\n//go:build mage\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

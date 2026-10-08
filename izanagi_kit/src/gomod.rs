//! Go `go.mod` module-definition format.
//!
//! go.mod uses `module path`, `go 1.x` / `toolchain go1.x`,
//! `require (...)` blocks or single `require` lines, `replace`,
//! `exclude`, `retract`, and `// indirect` trailing markers.
//!
//! ```
//! let b = concat!(
//!     "module example.com/m\n",
//!     "\n",
//!     "go 121\n",
//!     "\n",
//!     "toolchain go122\n",
//!     "\n",
//!     "require (\n",
//!     "\texample.com/a v100\n",
//!     "\texample.com/b v200 // indirect\n",
//!     ")\n",
//!     "\n",
//!     "replace example.com/a => ./a\n"
//! ).as_bytes();
//! assert!(izanagi_kit::gomod::detect(b));
//! let c = izanagi_kit::gomod::Gomod::parse(b).unwrap();
//! assert_eq!(c.requires, 2);
//! assert_eq!(c.indirect, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed go.mod summary.
#[derive(Debug, Clone)]
pub struct Gomod {
    /// `module` declarations.
    pub module: usize,
    /// `go`/`toolchain` directives.
    pub go: usize,
    /// `require` entries (block + single).
    pub requires: usize,
    /// `// indirect` markers.
    pub indirect: usize,
    /// `replace` directives.
    pub replace: usize,
    /// `exclude`/`retract` directives.
    pub exclude: usize,
    /// `//` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like go.mod.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines().any(|l| l.trim_start().starts_with("module "))
        && (t
            .lines()
            .any(|l| l.trim_start().starts_with("go ") || l.trim_start().starts_with("require")))
}

impl Gomod {
    /// Parses a go.mod summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            module: 0,
            go: 0,
            requires: 0,
            indirect: 0,
            replace: 0,
            exclude: 0,
            comments: 0,
        };
        let mut in_block = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if in_block {
                if tr.starts_with(')') {
                    in_block = false;
                    continue;
                }
                c.requires += 1;
                if tr.contains("// indirect") {
                    c.indirect += 1;
                }
                continue;
            }
            if tr.starts_with("require (") || tr == "require (" {
                in_block = true;
            } else if tr.starts_with("require ") {
                c.requires += 1;
                if tr.contains("// indirect") {
                    c.indirect += 1;
                }
            } else if tr.starts_with("module ") {
                c.module += 1;
            } else if tr.starts_with("go ") || tr.starts_with("toolchain ") {
                c.go += 1;
            } else if tr.starts_with("replace ") {
                c.replace += 1;
            } else if tr.starts_with("exclude ") || tr.starts_with("retract") {
                c.exclude += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gomod() {
        let b = concat!(
            "module example.com/m\n",
            "\n",
            "go 121\n",
            "\n",
            "toolchain go122\n",
            "\n",
            "require (\n",
            "\texample.com/a v100\n",
            "\texample.com/b v200 // indirect\n",
            "\texample.com/c v300\n",
            ")\n",
            "\n",
            "require example.com/d v400 // indirect\n",
            "\n",
            "replace example.com/a => ./a\n",
            "exclude example.com/e v500\n",
            "// tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Gomod::parse(b).unwrap();
        assert_eq!(c.module, 1);
        assert_eq!(c.go, 2);
        assert_eq!(c.requires, 4);
        assert_eq!(c.indirect, 2);
        assert_eq!(c.replace, 1);
        assert_eq!(c.exclude, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"require (\n  x v1\n)\n"));
        assert!(Gomod::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

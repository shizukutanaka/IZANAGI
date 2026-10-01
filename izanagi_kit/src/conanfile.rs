//! Conan `conanfile.txt` consumer-recipe format.
//!
//! conanfile.txt is INI-ish with `[requires]`, `[tool_requires]`,
//! `[test_requires]`, `[generators]`, `[layout]`, `[options]`,
//! `[imports]` sections; requires entries look like
//! `pkg/1.0`, `pkg/1.0@user/channel`, `pkg/[>=1.0]`.
//!
//! ```
//! let b = concat!(
//!     "[requires]\n",
//!     "boost/179\n",
//!     "fmt/1000@u/c\n",
//!     "[tool_requires]\n",
//!     "cmake/327\n",
//!     "[generators]\n",
//!     "CMakeDeps\n",
//!     "CMakeToolchain\n",
//!     "[options]\n",
//!     "boost/*:shared=True\n"
//! ).as_bytes();
//! assert!(izanagi_kit::conanfile::detect(b));
//! let c = izanagi_kit::conanfile::Conanfile::parse(b).unwrap();
//! assert_eq!(c.requires, 3);
//! assert_eq!(c.generators, 2);
//! ```

/// Parsed conanfile.txt summary.
#[derive(Debug, Clone)]
pub struct Conanfile {
    /// `[section]` headers.
    pub sections: usize,
    /// Entries in `[requires]`/`[tool_requires]`/`[test_requires]`.
    pub requires: usize,
    /// `[generators]` entries.
    pub generators: usize,
    /// `[options]` entries (`pkg/*:opt=val`).
    pub options: usize,
    /// `[layout]`/`[imports]` entries.
    pub other: usize,
    /// `@user/channel` refs.
    pub user_refs: usize,
    /// Version-range refs (`pkg/[>=1.0]`).
    pub ranges: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "requires",
    "tool_requires",
    "test_requires",
    "generators",
    "layout",
    "options",
    "imports",
];

/// Whether the buffer looks like conanfile.txt.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut seen = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with('[') && tr.ends_with(']') {
            let inner = &tr[1..tr.len() - 1];
            if SECTIONS.contains(&inner) {
                seen += 1;
            }
        }
    }
    seen >= 1
        && t.lines().any(|l| {
            let tr = l.trim();
            !tr.starts_with('[') && tr.contains('/') && !tr.is_empty()
        })
}

impl Conanfile {
    /// Parses a conanfile.txt summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            requires: 0,
            generators: 0,
            options: 0,
            other: 0,
            user_refs: 0,
            ranges: 0,
            comments: 0,
        };
        let mut section: Option<&str> = None;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
                section = Some(&tr[1..tr.len() - 1]);
                continue;
            }
            match section {
                Some("requires") | Some("tool_requires") | Some("test_requires") => {
                    c.requires += 1;
                    if tr.contains('@') {
                        c.user_refs += 1;
                    }
                    if tr.contains('[') {
                        c.ranges += 1;
                    }
                }
                Some("generators") => c.generators += 1,
                Some("options") => {
                    c.options += 1;
                    if tr.contains('@') {
                        c.user_refs += 1;
                    }
                }
                Some("layout") | Some("imports") => c.other += 1,
                _ => c.other += 1,
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_conanfile() {
        let b = concat!(
            "[requires]\n",
            "boost/179\n",
            "fmt/1000@u/c\n",
            "openssl/[>300]\n",
            "[tool_requires]\n",
            "cmake/327\n",
            "[test_requires]\n",
            "gtest/115\n",
            "[generators]\n",
            "CMakeDeps\n",
            "CMakeToolchain\n",
            "[options]\n",
            "boost/*:shared=True\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Conanfile::parse(b).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.requires, 5);
        assert_eq!(c.generators, 2);
        assert_eq!(c.options, 1);
        assert_eq!(c.user_refs, 1);
        assert_eq!(c.ranges, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[other]\nkey=value\n"));
        assert!(Conanfile::parse(b"x").is_none());
    }
}

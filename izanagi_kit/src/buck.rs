//! Buck/Buck2 `BUCK` file — `load("@fbcode_macros//…")`, rule calls like
//! `cxx_binary`, `rust_library`, `python_test`, `export_file`, and `deps` lists.
//!
//! ```
//! let d = b"cxx_binary(\n    name = \"app\",\n    srcs = [\"main\x2ecpp\"],\n    deps = [\"//lib:core\"],\n)\nrust_library(name = \"core\")\n";
//! let b = izanagi_kit::buck::parse(d).unwrap();
//! assert_eq!(b.rules, 2);
//! assert_eq!(b.targets, 2);
//! assert!(izanagi_kit::buck::detect(d));
//! ```

/// A parsed Buck `BUCK`/`TARGETS` file summary.
#[derive(Debug, Clone)]
pub struct Buck {
    /// `load("@…")` statements.
    pub loads: usize,
    /// Rule invocations (`name = "…"` inside a call counts as a target).
    pub rules: usize,
    /// Distinct `name = "…"` kwargs (target count).
    pub targets: usize,
    /// `*_binary` / `*_test` rules.
    pub binaries: usize,
    /// `*_library` rules.
    pub libraries: usize,
    /// `cxx_*`/`rust_*`/`java_*`/`python_*`/`go_*`/`sh_*` language-prefixed rules.
    pub lang_rules: usize,
    /// `export_file`/`filegroup`/`genrule`/`custom_unittest` helpers.
    pub helpers: usize,
    /// `deps = […]` lists.
    pub deps_lists: usize,
    /// `visibility = […]` lists.
    pub visibility: usize,
    /// Comments.
    pub comments: usize,
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn is_callable(head: &str) -> bool {
    !head.is_empty()
        && head
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && head.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
}

const HELPERS: &[&str] = &[
    "export_file",
    "filegroup",
    "genrule",
    "custom_unittest",
    "remote_file",
    "http_file",
    "alias",
];

/// Detects a Buck file: `load(` with `@` or a `*_binary`/`*_library`-style call.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().map(strip_comment).any(|l| {
        let l = l.trim();
        if l.starts_with("load(") && l.contains('@') {
            return true;
        }
        if let Some(i) = l.find('(') {
            let h = l[..i].trim_end();
            return is_callable(h)
                && (h.ends_with("_binary")
                    || h.ends_with("_library")
                    || h.ends_with("_test")
                    || HELPERS.contains(&h));
        }
        false
    })
}

/// Parses a `BUCK` file; `None` without Buck constructs.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Buck> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Buck {
        loads: 0,
        rules: 0,
        targets: 0,
        binaries: 0,
        libraries: 0,
        lang_rules: 0,
        helpers: 0,
        deps_lists: 0,
        visibility: 0,
        comments: 0,
    };
    for raw in t.lines() {
        if raw.trim_start().starts_with('#') {
            s.comments += 1;
        }
        let l = strip_comment(raw).trim();
        if l.starts_with("load(") {
            s.loads += 1;
        }
        if let Some(i) = l.find('(') {
            let h = l[..i].trim_end();
            if is_callable(h) && h != "load" {
                s.rules += 1;
                if h.ends_with("_binary") || h.ends_with("_test") {
                    s.binaries += 1;
                }
                if h.ends_with("_library") {
                    s.libraries += 1;
                }
                if HELPERS.contains(&h) {
                    s.helpers += 1;
                }
                if [
                    "cxx_", "rust_", "java_", "python_", "go_", "sh_", "apple_", "android_",
                ]
                .iter()
                .any(|p| h.starts_with(p))
                {
                    s.lang_rules += 1;
                }
            }
        }
        if l.starts_with("deps =") || l.contains(" deps =") || l.starts_with("deps=[") {
            s.deps_lists += 1;
        }
        if l.contains("visibility") {
            s.visibility += 1;
        }
        if l.contains("name =") {
            s.targets += 1;
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"load(\"@fbcode_macros//build_defs:custom_unittest.bzl\", \"custom_unittest\")\n# cxx app\ncxx_binary(\n    name = \"app\",\n    srcs = [\"main.cpp\"],\n    deps = [\"//lib:core\"],\n)\nrust_library(name = \"core\")\nexport_file(name = \"data\")\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.loads, 1);
        assert_eq!(s.rules, 3);
        assert_eq!(s.targets, 3);
        assert_eq!(s.binaries, 1);
        assert_eq!(s.libraries, 1);
        assert_eq!(s.lang_rules, 2);
        assert_eq!(s.helpers, 1);
        assert_eq!(s.deps_lists, 1);
        assert_eq!(s.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"java_library(name = \"j\")"));
        assert!(!detect(b"x_binary = 1"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"$(shell ls)").is_none());
    }
}

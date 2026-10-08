//! Bazel `BUILD` / `BUILD.bazel` — `load()` statements and rule calls like
//! `cc_binary`, `java_library`, `genrule`, `filegroup`.
//!
//! ```
//! let d = b"load(\"@rules_cc//cc:defs\x2ebzl\", \"cc_binary\")\ncc_binary(name = \"app\", srcs = [\"main\x2ecc\"])\ncc_library(name = \"core\")\n";
//! let z = izanagi_kit::bazel::parse(d).unwrap();
//! assert_eq!(z.loads, 1);
//! assert_eq!(z.rules, 2);
//! assert_eq!(z.names, 2);
//! assert!(izanagi_kit::bazel::detect(d));
//! ```

use crate::textutil::strip_bom;
/// A parsed Bazel BUILD-file summary.
#[derive(Debug, Clone)]
pub struct Bazel {
    /// `load("@…")` statements.
    pub loads: usize,
    /// Rule invocations (`name = "…"` kwarg inside a `foo(…)` call).
    pub rules: usize,
    /// Total `name = "…"` rule targets.
    pub names: usize,
    /// `cc_*` rules.
    pub cc_rules: usize,
    /// `java_*`/`py_*`/`go_*`/`rust_*` language rules.
    pub lang_rules: usize,
    /// `genrule` count.
    pub genrules: usize,
    /// `filegroup`/`exports_files`/`alias`/`toolchain`/`platform` helper rules.
    pub helpers: usize,
    /// Glob calls (`glob([...])`).
    pub globs: usize,
    /// Visiblity args (`visibility = [...]`).
    pub visibility: usize,
    /// Comments.
    pub comments: usize,
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn looks_like_call(line: &str) -> bool {
    let line = line.trim();
    line.ends_with('(') || line.contains('(')
}

fn is_rule_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
}

/// Detects a Bazel BUILD file: `load(…)` or a known rule call.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines().map(strip_comment).any(|l| {
        let l = l.trim();
        (l.starts_with("load(") && l.contains('@'))
            || (looks_like_call(l)
                && l.split('(').next().is_some_and(is_rule_name)
                && (l.contains("name =") || l.contains("srcs") || l.contains("deps")))
    })
}

/// Parses a Bazel BUILD file; `None` without a `load` or rule call.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Bazel> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut s = Bazel {
        loads: 0,
        rules: 0,
        names: 0,
        cc_rules: 0,
        lang_rules: 0,
        genrules: 0,
        helpers: 0,
        globs: 0,
        visibility: 0,
        comments: 0,
    };
    let mut in_call = false;
    for raw in t.lines() {
        let line = strip_comment(raw);
        if raw.trim_start().starts_with('#') {
            s.comments += 1;
        }
        let l = line.trim();
        if l.starts_with("load(") {
            s.loads += 1;
            in_call = !l.contains(')');
            continue;
        }
        if let Some(open) = l.find('(') {
            let head = l[..open].trim_end();
            if is_rule_name(head) && !head.starts_with("if") && !head.starts_with("for") {
                s.rules += 1;
                in_call = !l.contains(')');
                if head.starts_with("cc_") {
                    s.cc_rules += 1;
                }
                if ["java_", "py_", "go_", "rust_", "sh_", "proto_"]
                    .iter()
                    .any(|p| head.starts_with(p))
                {
                    s.lang_rules += 1;
                }
                match head {
                    "genrule" => s.genrules += 1,
                    "filegroup" | "exports_files" | "alias" | "toolchain" | "platform" => {
                        s.helpers += 1;
                    }
                    _ => {}
                }
            }
        }
        // nested `glob(` inside an arg list is counted separately below
        if in_call || l.contains('(') {
            if l.contains("name =") {
                s.names += 1;
            }
            if l.contains("visibility") {
                s.visibility += 1;
            }
        }
        if in_call && l.ends_with(')') {
            in_call = false;
        }
        s.globs += l.match_indices("glob(").count();
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"# build file\nload(\"@rules_cc//cc:defs\x2ebzl\",\"cc_binary\")\ncc_binary(\n    name = \"app\",\n    srcs = glob([\"**/*.cc\"]),\n    deps = [\":core\"],\n    visibility = [\"//visibility:public\"],\n)\ncc_library(name = \"core\")\ngenrule(name = \"gen\")\nfilegroup(name = \"files\")\njava_library(name = \"j\")\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.loads, 1);
        assert_eq!(s.rules, 5); // glob inside an arg list is not a top-level rule
        assert_eq!(s.names, 5);
        assert_eq!(s.cc_rules, 2);
        assert_eq!(s.lang_rules, 1);
        assert_eq!(s.genrules, 1);
        assert_eq!(s.helpers, 1);
        assert_eq!(s.globs, 1);
        assert_eq!(s.visibility, 1);
        assert_eq!(s.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"load(\"@bazel_tools//:x\")"));
        assert!(!detect(b"def f():"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"x = 1").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

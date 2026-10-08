//! GN build files (`BUILD.gn`) — `group()`, `executable()`, `static_library()`,
//! `source_set()`, `config()`, `action()`, `template()` calls and `deps`/`sources` lists.
//!
//! ```
//! let d = b"group(\"all\") { deps = [\":app\"] }\nexecutable(\"app\") {\n  sources = [\"main\x2ecc\"]\n  deps = [\":core\"]\n}\n";
//! let g = izanagi_kit::gn::parse(d).unwrap();
//! assert_eq!(g.targets, 2);
//! assert_eq!(g.deps_lists, 1);
//! assert_eq!(g.sources_lists, 1);
//! assert!(izanagi_kit::gn::detect(d));
//! ```

/// A parsed `BUILD.gn` file summary.
#[derive(Debug, Clone)]
pub struct Gn {
    /// Target-type calls (`group`/`executable`/`static_library`/`source_set`/
    /// `shared_library`/`action`/`copy`/`generate_*`/`template`/`tool`/`config`/`test`).
    pub targets: usize,
    /// `deps = […]` / `public_deps = […]` list assignments.
    pub deps_lists: usize,
    /// `sources = […]` assignments.
    pub sources_lists: usize,
    /// `configs = […]` / `public_configs = […]` assignments.
    pub configs_lists: usize,
    /// `defines = […]`, `cflags = […]`, `include_dirs = […]` assignments combined.
    pub flag_lists: usize,
    /// `if (cond) { }` blocks.
    pub ifs: usize,
    /// `foreach (x, list) { }` blocks.
    pub foreaches: usize,
    /// `import("…")` calls.
    pub imports: usize,
    /// `assert(…)`/`print(…)`/`error(…)`/`warning(…)` calls.
    pub diagnostics: usize,
    /// Comments.
    pub comments: usize,
}

const TARGETS: &[&str] = &[
    "group",
    "executable",
    "static_library",
    "shared_library",
    "source_set",
    "action",
    "copy",
    "bundle_data",
    "create_bundle",
    "generated_file",
    "generate_json",
    "template",
    "tool",
    "config",
    "test",
    "source_identity",
];
const DIAG: &[&str] = &["assert", "print", "error", "warning", "not_needed"];

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn call_head(line: &str) -> Option<&str> {
    let l = line.trim();
    let i = l.find('(')?;
    let head = l[..i].trim_end();
    if !head.is_empty()
        && head
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
        && head.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
    {
        Some(head)
    } else {
        None
    }
}

/// Detects GN: a target-type call or a `deps`/`sources` list assignment.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().map(strip_comment).any(|l| {
        let l = l.trim();
        if let Some(h) = call_head(l) {
            if TARGETS.contains(&h) {
                return true;
            }
            // GN imports take a quoted .gn/.gni path (`import("//x.gni")`);
            // a bare `import foo` is not GN, and `print(`/`assert(` heads
            // are shared with Python/JS so they are not evidence.
            if h == "import" {
                return l.contains('"') && (l.contains(".gn") || l.contains(".gni"));
            }
        }
        l.starts_with("deps =") || l.starts_with("sources =") || l.starts_with("public_deps =")
    })
}

/// Parses a `BUILD.gn` file; `None` on non-UTF-8 or no GN constructs.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Gn> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Gn {
        targets: 0,
        deps_lists: 0,
        sources_lists: 0,
        configs_lists: 0,
        flag_lists: 0,
        ifs: 0,
        foreaches: 0,
        imports: 0,
        diagnostics: 0,
        comments: 0,
    };
    for raw in t.lines() {
        if raw.trim_start().starts_with('#') {
            s.comments += 1;
        }
        let l = strip_comment(raw).trim();
        if let Some(h) = call_head(l) {
            if TARGETS.contains(&h) {
                s.targets += 1;
            } else if DIAG.contains(&h) {
                s.diagnostics += 1;
            } else if h == "import" {
                s.imports += 1;
            } else if h == "if" {
                s.ifs += 1;
            } else if h == "foreach" {
                s.foreaches += 1;
            }
        }
        let assign = |names: &[&str]| names.iter().any(|n| l.starts_with(&format!("{n} =")));
        if assign(&["deps", "public_deps", "data_deps"]) {
            s.deps_lists += 1;
        }
        if assign(&["sources"]) {
            s.sources_lists += 1;
        }
        if assign(&["configs", "public_configs"]) {
            s.configs_lists += 1;
        }
        if assign(&["defines", "cflags", "include_dirs", "ldflags", "libs"]) {
            s.flag_lists += 1;
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"# gn file\nimport(\"//tools/grit/grit_rule.gni\")\ngroup(\"all\") { deps = [\":app\"] }\nexecutable(\"app\") {\n  sources = [\"main.cc\"]\n  deps = [\":core\"]\n  configs += [\":warn\"]\n  defines = [\"FOO\"]\n}\nif (is_win) { }\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.targets, 2); // group + executable
        assert_eq!(s.deps_lists, 1); // only line-anchored `deps =` counts
        assert_eq!(s.sources_lists, 1);
        assert_eq!(s.flag_lists, 1);
        assert_eq!(s.ifs, 1);
        assert_eq!(s.imports, 1);
        assert_eq!(s.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"executable(\"x\") { }"));
        assert!(detect(b"import(\"//tools/x.gni\")"));
        assert!(detect(b"sources = [\"a.cc\"]"));
        assert!(!detect(b"print x"));
        assert!(!detect(b"plain"));
        // Python/JS-shaped files share `print(`/`assert(` heads and a bare
        // `import`; none of those alone may detect (round-417 census).
        assert!(!detect(b"import foo\nprint(1)\n"));
        assert!(!detect(b"print(1)\nassert(x)\n"));
        assert!(!detect(b"import foo\n"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{}").is_none());
    }
}

//! Pants build files (`BUILD`/`BUILD.pants`) — `python_sources()`,
//! `python_tests()`, `pex_binary()`, `shell_source()`, `archive()` target calls.
//!
//! ```
//! let d = b"python_sources(name=\"src\")\npex_binary(name=\"app\", entry_point=\"main.py\")\npython_tests(name=\"tests\", dependencies=[\":src\"])\n";
//! let p = izanagi_kit::pants::parse(d).unwrap();
//! assert_eq!(p.targets, 3);
//! assert_eq!(p.python, 3); // python_sources + pex_binary + python_tests
//! assert!(izanagi_kit::pants::detect(d));
//! ```

/// A parsed Pants BUILD file summary.
#[derive(Debug, Clone)]
pub struct Pants {
    /// Total target-definition calls.
    pub targets: usize,
    /// `python_*` family (`python_sources`, `python_tests`, `pex_binary`, …).
    pub python: usize,
    /// `shell_*` family.
    pub shell: usize,
    /// `go_*` / `scala_*` / `java_*` families.
    pub other_lang: usize,
    /// `archive`/`files`/`resources`/`target`/`docker_image` helpers.
    pub helpers: usize,
    /// `dependencies = […]` lists.
    pub dependencies: usize,
    /// `sources = […]`/`source = …` args.
    pub sources: usize,
    /// `entry_point = …` args.
    pub entry_points: usize,
    /// Comments.
    pub comments: usize,
}

const TARGET_FAMILIES: &[&str] = &[
    "python_sources",
    "python_tests",
    "python_distribution",
    "pex_binary",
    "pex_binaries",
    "python_requirements",
    "poetry_requirements",
    "pipenv_requirements",
    "shell_sources",
    "shell_command",
    "shunit2_tests",
    "go_sources",
    "scala_sources",
    "java_sources",
    "junit_tests",
    "archive",
    "files",
    "resources",
    "relocated_files",
    "target",
    "docker_image",
    "experimental_shell_command",
];

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
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && head.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
    {
        Some(head)
    } else {
        None
    }
}

/// Detects a Pants BUILD file: a known target-family call.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().map(strip_comment).any(|l| {
        call_head(l).is_some_and(|h| TARGET_FAMILIES.contains(&h) || h.starts_with("pex_"))
    })
}

/// Parses a Pants BUILD file; `None` without Pants target calls.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Pants> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Pants {
        targets: 0,
        python: 0,
        shell: 0,
        other_lang: 0,
        helpers: 0,
        dependencies: 0,
        sources: 0,
        entry_points: 0,
        comments: 0,
    };
    for raw in t.lines() {
        if raw.trim_start().starts_with('#') {
            s.comments += 1;
        }
        let l = strip_comment(raw).trim();
        if let Some(h) = call_head(l) {
            if TARGET_FAMILIES.contains(&h) || h.starts_with("pex_") {
                s.targets += 1;
                if h.starts_with("python_") || h.starts_with("pex_") {
                    s.python += 1;
                }
                if h.starts_with("shell_")
                    || h.starts_with("shunit2")
                    || h.contains("shell_command")
                {
                    s.shell += 1;
                }
                if ["go_", "scala_", "java_", "junit_"]
                    .iter()
                    .any(|p| h.starts_with(p))
                {
                    s.other_lang += 1;
                }
                if [
                    "archive",
                    "files",
                    "resources",
                    "target",
                    "docker_image",
                    "relocated_files",
                ]
                .contains(&h)
                {
                    s.helpers += 1;
                }
            }
        }
        if l.contains("dependencies") {
            s.dependencies += 1;
        }
        if l.contains("sources") || l.contains("source =") {
            s.sources += 1;
        }
        if l.contains("entry_point") {
            s.entry_points += 1;
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"# pants\npython_sources(name=\"src\")\npex_binary(name=\"app\", entry_point=\"main.py\")\npython_tests(name=\"tests\", dependencies=[\":src\"])\nshell_command(name=\"gen\")\narchive(name=\"pkg\")\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.targets, 5);
        assert_eq!(s.python, 3); // python_sources + pex_binary + python_tests
        assert_eq!(s.shell, 1);
        assert_eq!(s.helpers, 1);
        assert_eq!(s.dependencies, 1);
        assert_eq!(s.entry_points, 1);
        assert_eq!(s.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"pex_binary(name=\"x\")"));
        assert!(!detect(b"cc_library(name=\"x\")"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"target").is_none());
    }
}

//! Meson `meson.build` — `project('name','lang')`, `executable()`/`library()`,
//! `subdir()`, `dependency()`, and `option`-style assignments.
//!
//! ```
//! let src = b"project('demo','c', version: '1\x2e0')\nexecutable('demo','main\x2ec')\nlib = library('core','c\x2ec')\ndep = dependency('threads')\nsubdir('tests')\n";
//! let m = izanagi_kit::meson::parse(src).unwrap();
//! assert_eq!(m.project, "demo");
//! assert_eq!(m.targets, 2);
//! assert_eq!(m.deps, 1);
//! assert!(izanagi_kit::meson::detect(src));
//! ```

/// A parsed `meson.build` summary.
#[derive(Debug, Clone)]
pub struct Meson {
    /// `project('name', …)` first argument.
    pub project: String,
    /// `version:` kwarg value in `project()`.
    pub version: String,
    /// `executable()`/`library()`/`shared_library()`/`static_library()`/
    /// `custom_target()`/`run_target()` count.
    pub targets: usize,
    /// `dependency()` calls.
    pub deps: usize,
    /// `subdir()` calls.
    pub subdirs: usize,
    /// `if`/`elif`/`foreach`/`while` statements.
    pub conditionals: usize,
    /// `test()` calls.
    pub tests: usize,
    /// `install_data`/`install_headers`/`install_subdir` calls.
    pub installs: usize,
    /// Assignments `name = value`.
    pub assignments: usize,
    /// Other function calls not grouped above.
    pub other: usize,
}

const TARGETS: &[&str] = &[
    "executable",
    "library",
    "shared_library",
    "static_library",
    "shared_module",
    "jar",
    "custom_target",
    "run_target",
    "benchmark",
];
const INSTALLS: &[&str] = &["install_data", "install_headers", "install_subdir"];

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn call_args(rest: &str) -> Option<&str> {
    let rest = rest.trim_start();
    rest.strip_prefix('(')?.split(')').next()
}

fn first_str_arg(rest: &str) -> String {
    let Some(args) = call_args(rest) else {
        return String::new();
    };
    args.split(',')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string()
}

fn kwarg(rest: &str, key: &str) -> String {
    let Some(args) = call_args(rest) else {
        return String::new();
    };
    for a in args.split(',') {
        if let Some((k, v)) = a.split_once(':') {
            if k.trim() == key {
                return v.trim().trim_matches(|c| c == '\'' || c == '"').to_string();
            }
        }
    }
    String::new()
}

fn cmd_call<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let lower = line.to_ascii_lowercase();
    let i = lower.find(name)?;
    // must be a call token, not part of a longer identifier
    let before_ok = i == 0
        || !lower.as_bytes()[i - 1].is_ascii_alphanumeric() && lower.as_bytes()[i - 1] != b'_';
    if !before_ok {
        return None;
    }
    let j = i + name.len();
    let rest = line[j..].trim_start();
    rest.starts_with('(').then_some(rest)
}

/// Detects Meson: a `project(` call or two known built-in calls.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0;
    for line in t.lines().map(strip_comment) {
        for n in TARGETS
            .iter()
            .chain(INSTALLS.iter())
            .chain(["project", "dependency", "subdir", "test"].iter())
        {
            if cmd_call(line, n).is_some() {
                hits += 1;
            }
        }
    }
    hits >= 2
        || t.lines()
            .map(strip_comment)
            .any(|l| cmd_call(l, "project").is_some())
}

/// Parses `meson.build`; `None` on non-UTF-8 or no recognizable calls.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Meson> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Meson {
        project: String::new(),
        version: String::new(),
        targets: 0,
        deps: 0,
        subdirs: 0,
        conditionals: 0,
        tests: 0,
        installs: 0,
        assignments: 0,
        other: 0,
    };
    for line in t.lines().map(strip_comment) {
        let line = line.trim();
        if let Some(rest) = cmd_call(line, "project") {
            if s.project.is_empty() {
                s.project = first_str_arg(rest);
                s.version = kwarg(rest, "version");
            }
        }
        for &n in TARGETS {
            if cmd_call(line, n).is_some() {
                s.targets += 1;
            }
        }
        for &n in INSTALLS {
            if cmd_call(line, n).is_some() {
                s.installs += 1;
            }
        }
        if cmd_call(line, "dependency").is_some() {
            s.deps += 1;
        }
        if cmd_call(line, "subdir").is_some() {
            s.subdirs += 1;
        }
        if cmd_call(line, "test").is_some() {
            s.tests += 1;
        }
        if line.starts_with("if ")
            || line.starts_with("elif ")
            || line.starts_with("foreach ")
            || line.starts_with("while ")
        {
            s.conditionals += 1;
        }
        // plain `name = expr` (not `==`/`<=`/`>=`)
        if let Some(eq) = line.find('=') {
            let before = line[..eq].trim_end();
            let next = line.as_bytes().get(eq + 1).copied().unwrap_or(0);
            if !before.ends_with(['=', '<', '>', '!'])
                && next != b'='
                && before
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                && !before.is_empty()
            {
                s.assignments += 1;
            }
        }
        // other calls like `message(`, `configure_file(`, `summary(`
        for (i, _) in line.match_indices('(') {
            let head = line[..i].trim_end();
            if head.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !head.is_empty()
                && !TARGETS.contains(&head)
                && !INSTALLS.contains(&head)
                && ![
                    "project",
                    "dependency",
                    "subdir",
                    "test",
                    "if",
                    "elif",
                    "foreach",
                    "while",
                ]
                .contains(&head)
            {
                s.other += 1;
            }
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"project('demo','c', version: '1\x2e2')\nexe = executable('demo','main\x2ec')\nlib = static_library('core','c\x2ec')\ndep = dependency('threads')\nsubdir('tests')\nif get_option('x')\nendif\ninstall_headers('h\x2eh')\n";

    #[test]
    fn parses() {
        let s = parse(SRC).unwrap();
        assert_eq!(s.project, "demo");
        assert_eq!(s.version, "1\x2e2");
        assert_eq!(s.targets, 2);
        assert_eq!(s.deps, 1);
        assert_eq!(s.subdirs, 1);
        assert_eq!(s.conditionals, 1);
        assert_eq!(s.installs, 1);
        assert!(s.assignments >= 3);
    }

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(detect(b"executable('a','a\x2ec')\ndependency('d')"));
        assert!(!detect(b"echo hi"));
        assert!(!detect(b"int x = 0"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# nothing").is_none());
    }
}

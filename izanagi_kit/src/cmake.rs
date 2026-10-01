//! CMake `CMakeLists.txt` — `cmake_minimum_required`, `project()`, and command-name census.
//!
//! ```
//! let src = b"cmake_minimum_required(VERSION 3\x2e20)\nproject(demo CXX)\nadd_executable(demo main\x2ecpp)\nadd_library(lib STATIC l\x2ecpp)\ntarget_link_libraries(demo lib)\ninstall(TARGETS demo)\n";
//! let c = izanagi_kit::cmake::parse(src).unwrap();
//! assert_eq!(c.min_version, "3\x2e20");
//! assert_eq!(c.project, "demo");
//! assert_eq!(c.executables, 1);
//! assert_eq!(c.libraries, 1);
//! assert!(izanagi_kit::cmake::detect(src));
//! ```

/// A parsed `CMakeLists.txt` summary.
#[derive(Debug, Clone)]
pub struct Cmake {
    /// `cmake_minimum_required(VERSION x)` value.
    pub min_version: String,
    /// First `project(name …)` name.
    pub project: String,
    /// `add_executable` count.
    pub executables: usize,
    /// `add_library` count.
    pub libraries: usize,
    /// `add_subdirectory` count.
    pub subdirs: usize,
    /// `find_package` count.
    pub packages: usize,
    /// `if`/`elseif`/`foreach`/`while` flow-control commands.
    pub conditionals: usize,
    /// `set`/`unset` commands.
    pub sets: usize,
    /// `install` commands.
    pub installs: usize,
    /// `option`/`include`/`message`/`target_*` and other commands combined.
    pub other: usize,
}

const COMMANDS: &[&str] = &[
    "cmake_minimum_required",
    "project",
    "add_executable",
    "add_library",
    "add_subdirectory",
    "add_test",
    "add_custom_command",
    "add_custom_target",
    "find_package",
    "if",
    "elseif",
    "else",
    "endif",
    "foreach",
    "endforeach",
    "while",
    "endwhile",
    "set",
    "unset",
    "option",
    "include",
    "message",
    "install",
    "enable_testing",
    "target_link_libraries",
    "target_include_directories",
    "target_compile_options",
    "target_compile_definitions",
    "target_compile_features",
];

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

/// First whitespace-or-paren-separated argument inside a `name(…)` call.
fn first_arg(call_rest: &str) -> String {
    let rest = call_rest.trim_start().trim_start_matches('(');
    rest.split(|c: char| c.is_whitespace() || c == ')')
        .next()
        .unwrap_or("")
        .to_string()
}

fn cmd_call<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let lower = line.to_ascii_lowercase();
    let mut off = 0;
    loop {
        let i = lower[off..].find(name)? + off;
        // must not be the tail of a longer command like `endif`
        let boundary = i == 0 || !lower.as_bytes()[i - 1].is_ascii_alphanumeric();
        let j = i + name.len();
        let rest = line[j..].trim_start();
        if boundary && rest.starts_with('(') {
            return Some(rest);
        }
        off = j;
    }
}

/// Detects CMake: any known command invocation on a non-comment line.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .map(strip_comment)
        .any(|l| COMMANDS.iter().any(|c| cmd_call(l, c).is_some()))
}

/// Parses `CMakeLists.txt`; `None` on non-UTF-8 or zero recognized commands.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Cmake> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Cmake {
        min_version: String::new(),
        project: String::new(),
        executables: 0,
        libraries: 0,
        subdirs: 0,
        packages: 0,
        conditionals: 0,
        sets: 0,
        installs: 0,
        other: 0,
    };
    let mut total = 0usize;
    for line in t.lines().map(strip_comment) {
        for &name in COMMANDS {
            if let Some(rest) = cmd_call(line, name) {
                total += 1;
                match name {
                    "cmake_minimum_required" => {
                        if s.min_version.is_empty() {
                            // args look like `VERSION 3.20` — take the token after VERSION
                            let mut it = rest
                                .trim_start_matches('(')
                                .trim_end_matches(')')
                                .split_whitespace();
                            s.min_version = match it.next() {
                                Some(w) if w.eq_ignore_ascii_case("version") => {
                                    it.next().unwrap_or("").to_string()
                                }
                                Some(w) => w.to_string(),
                                None => String::new(),
                            };
                        }
                    }
                    "project" => {
                        if s.project.is_empty() {
                            s.project = first_arg(rest);
                        }
                    }
                    "add_executable" => s.executables += 1,
                    "add_library" => s.libraries += 1,
                    "add_subdirectory" => s.subdirs += 1,
                    "find_package" => s.packages += 1,
                    "if" | "elseif" | "foreach" | "while" => s.conditionals += 1,
                    "set" | "unset" => s.sets += 1,
                    "install" => s.installs += 1,
                    _ => s.other += 1,
                }
            }
        }
    }
    let grouped = s.executables
        + s.libraries
        + s.subdirs
        + s.packages
        + s.conditionals
        + s.sets
        + s.installs
        + usize::from(!s.min_version.is_empty())
        + usize::from(!s.project.is_empty());
    s.other += total.saturating_sub(grouped);
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"# comment cmake_minimum_required(fake)\ncmake_minimum_required(VERSION 3\x2e16)\nproject(app VERSION 1\x2e0 LANGUAGES CXX)\noption(BUILD_TESTS \"t\" ON)\nif(BUILD_TESTS)\n  enable_testing()\nendif()\nadd_executable(app m\x2ecpp)\nadd_library(core c\x2ecpp)\ntarget_link_libraries(app core)\nadd_subdirectory(sub)\nfind_package(Threads REQUIRED)\ninstall(TARGETS app)\nset(X 1)\nunset(X)\n";

    #[test]
    fn parses() {
        let s = parse(SRC).unwrap();
        assert_eq!(s.min_version, "3\x2e16");
        assert_eq!(s.project, "app");
        assert_eq!(s.executables, 1);
        assert_eq!(s.libraries, 1);
        assert_eq!(s.subdirs, 1);
        assert_eq!(s.packages, 1);
        assert_eq!(s.conditionals, 1);
        assert_eq!(s.sets, 2);
        assert_eq!(s.installs, 1);
        assert!(s.other > 0);
    }

    #[test]
    fn case_insensitive_commands() {
        let s = parse(b"ADD_EXECUTABLE(x y\x2ecpp)\nPROJECT(p)\n").unwrap();
        assert_eq!(s.executables, 1);
        assert_eq!(s.project, "p");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"echo hello").is_none());
        assert!(parse(b"just text").is_none());
        assert!(!detect(b"not a build file"));
    }
}

//! Nim `.nimble` package definition — `version`/`srcDir`/`bin` fields,
//! `requires "nim >= …"` dependencies and `task name, "desc":` blocks.
//!
//! ```
//! let d = b"# Package\nversion     = \"0\x2e1\x2e0\"\nauthor      = \"Jane\"\ndescription = \"demo nimble package\"\nlicense     = \"MIT\"\nsrcDir      = \"src\"\nbin         = @[\"demo\"]\n\n# Dependencies\nrequires \"nim >= 1\x2e6\x2e0\"\nrequires \"chronos >= 3\x2e0\"\n\ntask test, \"Run tests\":\n  exec \"nim c -r tests/all\x2enim\"\n";
//! let n = izanagi_kit::nimble::parse(d).unwrap();
//! assert_eq!(n.version, "0.1.0");
//! assert_eq!(n.src_dir, "src");
//! assert_eq!(n.requires, 2);
//! assert_eq!(n.tasks, 1);
//! assert!(izanagi_kit::nimble::detect(d));
//! ```

/// A parsed `.nimble` file summary.
#[derive(Debug, Clone)]
pub struct Nimble {
    /// `packageName` or `name` value.
    pub name: String,
    /// `version` value.
    pub version: String,
    /// `author` value.
    pub author: String,
    /// `license` value.
    pub license: String,
    /// `srcDir` value.
    pub src_dir: String,
    /// `binDir` value.
    pub bin_dir: String,
    /// `bin = @[…]` executable entries.
    pub bins: usize,
    /// `skipDirs`/`skipFiles`/`skipExt`/`installDirs`/`installExt` entries.
    pub fs_filters: usize,
    /// `requires "pkg >= ver"` directives.
    pub requires: usize,
    /// `requires` entries pinning Nim itself (`nim >= x`).
    pub nim_requires: usize,
    /// `task name, "desc":` / `task name:` blocks.
    pub tasks: usize,
    /// `feature`/`namedBin`/`backend`/`before`/`after` keys present.
    pub misc_keys: usize,
    /// `foreignDeps` entries (system packages).
    pub foreign_deps: usize,
}

/// `key = "value"` / `key     = "value"` scalar.
fn scalar<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for line in t.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                return Some(v);
            }
        }
    }
    None
}

fn has_word(t: &str, w: &str) -> bool {
    t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|p| p == w)
}

/// `requires "…"` lines.
fn requires(t: &str) -> usize {
    t.lines()
        .filter(|l| {
            let l = l.trim_start();
            l.starts_with("requires") && l[8..].trim_start().starts_with('"')
        })
        .count()
}

/// `@["a", "b"]` entry count inside a `key = @` line.
fn at_list(t: &str, key: &str) -> usize {
    for line in t.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                let v = v.trim();
                if v.starts_with('@') {
                    return v
                        .split(',')
                        .filter(|p| p.contains('"') || p.contains('\''))
                        .count();
                }
            }
        }
    }
    0
}

/// Detects a `.nimble` file: `srcDir`/`binDir`/`bin =` or `requires "…"` markers.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    has_word(t, "srcDir") || has_word(t, "binDir") || requires(t) > 0 && has_word(t, "nim")
}

/// Parses a `.nimble` file; `None` without nimble markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nimble> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut fs = 0;
    for k in [
        "skipDirs",
        "skipFiles",
        "skipExt",
        "installDirs",
        "installExt",
    ] {
        if scalar(t, k).is_some() {
            fs += 1;
        }
    }
    let mut misc = 0;
    for k in ["features", "namedBin", "backend", "before", "after"] {
        if has_word(t, k) {
            misc += 1;
        }
    }
    let tasks = t
        .lines()
        .filter(|l| {
            let l = l.trim_start();
            l.starts_with("task") && (l.contains(',') || l.ends_with(':'))
        })
        .count();
    Some(Nimble {
        name: scalar(t, "packageName")
            .or_else(|| scalar(t, "name"))
            .unwrap_or("")
            .to_string(),
        version: scalar(t, "version").unwrap_or("").to_string(),
        author: scalar(t, "author").unwrap_or("").to_string(),
        license: scalar(t, "license").unwrap_or("").to_string(),
        src_dir: scalar(t, "srcDir").unwrap_or("").to_string(),
        bin_dir: scalar(t, "binDir").unwrap_or("").to_string(),
        bins: at_list(t, "bin"),
        fs_filters: fs,
        requires: requires(t),
        nim_requires: t
            .lines()
            .filter(|l| {
                let l = l.trim_start();
                l.starts_with("requires") && l.contains("\"nim")
            })
            .count(),
        tasks,
        misc_keys: misc,
        foreign_deps: scalar(t, "foreignDeps")
            .map(|_| {
                t.lines()
                    .filter(|l| l.trim_start().starts_with("foreignDep"))
                    .count()
                    .max(1)
            })
            .unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"# Package\n\nversion       = \"0\x2e1\x2e0\"\nauthor        = \"Jane\"\ndescription   = \"demo nimble package\"\nlicense       = \"MIT\"\nsrcDir        = \"src\"\nbinDir        = \"bin\"\nbin           = @[\"demo\", \"demo-tool\"]\nskipDirs      = @[\"tests\"]\n\n# Dependencies\n\nrequires \"nim >= 1\x2e6\x2e0\"\nrequires \"chronos >= 3\x2e0\"\nrequires \"stew\"\n\ntask test, \"Run tests\":\n  exec \"nim c -r tests/all\x2enim\"\n\ntask docs, \"Build docs\":\n  exec \"nim doc src/demo\x2enim\"\n";

    #[test]
    fn parses() {
        let n = parse(D).unwrap();
        assert_eq!(n.version, "0\x2e1\x2e0");
        assert_eq!(n.author, "Jane");
        assert_eq!(n.src_dir, "src");
        assert_eq!(n.bin_dir, "bin");
        assert_eq!(n.bins, 2);
        assert_eq!(n.requires, 3);
        assert_eq!(n.nim_requires, 1);
        assert_eq!(n.tasks, 2);
        assert_eq!(n.fs_filters, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"srcDir = \"src\"\n"));
        assert!(detect(b"requires \"nim >= 1\x2e0\"\n"));
        assert!(!detect(b"version = \"1\x2e0\"\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}

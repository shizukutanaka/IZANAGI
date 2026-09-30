//! Haskell `.cabal` package description — `cabal-version:`/`build-type:`,
//! `library`/`executable`/`test-suite`/`benchmark`/`flag`/`common` sections
//! and `build-depends:`/`exposed-modules:` field census.
//!
//! ```
//! let d = b"cabal-version: 2\x2e4\nname: demo\nversion: 0\x2e1\x2e0\nbuild-type: Simple\nlicense: MIT\n\nlibrary\n  exposed-modules: Demo, Demo.Internal\n  build-depends: base >=4 && <5, text\n  hs-source-dirs: src\n  default-language: Haskell2010\n\nexecutable demo\n  main-is: Main\x2ehs\n  build-depends: base, demo\n";
//! let c = izanagi_kit::cabal::parse(d).unwrap();
//! assert_eq!(c.name, "demo");
//! assert_eq!(c.libraries, 1);
//! assert_eq!(c.build_depends, 4);
//! assert!(izanagi_kit::cabal::detect(d));
//! ```

/// A parsed `.cabal` file summary.
#[derive(Debug, Clone)]
pub struct Cabal {
    /// `name:` value.
    pub name: String,
    /// `version:` value.
    pub version: String,
    /// `cabal-version:` value.
    pub cabal_version: String,
    /// `build-type:` value (`Simple`/`Configure`/`Make`/`Custom`).
    pub build_type: String,
    /// `license:`/`license-file:` value.
    pub license: String,
    /// `library` stanzas (incl. `library name` sub-libraries).
    pub libraries: usize,
    /// `executable` stanzas.
    pub executables: usize,
    /// `test-suite` stanzas.
    pub test_suites: usize,
    /// `benchmark` stanzas.
    pub benchmarks: usize,
    /// `flag` stanzas.
    pub flags: usize,
    /// `common` stanzas.
    pub commons: usize,
    /// `source-repository` stanzas.
    pub source_repos: usize,
    /// Comma-separated dependency atoms across `build-depends:` fields.
    pub build_depends: usize,
    /// Modules listed in `exposed-modules:` fields.
    pub exposed_modules: usize,
    /// Modules listed in `other-modules:` fields.
    pub other_modules: usize,
    /// `hs-source-dirs:` path entries.
    pub hs_source_dirs: usize,
    /// `default-language:` present (`Haskell2010`/`GHC2021`).
    pub default_language: bool,
}

/// Depth-0 `key:` or `key value` handling: cabal uses `key: value`.
fn top_value<'a>(t: &'a str, key: &str) -> Option<&'a str> {
    for line in t.lines() {
        if line.starts_with(char::is_whitespace) || line.trim_start().starts_with("--") {
            continue;
        }
        let l = line.trim_end();
        if let Some(rest) = l.strip_prefix(key) {
            let rest = rest.strip_prefix(':')?.trim();
            let v = rest.trim_matches('\'').trim_matches('"');
            return Some(v);
        }
    }
    None
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with(key) && l[key.len()..].trim_start().starts_with(':')
    })
}

/// Count comma-separated atoms across all `field:` lines at any depth.
fn field_atoms(t: &str, field: &str) -> usize {
    let mut n = 0;
    for line in t.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix(field) {
            if let Some(v) = rest.strip_prefix(':') {
                for atom in v.split(',') {
                    let a = atom.trim();
                    if !a.is_empty() && a.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
                    {
                        n += 1;
                    }
                }
            }
        }
    }
    n
}

/// Count stanzas whose first depth-0 word is `word` (`library`, `executable`, …).
fn stanzas(t: &str, word: &str) -> usize {
    t.lines()
        .filter(|l| {
            !l.starts_with(char::is_whitespace)
                && l.trim_end().starts_with(word)
                && l[word.len()..]
                    .trim_end()
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c.is_whitespace())
        })
        .count()
}

/// Detects a `.cabal` file: `cabal-version:` or `build-depends:`+stanza markers.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    has_key(t, "cabal-version")
        || (has_key(t, "build-depends")
            && (has_key(t, "exposed-modules")
                || has_key(t, "other-modules")
                || has_key(t, "hs-source-dirs")))
}

/// Parses a `.cabal` file; `None` without cabal markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Cabal> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let license = top_value(t, "license")
        .or_else(|| top_value(t, "license-file"))
        .unwrap_or("");
    Some(Cabal {
        name: top_value(t, "name").unwrap_or("").to_string(),
        version: top_value(t, "version").unwrap_or("").to_string(),
        cabal_version: top_value(t, "cabal-version").unwrap_or("").to_string(),
        build_type: top_value(t, "build-type").unwrap_or("").to_string(),
        license: license.to_string(),
        libraries: stanzas(t, "library"),
        executables: stanzas(t, "executable"),
        test_suites: stanzas(t, "test-suite"),
        benchmarks: stanzas(t, "benchmark"),
        flags: stanzas(t, "flag"),
        commons: stanzas(t, "common"),
        source_repos: stanzas(t, "source-repository"),
        build_depends: field_atoms(t, "build-depends"),
        exposed_modules: field_atoms(t, "exposed-modules"),
        other_modules: field_atoms(t, "other-modules"),
        hs_source_dirs: field_atoms(t, "hs-source-dirs"),
        default_language: has_key(t, "default-language"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"cabal-version: 2\x2e4\nname: demo\nversion: 0\x2e1\x2e0\nbuild-type: Simple\nlicense: MIT\nsynopsis: demo package\n\nlibrary\n  exposed-modules: Demo, Demo.Internal\n  build-depends: base >=4 && <5, text, bytestring\n  hs-source-dirs: src\n  default-language: Haskell2010\n\nexecutable demo\n  main-is: Main\x2ehs\n  hs-source-dirs: app\n  build-depends: base, demo\n\ntest-suite spec\n  type: exitcode-stdio-1\x2e0\n  build-depends: base, demo, hspec\n";

    #[test]
    fn parses() {
        let c = parse(D).unwrap();
        assert_eq!(c.name, "demo");
        assert_eq!(c.cabal_version, "2\x2e4");
        assert_eq!(c.build_type, "Simple");
        assert_eq!(c.license, "MIT");
        assert_eq!(c.libraries, 1);
        assert_eq!(c.executables, 1);
        assert_eq!(c.test_suites, 1);
        assert_eq!(c.build_depends, 8);
        assert_eq!(c.exposed_modules, 2);
        assert_eq!(c.hs_source_dirs, 2);
        assert!(c.default_language);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"cabal-version: 3\x2e0\n"));
        assert!(!detect(b"name: x\nversion: 1\x2e0\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}

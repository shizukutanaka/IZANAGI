//! Perl `cpanfile` — `requires 'Module';` dependency lines,
//! `on 'phase' => sub { … }` blocks, `recommends`/`suggests`/`conflicts`,
//! `feature` groups and `requires 'perl', 'version'`.
//!
//! ```
//! let d = b"requires 'perl', '5\x2e010';\nrequires 'Moo';\nrequires 'JSON::PP', '2\x2e0';\n\non 'test' => sub {\n    requires 'Test::More';\n    requires 'Test::Fatal', '0\x2e010';\n};\n\non 'develop' => sub {\n    recommends 'Perl::Critic';\n};\n";
//! let c = izanagi_kit::cpanfile::parse(d).unwrap();
//! assert_eq!(c.requires, 5);
//! assert_eq!(c.phase_blocks, 2);
//! assert_eq!(c.recommends, 1);
//! assert!(izanagi_kit::cpanfile::detect(d));
//! ```

/// A parsed `cpanfile` summary.
#[derive(Debug, Clone)]
pub struct Cpanfile {
    /// `requires 'Module'` directives (all phases, incl. `perl`).
    pub requires: usize,
    /// `requires` entries carrying a version constraint (`, 'x.y'` or `=> `).
    pub versioned_requires: usize,
    /// `requires 'perl', 'x.y'` entries.
    pub perl_requires: usize,
    /// `configure_requires`/`build_requires`/`test_requires` directives.
    pub stage_requires: usize,
    /// `recommends`/`suggests` entries.
    pub recommends: usize,
    /// `conflicts` entries.
    pub conflicts: usize,
    /// `on 'phase' => sub { … }` blocks (`test`/`build`/`configure`/`runtime`/`develop`/`author`).
    pub phase_blocks: usize,
    /// `feature 'name' => sub { … }` groups.
    pub features: usize,
    /// `os_prereqs`/`os` guards (`win32`/`!windows`/`linux`/`darwin`).
    pub os_guards: usize,
}

/// `keyword '…'` occurrences where `'` opens a quoted module name.
/// `keyword` must not be part of a longer identifier (`configure_requires` ≠ `requires`).
fn kw_quotes(t: &str, kw: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(kw) {
        let j = off + i;
        let before_ok =
            j == 0 || (!t.as_bytes()[j - 1].is_ascii_alphanumeric() && t.as_bytes()[j - 1] != b'_');
        let rest = t[j + kw.len()..].trim_start();
        if before_ok && (rest.starts_with('\'') || rest.starts_with('"')) {
            n += 1;
        }
        off = j + kw.len();
    }
    n
}

/// `kw` followed by `'mod', 'ver'` pairs.
fn versioned_kw(t: &str, kw: &str) -> usize {
    let mut n = 0;
    for line in t.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix(kw) {
            let rest = rest.trim_start();
            if (rest.starts_with('\'') || rest.starts_with('"')) && rest.contains("', '") {
                n += 1;
            }
        }
    }
    n
}

/// Detects a cpanfile: `requires 'X'` / `on 'phase' =>` / `feature 'x' =>` markers.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    kw_quotes(t, "requires") > 0
        || kw_quotes(t, "on") > 0 && t.contains("=>")
        || kw_quotes(t, "feature") > 0 && t.contains("=>")
}

/// Parses a `cpanfile`; `None` without `requires '…'` markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Cpanfile> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut stage = 0;
    for kw in [
        "configure_requires",
        "build_requires",
        "test_requires",
        "author_requires",
    ] {
        stage += kw_quotes(t, kw);
    }
    let mut os = 0;
    for line in t.lines() {
        let l = line.trim();
        if l.starts_with("os_prereqs") || l.starts_with("os(") || l.starts_with("os ") {
            os += 1;
        }
    }
    Some(Cpanfile {
        requires: kw_quotes(t, "requires"),
        versioned_requires: versioned_kw(t, "requires"),
        perl_requires: t
            .lines()
            .filter(|l| {
                let l = l.trim();
                l.starts_with("requires") && (l.contains("'perl'") || l.contains("\"perl\""))
            })
            .count(),
        stage_requires: stage,
        recommends: kw_quotes(t, "recommends") + kw_quotes(t, "suggests"),
        conflicts: kw_quotes(t, "conflicts"),
        phase_blocks: kw_quotes(t, "on"),
        features: kw_quotes(t, "feature"),
        os_guards: os,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"requires 'perl', '5\x2e010';\nrequires 'Moo';\nrequires 'JSON::PP', '2\x2e0';\nrequires 'Try::Tiny';\n\non 'test' => sub {\n    requires 'Test::More';\n    requires 'Test::Fatal', '0\x2e010';\n};\n\non 'configure' => sub {\n    requires 'ExtUtils::MakeMaker';\n};\n\non 'develop' => sub {\n    recommends 'Perl::Critic';\n    suggests 'Perl::Tidy';\n};\n\nfeature 'postgres' => sub {\n    requires 'DBD::Pg';\n};\n\nconflicts 'Old::Module';\n";

    #[test]
    fn parses() {
        let c = parse(D).unwrap();
        assert_eq!(c.requires, 8);
        assert_eq!(c.versioned_requires, 3);
        assert_eq!(c.perl_requires, 1);
        assert_eq!(c.phase_blocks, 3);
        assert_eq!(c.features, 1);
        assert_eq!(c.recommends, 2);
        assert_eq!(c.conflicts, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"requires 'Moo';\n"));
        assert!(!detect(b"require 'foo'\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}

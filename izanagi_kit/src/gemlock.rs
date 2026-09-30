//! `Gemfile.lock` — `GEM`/`PATH`/`GIT` sections, `remote:`/`specs:`,
//! `name (version)` spec lines, `PLATFORMS`, `DEPENDENCIES`, `BUNDLED WITH`.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of `Gemfile.lock`.
pub struct Gemlock {
    /// `GEM`/`PATH`/`GIT` source sections.
    pub sections: usize,
    /// `remote:` URLs.
    pub remotes: usize,
    /// `specs:` blocks.
    pub spec_blocks: usize,
    /// `name (ver)` spec entries (4-space indent).
    pub specs: usize,
    /// `dep (>= x)` dependency lines (6-space indent).
    pub dep_lines: usize,
    /// `revision:` entries (GIT sections).
    pub revisions: usize,
    /// `PLATFORMS` entries.
    pub platforms: usize,
    /// `DEPENDENCIES` entries.
    pub dependencies: usize,
    /// `BUNDLED WITH` version string.
    pub bundled_with: String,
    /// `CHECKSUMS` section entries.
    pub checksums: usize,
    /// `ruby` platform entries.
    pub ruby: usize,
}

fn section_lines<'a>(t: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in t.lines() {
        if l.trim() == name {
            inside = true;
            continue;
        }
        if inside {
            if l.trim().is_empty() || (!l.starts_with(' ') && l.trim_end() == l.trim()) {
                inside = false;
            } else {
                out.push(l);
            }
        }
    }
    out
}

fn version_after<'a>(t: &'a str, name: &str) -> Option<&'a str> {
    let lines = section_lines(t, name);
    lines.iter().find_map(|l| {
        let tr = l.trim();
        (!tr.is_empty()).then_some(tr)
    })
}

/// `true` when the text looks like Gemfile.lock.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    (t.contains("\nGEM\n") || t.starts_with("GEM\n") || t.contains("\nGIT\n"))
        && t.contains("specs:")
        || t.contains("BUNDLED WITH")
}

impl Gemlock {
    #[must_use]
    /// Parses `b` into `Gemlock`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let specs_lines = section_lines(t, "GEM")
            .into_iter()
            .chain(section_lines(t, "PATH"))
            .chain(section_lines(t, "GIT"))
            .collect::<Vec<_>>();
        let platforms = section_lines(t, "PLATFORMS");
        let deps = section_lines(t, "DEPENDENCIES");
        let checksums = section_lines(t, "CHECKSUMS");
        Some(Self {
            sections: ["GEM", "PATH", "GIT"]
                .iter()
                .filter(|s| t.lines().any(|l| l.trim() == **s))
                .count(),
            remotes: t
                .lines()
                .filter(|l| l.trim().starts_with("remote:"))
                .count(),
            spec_blocks: t.matches("specs:").count(),
            specs: specs_lines
                .iter()
                .filter(|l| {
                    let i = l.len() - l.trim_start().len();
                    i == 4 && l.trim().contains('(')
                })
                .count(),
            dep_lines: specs_lines
                .iter()
                .filter(|l| {
                    let i = l.len() - l.trim_start().len();
                    i == 6 && l.trim().contains('(')
                })
                .count(),
            revisions: t
                .lines()
                .filter(|l| l.trim().starts_with("revision:"))
                .count(),
            platforms: platforms.iter().filter(|l| !l.trim().is_empty()).count(),
            dependencies: deps.iter().filter(|l| !l.trim().is_empty()).count(),
            bundled_with: version_after(t, "BUNDLED WITH").unwrap_or("").to_string(),
            checksums: checksums.iter().filter(|l| !l.trim().is_empty()).count(),
            ruby: platforms.iter().filter(|l| l.trim() == "ruby").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"GEM\n  remote: https://rubygems.org/\n  specs:\n    rake (13\x2e0)\n    rspec (3\x2e12)\n      rspec-core (= 3\x2e12)\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  rake\n  rspec\n\nBUNDLED WITH\n   2\x2e4\n";

    #[test]
    fn detects_gemlock() {
        assert!(detect(FIX));
        assert!(!detect(b"GEM x"));
    }

    #[test]
    fn parses_gemlock() {
        let g = Gemlock::parse(FIX).unwrap();
        assert_eq!(g.sections, 1);
        assert_eq!(g.remotes, 1);
        assert_eq!(g.spec_blocks, 1);
        assert_eq!(g.specs, 2);
        assert_eq!(g.dep_lines, 1);
        assert_eq!(g.platforms, 1);
        assert_eq!(g.dependencies, 2);
        assert_eq!(g.bundled_with, "2\x2e4");
        assert!(Gemlock::parse(b"").is_none());
    }
}

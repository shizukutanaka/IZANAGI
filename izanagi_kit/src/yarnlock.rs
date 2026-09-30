//! Yarn v1 `yarn.lock` — `# yarn lockfile v1` comment header,
//! `pkg@range:` entries with indented `version`/`resolved`/`integrity`/`dependencies`.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of `yarn.lock`.
pub struct Yarnlock {
    /// `name@range[,name@range2]:` entry headers.
    pub entries: usize,
    /// Multi-range headers (`a@1, a@2:`).
    pub multi_range: usize,
    /// `version "x"` lines.
    pub versions: usize,
    /// `resolved "url"` lines.
    pub resolved: usize,
    /// `integrity "sha…"` lines.
    pub integrity: usize,
    /// `dependencies:` blocks.
    pub dependency_blocks: usize,
    /// Dependency `name "range"` lines.
    pub dependencies: usize,
    /// `optionalDependencies:` blocks.
    pub optional_dependencies: usize,
    /// `peerDependencies:` blocks.
    pub peer_dependencies: usize,
    /// `#` comments.
    pub comments: usize,
    /// Entries pinned to a git/registry URL.
    pub urls: usize,
    /// `languageName`/`linkType` Yarn ≥2 keys.
    pub berry_keys: usize,
}

fn in_block<'a>(t: &'a str, key: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in t.lines() {
        let tr = l.trim_end();
        if tr.trim().is_empty() {
            continue;
        }
        let i = tr.len() - tr.trim_start().len();
        if inside {
            if i == 0 {
                inside = false;
            } else {
                out.push(tr.trim());
                continue;
            }
        }
        if tr.trim() == [key, ":"].concat() || tr.trim() == key {
            inside = true;
        }
    }
    out
}

/// `true` when the text looks like a yarn.lock file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    t.contains("yarn lockfile")
        || (t.contains("\nversion ")
            && t.lines().any(|l| {
                let tr = l.trim_end();
                !tr.starts_with('#')
                    && !tr.starts_with(' ')
                    && tr.ends_with(':')
                    && tr.contains('@')
            }))
}

impl Yarnlock {
    #[must_use]
    /// Parses `b` into `Yarnlock`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut entries = 0usize;
        let mut multi = 0usize;
        for l in t.lines() {
            let tr = l.trim_end();
            if tr.is_empty() || tr.starts_with('#') || tr.starts_with(' ') || !tr.ends_with(':') {
                continue;
            }
            if tr.contains('@') {
                entries += 1;
                if tr.contains(',') {
                    multi += 1;
                }
            }
        }
        let deps = in_block(t, "dependencies");
        Some(Self {
            entries,
            multi_range: multi,
            versions: t
                .lines()
                .filter(|l| l.trim().starts_with("version \""))
                .count(),
            resolved: t.matches("resolved \"").count(),
            integrity: t
                .lines()
                .filter(|l| l.trim().starts_with("integrity"))
                .count(),
            dependency_blocks: t.lines().filter(|l| l.trim() == "dependencies:").count(),
            dependencies: deps.iter().filter(|l| l.contains('"')).count(),
            optional_dependencies: t.matches("optionalDependencies:").count(),
            peer_dependencies: t.matches("peerDependencies:").count(),
            comments: t
                .lines()
                .filter(|l| l.trim_start().starts_with('#'))
                .count(),
            urls: t.matches("http://").count() + t.matches("https://").count(),
            berry_keys: t.matches("languageName:").count() + t.matches("linkType:").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"# yarn lockfile v1\n\nleft-pad@^1\x2e0\x2e0:\n  version \"1\x2e3\x2e0\"\n  resolved \"https://r/left-pad.tgz\"\n  integrity sha512-x\n\nis-array@1, is-array@^1:\n  version \"1\x2e0\x2e0\"\n  resolved \"https://r/is-array.tgz\"\n  dependencies:\n    tiny \"^2\x2e0\"\n";

    #[test]
    fn detects_yarnlock() {
        assert!(detect(FIX));
        assert!(!detect(b"version = 1"));
    }

    #[test]
    fn parses_yarnlock() {
        let y = Yarnlock::parse(FIX).unwrap();
        assert_eq!(y.entries, 2);
        assert_eq!(y.multi_range, 1);
        assert_eq!(y.versions, 2);
        assert_eq!(y.resolved, 2);
        assert_eq!(y.integrity, 1);
        assert_eq!(y.dependencies, 1);
        assert_eq!(y.urls, 2);
        assert!(Yarnlock::parse(b"").is_none());
    }
}

//! cgit `cgitrc` parser.
//!
//! Detects cgit configuration by `key=value` lines with `repo.*`/`scan-path`/
//! `virtual-root`/`clone-url`/`readme=`/`css=` directives, and counts
//! structure.
//!
//! ```
//! let b = b"virtual-root=/cgit\ncache-size=1000\nscan-path=/srv/git\nrepo.url=myrepo\nrepo.path=/srv/git/myrepo.git\nrepo.desc=demo repo\nclone-url=https://git.example.com/myrepo.git\n";
//! assert!(izanagi_kit::cgitrc::detect(b));
//! let c = izanagi_kit::cgitrc::Cgit::parse(b).unwrap();
//! assert!(c.repo_keys >= 3);
//! ```

/// Parsed cgitrc summary.
#[derive(Debug, Clone)]
pub struct Cgit {
    /// Recognized directive occurrences.
    pub keys: usize,
    /// `repo.*` directives (`repo.url`/`repo.name`/`repo.path`/`repo.desc`/`repo.owner`/...).
    pub repo_keys: usize,
    /// Global/site directives (`scan-path`/`virtual-root`/`clone-url`/`css`/`readme`/`snapshots`/`enable-*`/`max-*`/`about-filter`/...).
    pub site_keys: usize,
    /// `section=` block starts (`section=path` groups repos).
    pub section_keys: usize,
    /// `key=value` assignment lines.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Repo directives.
const REPO_KEYS: &[&str] = &[
    "repo.url",
    "repo.name",
    "repo.path",
    "repo.des\u{63}",
    "repo.owner",
    "repo.defbranch",
    "repo.readme",
    "repo.clone-url",
    "repo.section",
    "repo.homepage",
    "repo.enable-commit-graph",
    "repo.enable-log-filecount",
    "repo.enable-log-linecount",
    "repo.enable-remote-branches",
    "repo.enable-subject-links",
    "repo.max-stats",
    "repo.module-link",
    "repo.snapshots",
    "repo.hide",
    "repo.ignore",
    "repo.about-filter",
    "repo.commit-filter",
    "repo.source-filter",
    "repo.email-filter",
    "repo.owner-filter",
    "repo.mtime",
    "repo.branch-sort",
    "repo.commit-sort",
];

/// Global/site directives.
const SITE_KEYS: &[&str] = &[
    "scan-path",
    "virtual-root",
    "clone-url",
    "css",
    "logo",
    "readme",
    "root-title",
    "root-des\u{63}",
    "root-readme",
    "snapshots",
    "robots",
    "include",
    "about-filter",
    "commit-filter",
    "source-filter",
    "email-filter",
    "owner-filter",
    "auth-filter",
    "enable-commit-graph",
    "enable-index-links",
    "enable-index-owner",
    "enable-log-filecount",
    "enable-log-linecount",
    "enable-remote-branches",
    "enable-subject-links",
    "enable-html-serving",
    "enable-blame",
    "max-repo-count",
    "max-commit-count",
    "max-lock-duration",
    "max-blob-size",
    "max-stats",
    "cache-size",
    "cache-root",
    "cache-static-ttl",
    "cache-dynamic-ttl",
    "cache-repo-ttl",
    "cache-scanrc-ttl",
    "cache-about-ttl",
    "cache-snapshot-ttl",
    "cache-blob-ttl",
    "local-time",
    "agefile",
    "mimetype",
    "head-include",
    "header",
    "footer",
    "logo-link",
    "module-link",
    "strict-export",
    "project-list",
    "section-from-path",
    "remove-suffix",
    "renumber",
    "side-by-side-diffs",
    "summary-branches",
    "summary-log",
    "summary-tags",
    "branch-sort",
    "commit-sort",
    "default-branch",
    "noprefixmatch",
    "clone-prefix",
    "clone-suffix",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "repo.url",
    "repo.path",
    "repo.name",
    "repo.des\u{63}",
    "scan-path",
    "virtual-root",
    "clone-url",
    "root-title",
    "css=",
    "snapshots=",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a cgitrc.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Cgit {
    /// Count categories in a cgitrc. Returns `None` when the input does
    /// not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            repo_keys: 0,
            site_keys: 0,
            section_keys: 0,
            assignments: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("section=") {
                c.section_keys += 1;
            }
            if tr.contains('=') {
                c.assignments += 1;
            }
        }
        for k in REPO_KEYS {
            c.repo_keys += t.matches(k).count();
        }
        for k in SITE_KEYS {
            c.site_keys += t.matches(k).count();
        }
        c.keys = c.repo_keys + c.site_keys + c.section_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# cgit config\nvirtual-root=/cgit\ncache-size=1000\nscan-path=/srv/git\nsection=public\nrepo.url=myrepo\nrepo.path=/srv/git/myrepo.git\nrepo.desc=demo repo\nrepo.owner=admin\nclone-url=https://git.example.com/myrepo.git\n";
        assert!(detect(b));
        let c = Cgit::parse(b).unwrap();
        assert!(c.repo_keys >= 4);
        assert!(c.site_keys >= 3);
        assert_eq!(c.section_keys, 1);
        assert_eq!(c.comments, 1);
        assert!(c.keys >= 8);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[section]\nkey=value\n"));
        assert!(Cgit::parse(b"a = b\n").is_none());
    }
}

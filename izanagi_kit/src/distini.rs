//! Dist::Zilla `dist.ini` census.
//!
//! INI: `[Name]`/`[]` sections for metadata, `[Plugin]` and
//! `[Plugin / Alias]` plugin sections, `[@Bundle]` bundle sections;
//! `key = value` metadata (`name`, `version`, `author`, `license`,
//! `copyright_holder`, `abstract`, `main_module`, `gather` …).
//! `;`/`#` comments.
//!
//! ```rust
//! let d = b"name    = My-Dist\nauthor  = A U Thor <a\x40x.com>\nlicense = Perl_5\ncopyright_holder = A U Thor\nabstract = my dist\nversion = 0.01\n[@Basic]\n[MetaJSON]\n";
//! assert!(izanagi_kit::distini::detect(d));
//! ```

/// dist.ini census.
#[derive(Debug, Clone)]
pub struct Distini {
    /// `[Section]`/`[@Bundle]` headers.
    pub sections: usize,
    /// `key = value` lines matching known Dist::Zilla metadata/keys.
    pub settings: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// Metadata and global keys (top of file or in core sections).
const KEYS: &[&str] = &[
    "abstract",
    "author",
    "auto_version",
    "bumpversion",
    "changelog",
    "charset",
    "comment_out",
    "comments",
    "copyright_holder",
    "copyright_year",
    "critical",
    "deletes",
    "distname",
    "exclude_files",
    "exclude_match",
    "exclude_prune",
    "ext",
    "filename",
    "follow_prereqs",
    "gather",
    "generatedby",
    "generated_by",
    "identifier",
    "install",
    "is_trial",
    "last",
    "license",
    "main_module",
    "match",
    "matches",
    "mode",
    "name",
    "newline",
    "perl",
    "perlbrew",
    "phase",
    "pragma",
    "prereqs",
    "prune",
    "readme",
    "relationship",
    "release_status",
    "repository",
    "root",
    "skip",
    "tag",
    "tag_format",
    "tag_message",
    "time_zone",
    "trial",
    "type",
    "version",
];

fn section(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim())
}

fn assign_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty() || t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() {
        return None;
    }
    Some(k)
}

/// Detect a `dist.ini` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut meta = 0usize;
    let mut secs = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if let Some(s) = section(tr) {
            if s.starts_with('@') || s.contains('/') || !s.contains(' ') {
                secs += 1;
            }
            continue;
        }
        if let Some(k) = assign_key(l) {
            if KEYS.contains(&k) {
                meta += 1;
            }
        }
    }
    meta >= 3 || (meta >= 1 && secs >= 2)
}

impl Distini {
    /// Count sections and metadata keys. Returns `None` when the input
    /// does not look like a `dist.ini`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with(';') || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(s) = section(tr) {
                if s.starts_with('@') || s.contains('/') || !s.contains(' ') {
                    c.sections += 1;
                }
                continue;
            }
            if let Some(k) = assign_key(l) {
                if KEYS.contains(&k) {
                    c.settings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"name    = My-Dist\nauthor  = A U Thor <a\x40x.com>\nlicense = Perl_5\ncopyright_holder = A U Thor\ncopyright_year   = 2024\nabstract = my dist\nversion = 0.01\nmain_module = lib/My/Dist.pm\n\n[GatherDir]\n[PruneCruft]\n[ManifestSkip]\n[MetaYAML]\n[MetaJSON]\n[License]\n[Readme]\n[ExtraTests]\n[ExecDir]\n[ShareDir]\n[MakeMaker]\n[Manifest]\n[TestRelease]\n[ConfirmRelease]\n[UploadToCPAN]\n[@Git]\n";
        assert!(detect(b));
        let c = Distini::parse(b).unwrap();
        assert!(c.settings >= 8);
        assert_eq!(c.sections, 16);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[database]\nhost=x\n"));
        assert!(!detect(b"foo = 1\nbar = 2\n"));
        assert!(Distini::parse(b"").is_none());
    }
}

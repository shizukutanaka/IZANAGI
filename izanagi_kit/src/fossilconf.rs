//! Fossil SCM `.fossil-settings/*` and `fossil settings` census.
//!
//! Fossil's versionable settings live as one `key: value` file per
//! setting under `.fossil-settings/` (`allow-symlinks`,
//! `binary-glob`, `crlf-glob`, `crnl-glob`, `clean-glob`,
//! `ignore-glob`, `keep-glob`, `manifest`, `dotfiles`,
//! `encoding-glob`, `binary-glob`, `empty-dirs`,
//! `exec-glob`, `glob-match`, `ignore-glob`, `keep-glob`,
//! `merge-glob`, `svn-tag`, `tcl`, `ui-caps`,
//! `undo`, `autosync`, `autosync-tries`, `auto-captcha`,
//! `auto-shun`, `case-sensitive`, `clearsign`, `cgi-debug`,
//! `default-sync`, `dont-push`, `dont-publish`, `editor`,
//! `email-forwarding`, `email-self`, `empty-dirs`,
//! `exec-glob`, `gdiff-command`, `gmerge-command`,
//! `hash-digits`, `http-auth`, `https-login`,
//! `ignore-glob`, `keep-glob`, `localauth`, `main-branch`,
//! `manifest`, `max-loadavg`, `max-upload`, `merge-glob`,
//! `mv-rm-files`, `newline-glob`, `pgp-command`,
//! `proxy`, `relative-paths`, `repo-cksum`,
//! `self-register`, `ssh-command`, `ssl-ca-location`,
//! `ssl-identity`, `tcl`, `tclsh`, `th1-docs`,
//! `th1-hooks`, `uv-sync`, `web-browser`).

//! The `.fossil-settings/` form holds one file per name whose
//! contents are the value; `fossil settings` prints `name (v|l) value`.
//!
//! ```rust
//! let c = izanagi_kit::fossilconf::Fossilconf::parse(b"autosync: on\nbinary-glob: *.o *.bin\nignore-glob: target/\n").unwrap();
//! assert_eq!(c.settings, 3);
//! ```

/// Fossil settings census.
#[derive(Debug, Clone)]
pub struct Fossilconf {
    /// `key: value`/`key = value` settings.
    pub settings: usize,
    /// Glob-valued settings (`*-glob`).
    pub globs: usize,
    /// `#` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "allow-symlinks",
    "binary-glob",
    "crlf-glob",
    "crnl-glob",
    "clean-glob",
    "ignore-glob",
    "keep-glob",
    "manifest",
    "dotfiles",
    "encoding-glob",
    "empty-dirs",
    "exec-glob",
    "glob-match",
    "merge-glob",
    "svn-tag",
    "tcl",
    "ui-caps",
    "undo",
    "autosync",
    "autosync-tries",
    "auto-captcha",
    "auto-shun",
    "case-sensitive",
    "clearsign",
    "cgi-debug",
    "default-sync",
    "dont-push",
    "dont-publish",
    "editor",
    "email-forwarding",
    "email-self",
    "gdiff-command",
    "gmerge-command",
    "hash-digits",
    "http-auth",
    "https-login",
    "localauth",
    "main-branch",
    "max-loadavg",
    "max-upload",
    "mv-rm-files",
    "newline-glob",
    "pgp-command",
    "proxy",
    "relative-paths",
    "repo-cksum",
    "self-register",
    "ssh-command",
    "ssl-ca-location",
    "ssl-identity",
    "tclsh",
    "th1-docs",
    "th1-hooks",
    "uv-sync",
    "web-browser",
    "captcha-secret",
    "crlf-glob",
    "access-log",
    "admin-log",
    "error-log",
    "index-page",
    "project-name",
    "project-description",
    "timeline-block-markup",
    "timeline-date-format",
    "timeline-default",
    "timeline-max-comment",
    "timeline-plaintext",
    "timeline-truncate-at-blank",
    "timeline-truncate-at-blank-lines",
    "show-version-diffs",
    "sameas",
    "multiple-threads",
    "project-code",
];

/// Whether the buffer looks like Fossil settings output.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = KEYS
        .iter()
        .filter(|k| {
            t.lines().any(|l| {
                let s = l.trim();
                s.starts_with(&format!("{k}:"))
                    || s.starts_with(&format!("{k} :"))
                    || s.starts_with(&format!("{k}="))
                    || s.starts_with(&format!("{k} ="))
                    || s.starts_with(&format!("{k} "))
            })
        })
        .count();
    hits >= 2
}

impl Fossilconf {
    /// Parse Fossil settings into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            globs: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let key = s.split([':', '=']).next().unwrap_or("").trim();
            if KEYS.contains(&key) {
                c.settings += 1;
                if key.ends_with("-glob") {
                    c.globs += 1;
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
    fn parses_settings() {
        let b = concat!(
            "# fossil settings\n",
            "autosync: on\n",
            "binary-glob: *.o *.bin *.exe\n",
            "clean-glob: obj/\n",
            "ignore-glob: target/ *.tmp\n",
            "keep-glob: lib/*.a\n",
            "manifest: uuid 1\n",
            "editor: vim\n",
            "main-branch: trunk\n",
            "web-browser: firefox\n",
        );
        let c = Fossilconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 9);
        assert_eq!(c.globs, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Fossilconf::parse(b"foo: bar\n").is_none());
    }
}

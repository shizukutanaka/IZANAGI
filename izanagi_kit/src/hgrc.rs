//! Mercurial `hgrc`/`hgweb.config`/`~/.hgrc` census.
//!
//! hgrc is INI: `[ui]` (`username`/`verbose`/`editor`/`color`/
//! `ssh`), `[paths]` (`default`/`default-push`), `[auth]`
//! (`<prefix>.prefix` / `.username` / `.password` / `.schemes`),
//! `[extensions]`, `[hooks]` (`prechangegroup`/`pretxncommit`),
//! `[merge-tools]` (`<tool>.args` / `.executable` / `.check`),
//! `[trusted]` (`users`/`groups`), `[progress]`/`[format]`/
//! `[alias]`/`[web]`/`[subrepos]`/`[largefiles]`/`[mq]`/
//! `[rebase]`/`[command-templates]`/`[templates]`/`[ui]`/
//! `[encoding]`/`[config]`/`[server]`/`[http_proxy]`/
//! `[smtp]`/`[email]`/`[mailmap]`/`[revsetlog]`/`[phases]`/
//! `[ui.report_untrusted]`/`[data]`/`[packs]`/`[commitcloud]`/
//! `[share]`/`[persistent-nodemap]`/`[sparse]`/`[clonebundles]`/
//! `[censor]`/`[narrow]`/`[remotenames]`/`[absorb]`/`[shelve]`/
//! `[transplant]`/`[convert]`/`[patch]`/`[pager]`/`[purge]`/
//! `[extdiff]`/`[histedit]`/`[gpg]`/`[blackbox]`/`[bugzilla]`/
//! `[schemes]`/`[hostsecurity]`/`[color]`/`[help]`/`[clipboard]`/
//! `[keyring]`/`[fsmonitor]`/`[amend]`/`[changelog]`/`[fastannotate]`/
//! `[journal]`/`[lfs]`/`[releasenotes]`/`[try]`/`[uncommit]`/
//! `[topam]`/`[topic]`/`[byrn]`/`[felix]`/`[fenix]`/`[fix]`/
//! `[graph]`/`[dagsort]`/`[ttoken]`/`[dprint]`/`[repeat]`/
//! `[replay]`/`[effort]`/`[sparse-revlog]`/`[templater]`/
//! `[url]`/`[subrepos]`/`[multi-line-log]`/`[edgecase]`/`[pt]`/
//! `[autorebase]`/`[tinytest]`/`[patchlists]`/`[patterns]` sections
//! and `key = value` settings.
//!
//! ```rust
//! let c = izanagi_kit::hgrc::Hgrc::parse(b"[ui]\nusername = X <x@y>\n[paths]\ndefault = ../r\n").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// hgrc census.
#[derive(Debug, Clone)]
pub struct Hgrc {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value` entries.
    pub entries: usize,
    /// `[auth]` per-host entries (`x.prefix`/`.username`/…).
    pub auth_entries: usize,
    /// `[hooks]` hook definitions.
    pub hooks: usize,
    /// `;`/`#` comments.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "ui",
    "paths",
    "auth",
    "extensions",
    "hooks",
    "merge-tools",
    "trusted",
    "progress",
    "format",
    "alias",
    "web",
    "subrepos",
    "largefiles",
    "mq",
    "rebase",
    "command-templates",
    "templates",
    "encoding",
    "config",
    "server",
    "http_proxy",
    "smtp",
    "email",
    "mailmap",
    "phases",
    "data",
    "packs",
    "commitcloud",
    "share",
    "persistent-nodemap",
    "sparse",
    "clonebundles",
    "censor",
    "narrow",
    "remotenames",
    "absorb",
    "shelve",
    "transplant",
    "convert",
    "patch",
    "pager",
    "purge",
    "extdiff",
    "histedit",
    "gpg",
    "blackbox",
    "bugzilla",
    "schemes",
    "hostsecurity",
    "color",
    "help",
    "clipboard",
    "keyring",
    "fsmonitor",
    "amend",
    "changelog",
    "journal",
    "lfs",
    "releasenotes",
    "uncommit",
    "graph",
    "dagsort",
    "effort",
    "templater",
    "url",
    "autorebase",
];

/// Whether the buffer looks like an hgrc.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    SECTIONS
        .iter()
        .filter(|s| t.contains(&format!("[{s}]")))
        .count()
        >= 2
        || (t.contains("[ui]") && (t.contains("username =") || t.contains("username=")))
        || t.contains("[merge-tools]")
        || t.contains("[auth]")
}

impl Hgrc {
    /// Parse an hgrc into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            auth_entries: 0,
            hooks: 0,
            comments: 0,
        };
        let mut scope = "";
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
                scope = s.trim_start_matches('[').split(']').next().unwrap_or("");
                continue;
            }
            if s.contains('=') {
                c.entries += 1;
                if scope == "auth" {
                    c.auth_entries += 1;
                } else if scope == "hooks" {
                    c.hooks += 1;
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
    fn parses_hgrc() {
        let b = concat!(
            "[ui]\n",
            "username = Example User <u@example.com>\n",
            "verbose = True\n",
            "[paths]\n",
            "default = ../repo\n",
            "default-push = ../repo\n",
            "[auth]\n",
            "x.prefix = hg.example.com\n",
            "x.username = u\n",
            "x.password = p\n",
            "[hooks]\n",
            "prechangegroup = hg update\n",
            "pretxncommit = true\n",
            "[extensions]\n",
            "rebase =\n",
            "mq =\n",
            "[alias]\n",
            "ci = commit\n",
            "; end\n",
        );
        let c = Hgrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.entries, 12);
        assert_eq!(c.auth_entries, 3);
        assert_eq!(c.hooks, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Hgrc::parse(b"[foo]\nx=1\n").is_none());
    }
}

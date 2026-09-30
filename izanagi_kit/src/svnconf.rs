//! Subversion `config`/`servers` census.
//!
//! `~/.subversion/config` (INI): `[general]` (`store-plaintext-passwords`,
//! `store-passwords`, `store-auth-creds`), `[helpers]`
//! (`editor-cmd`, `diff-cmd`, `merge-tool-cmd`,
//! `password-stores`, `kwallet-password-stores`),
//! `[miscellany]` (`global-ignores`, `log-encoding`,
//! `use-commit-times`, `enable-auto-props`, `interactive-conflicts`,
//! `mimetypes-file`, `preserved-conflict-file-exts`),
//! `[auto-props]` (`*.py = svn:eol-style=native`),
//! `[auth]` (`store-passwords`, `store-plaintext-passwords`,
//! `password-stores`).
//!
//! `~/.subversion/servers`: `[global]`
//! (`http-proxy-host`, `http-proxy-port`, `http-proxy-user`,
//! `http-proxy-password`, `http-proxy-exceptions`,
//! `http-timeout`, `http-library`, `store-plaintext-passwords`,
//! `store-passwords`, `store-ssl-client-cert-pp`,
//! `neon-debug-mask`, `http-compression`),
//! `[groups]` (`g = *.example.com`) then per-group named
//! sections with the same http-proxy keys.
//!
//! ```rust
//! let c = izanagi_kit::svnconf::Svnconf::parse(b"[general]\nstore-passwords = on\n[auto-props]\n*.py = svn:eol-style=native\n").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// `svn` config/servers census.
#[derive(Debug, Clone)]
pub struct Svnconf {
    /// `[section]` headers.
    pub sections: usize,
    /// `key = value` entries.
    pub entries: usize,
    /// `[groups]` host-glob group definitions.
    pub groups: usize,
    /// `[auto-props]` `glob = prop=val` rules.
    pub auto_props: usize,
    /// `;`/`#` comments.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "general",
    "helpers",
    "miscellany",
    "auto-props",
    "auth",
    "global",
    "groups",
    "tunnels",
    "working-copy",
];

/// Whether the buffer looks like an svn config/servers file.
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
        || t.contains("svn:eol-style")
        || t.contains("store-plaintext-passwords")
        || t.contains("http-proxy-host")
        || t.contains("[auto-props]")
        || t.contains("[miscellany]")
}

impl Svnconf {
    /// Parse an svn config/servers file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            groups: 0,
            auto_props: 0,
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
                if scope == "groups" {
                    c.groups += 1;
                } else if scope == "auto-props" {
                    c.auto_props += 1;
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
    fn parses_config() {
        let b = concat!(
            "[general]\n",
            "store-passwords = on\n",
            "store-plaintext-passwords = off\n",
            "[helpers]\n",
            "editor-cmd = vim\n",
            "diff-cmd = /usr/bin/diff\n",
            "merge-tool-cmd = merge\n",
            "[miscellany]\n",
            "global-ignores = *.o *.lo\n",
            "enable-auto-props = yes\n",
            "[auto-props]\n",
            "*.py = svn:eol-style=native\n",
            "*.sh = svn:executable\n",
            "*.jpg = svn:mime-type=image/jpeg\n",
            "[tunnels]\n",
            "ssh-tunnel = $SVN_SSH ssh\n",
        );
        let c = Svnconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.entries, 11);
        assert_eq!(c.auto_props, 3);
    }

    #[test]
    fn parses_servers() {
        let b = concat!(
            "[groups]\n",
            "work = *.example.com\n",
            "home = *.home.net\n",
            "[global]\n",
            "http-proxy-host = proxy.corp\n",
            "http-proxy-port = 8080\n",
            "http-proxy-user = u\n",
            "http-timeout = 30\n",
            "[work]\n",
            "http-proxy-exceptions = internal.example.com\n",
            "store-passwords = on\n",
        );
        let c = Svnconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.entries, 8);
        assert_eq!(c.groups, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Svnconf::parse(b"[foo]\nx=1\n").is_none());
    }
}

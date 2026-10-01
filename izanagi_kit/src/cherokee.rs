//! Census of a Cherokee `cherokee.conf`.
//!
//! Cherokee config: flat `key = value` pairs with `vserver!N!key = value`
//! / `source!N!key` / `icons!key` dotted-bang numbering, `server!*`/
//! `vserver!*`/`source!*`/`rule!N!*` namespaces, keys like `document_root`,
//! `directory_index`, `handler`, `balancer`, `timeout`, `collect_statistics`,
//! `loggers`, `mime`, `handler!file`/`fcgi`/`common`/`redir`/`secdownload`,
//! `#` comments.
//!
//! ```rust
//! let c = izanagi_kit::cherokee::Cherokee::parse(
//!     b"vserver!10!document_root = /srv\nvserver!10!directory_index = index.html\n",
//! ).unwrap();
//! assert_eq!(c.namespaced, 2);
//! ```
#![forbid(unsafe_code)]

/// Cherokee config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cherokee {
    /// `vserver!`/`source!`/`rule!`/`icons!`-namespaced settings.
    pub namespaced: usize,
    /// Other `key = value` settings.
    pub settings: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

/// Namespace prefixes counted in `namespaced`.
const NS: &[&str] = &[
    "vserver!", "source!", "rule!", "icons!", "mime!", "config!", "admin!",
];

/// True if `b` looks like Cherokee config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("vserver!") || (t.contains("server!") && t.contains('!'))
}

impl Cherokee {
    /// Parse a `cherokee.conf` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            namespaced: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') || l.starts_with(';') {
                c.comments += 1;
            } else if let Some(eq) = l.find('=') {
                let key = l[..eq].trim();
                if NS.iter().any(|p| key.starts_with(p)) || key.starts_with("server!") {
                    c.namespaced += 1;
                } else if !key.is_empty() {
                    c.settings += 1;
                }
            }
        }
        if c.namespaced + c.settings == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        concat!(
            "# cherokee\n",
            "server!bind!1!port = 80\n",
            "server!bind!2!port = 443\n",
            "server!collector!enabled = 0\n",
            "vserver!10!document_root = /srv/www\n",
            "vserver!10!directory_index = index.html\n",
            "vserver!10!logger!filename = /var/log/cherokee/access.log\n",
            "vserver!10!rule!10!match = default\n",
            "vserver!10!rule!10!handler = common\n",
            "source!1!type = interpreter\n",
            "source!1!host = 127.0.0.1:9000\n",
            "icons!directory = folder.png\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Cherokee::parse(b.as_bytes()).unwrap();
        assert_eq!(c.namespaced, 11);
        assert_eq!(c.settings, 0);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"a=b\n"));
        assert!(Cherokee::parse(b"# none\n").is_none());
    }
}

//! Census of an Apache `httpd.conf` / `.conf` configuration.
//!
//! Apache config: `<VirtualHost>`/`<Directory>`/`<Location>`/`<IfModule>`/
//! `<DirectoryMatch>`/`<Files>`/`<Limit>`/`<Proxy>` containers and their
//! `</…>` closers, directives (`Listen`/`ServerName`/`DocumentRoot`/
//! `ServerAdmin`/`ErrorLog`/`CustomLog`/`LoadModule`/`Options`/`AllowOverride`/
//! `Require`/`DirectoryIndex`/`SSLEngine`/`Redirect`/`RewriteRule`/
//! `RewriteCond`/`ProxyPass`/`Header`/`SetEnv`), `.htaccess`-style flags
//! `+FollowSymLinks`, `Include`/`IncludeOptional`, `Define`, `#` comments,
//! `<IfDefine>`, mod-`IfModule` ids.
//!
//! ```rust
//! let c = izanagi_kit::apacheconf::Apacheconf::parse(
//!     b"Listen 80\nServerName example.com\n<VirtualHost *:80>\n    DocumentRoot /srv\n</VirtualHost>\n",
//! ).unwrap();
//! assert_eq!(c.containers, 1);
//! ```
#![forbid(unsafe_code)]

/// Apache config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Apacheconf {
    /// `<Container …>` opener lines.
    pub containers: usize,
    /// `</Container>` closers.
    pub closers: usize,
    /// Plain directive lines.
    pub directives: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like Apache config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("<VirtualHost") || t.contains("LoadModule") || t.contains("DocumentRoot"))
        && (t.contains("Listen") || t.contains("ServerName") || t.contains("</"))
}

impl Apacheconf {
    /// Parse an Apache config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            containers: 0,
            closers: 0,
            directives: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with("</") {
                c.closers += 1;
            } else if l.starts_with('<') && !l.starts_with("</") {
                c.containers += 1;
            } else {
                let head = l.split_whitespace().next().unwrap_or("");
                if head
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                    && !head.is_empty()
                {
                    c.directives += 1;
                }
            }
        }
        if c.directives + c.containers == 0 {
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
            "# apache\n",
            "Listen 80\n",
            "ServerName example.com\n",
            "LoadModule rewrite_module modules/mod_rewrite.so\n",
            "<VirtualHost *:80>\n",
            "    DocumentRoot /srv/www\n",
            "    <Directory /srv/www>\n",
            "        Options +FollowSymLinks\n",
            "        Require all granted\n",
            "    </Directory>\n",
            "    ErrorLog logs/err.log\n",
            "    CustomLog logs/acc.log common\n",
            "</VirtualHost>\n",
            "<IfModule ssl_module>\n",
            "    Listen 443\n",
            "</IfModule>\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Apacheconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.containers, 3);
        assert_eq!(c.closers, 3);
        assert!(c.directives >= 7);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"server { listen 80; }\n"));
        assert!(Apacheconf::parse(b"# none\n").is_none());
    }
}

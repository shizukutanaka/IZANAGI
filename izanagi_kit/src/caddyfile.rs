//! Census of a Caddyfile.
//!
//! Caddyfile: site-address blocks `example.com { … }` or `localhost:8080`,
//! directives (`reverse_proxy`/`file_server`/`root`/`rewrite`/`encode`/
//! `tls`/`handle`/`respond`/`log`/`import`/`php_fastcgi`/`try_files`/
//! `header`), matchers `@name`, snippets `(name)`, environment
//! placeholders `{$VAR}`, global options block `{ … }`, `#` comments,
//! `*`/`?` wildcards in site addresses.
//!
//! ```rust
//! let c = izanagi_kit::caddyfile::Caddyfile::parse(
//!     b"example.com {\n    root * /srv\n    file_server\n}\n",
//! ).unwrap();
//! assert_eq!(c.sites, 1);
//! ```
#![forbid(unsafe_code)]

/// Caddyfile census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caddyfile {
    /// Site-address / block-opening lines (`x {` or bare address lines).
    pub sites: usize,
    /// Directive lines (indented keywords + matchers).
    pub directives: usize,
    /// `@name` matcher definitions and `(name)` snippets.
    pub matchers: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// True if `b` looks like a Caddyfile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("reverse_proxy") || t.contains("file_server") || t.contains("php_fastcgi"))
        && !t.contains("server {")
}

impl Caddyfile {
    /// Parse a Caddyfile into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sites: 0,
            directives: 0,
            matchers: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with('@') || l.starts_with('(') {
                c.matchers += 1;
            } else if l.ends_with('{') || l == "{" {
                // site block opener or continuation `{`
                if l == "{"
                    || l.starts_with(|ch: char| {
                        ch.is_ascii_alphanumeric() || ch == '.' || ch == '*' || ch == ':'
                    })
                {
                    c.sites += 1;
                }
            } else if line.starts_with(|ch: char| ch.is_whitespace()) || !l.is_empty() {
                // indented directive or bare directive line
                let head = l.split_whitespace().next().unwrap_or("");
                if !head.is_empty()
                    && head
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
                {
                    if line.starts_with(char::is_whitespace) {
                        c.directives += 1;
                    } else if l.ends_with('{') {
                        c.sites += 1;
                    } else {
                        c.directives += 1;
                    }
                }
            }
        }
        if c.directives == 0 {
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
            "# caddy\n",
            "example.com {\n",
            "    root * /srv/www\n",
            "    file_server\n",
            "    encode gzip zstd\n",
            "    @api path /api/*\n",
            "    handle @api {\n",
            "        reverse_proxy localhost:9000\n",
            "    }\n",
            "}\n",
            "api.example.com {\n",
            "    reverse_proxy 127.0.0.1:8080\n",
            "}\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Caddyfile::parse(b.as_bytes()).unwrap();
        assert!(c.sites >= 3);
        assert!(c.directives >= 4);
        assert_eq!(c.matchers, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"server { listen 80; }\n"));
        assert!(Caddyfile::parse(b"# none\n").is_none());
    }
}

//! Census of an H2O `h2o.conf` file.
//!
//! H2O config (YAML-flavoured): `listen:`/`hosts:`/`paths:` structure
//! with `host:`/`port:`/`ssl:` entries, per-path handler directives
//! (`file.dir`/`proxy.reverse.url`/`proxy.preserve-host`/`fastcgi.connect`/
//! `mruby.handler`/`redirect`/`expires`/`compress`/`header.set`/`reproxy`),
//! `error-log`/`access-log`/`pid-file`/`user`/`max-connections`/
//! `http2-*`/`ssl-session-resumption`, `- ` list items, `&`/`*` YAML anchors,
//! `#` comments.
//!
//! ```rust
//! let c = izanagi_kit::h2oconf::H2oconf::parse(
//!     b"listen: 8080\nhosts:\n  \"example.com\":\n    paths:\n      \"/\": file.dir /srv\n",
//! ).unwrap();
//! assert_eq!(c.hosts, 1);
//! ```
#![forbid(unsafe_code)]

/// H2O config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H2oconf {
    /// `listen:`/`hosts:`/`paths:` structure keys.
    pub structure: usize,
    /// Host entries (`"name":`/`- name:` under hosts).
    pub hosts: usize,
    /// `key: value` scalar directives (incl. `mod.*`/`file.*`/`proxy.*`).
    pub directives: usize,
    /// `- ` list items.
    pub items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Structure keys counted once each.
const STRUCT: &[&str] = &[
    "listen:",
    "hosts:",
    "paths:",
    "ssl:",
    "access-log:",
    "error-log:",
    "user:",
    "pid-file:",
];

/// True if `b` looks like H2O config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("hosts:") && t.contains("paths:"))
        || t.contains("file.dir")
        || t.contains("proxy.reverse.url")
}

impl H2oconf {
    /// Parse an `h2o.conf` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            structure: 0,
            hosts: 0,
            directives: 0,
            items: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if l.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if let Some(colon) = l.find(':') {
                let key = l[..colon].trim();
                if key.is_empty() {
                    continue;
                }
                if key.starts_with('"') || key.starts_with('\'') {
                    let inner = key.trim_matches(|ch| ch == '"' || ch == '\'');
                    if inner.starts_with('/') {
                        c.structure += 1; // quoted path entry
                    } else {
                        c.hosts += 1;
                    }
                    continue;
                }
                let tag = format!("{key}:");
                if STRUCT.contains(&tag.as_str()) {
                    c.structure += 1;
                } else {
                    c.directives += 1;
                }
            }
        }
        if c.structure + c.directives == 0 {
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
            "# h2o\n",
            "listen: 8080\n",
            "error-log: /var/log/h2o/error.log\n",
            "pid-file: /run/h2o.pid\n",
            "user: nobody\n",
            "max-connections: 1024\n",
            "hosts:\n",
            "  \"example.com\":\n",
            "    paths:\n",
            "      \"/\":\n",
            "        file.dir: /srv/www\n",
            "        expires: 1 day\n",
            "      \"/api\":\n",
            "        proxy.reverse.url: http://127.0.0.1:9000\n",
            "        proxy.preserve-host: ON\n",
            "    access-log: /var/log/h2o/access.log\n",
            "  \"api.example.com\":\n",
            "    paths:\n",
            "      \"/\": proxy.reverse.url http://127.0.0.1:9001\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = H2oconf::parse(b.as_bytes()).unwrap();
        assert!(c.structure >= 5);
        assert_eq!(c.hosts, 2);
        assert!(c.directives >= 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"server { listen 80; }\n"));
        assert!(H2oconf::parse(b"# none\n").is_none());
    }
}

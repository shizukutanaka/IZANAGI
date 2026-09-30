//! HAProxy configuration census.
//!
//! `haproxy.cfg` has named sections (`global`, `defaults`, `frontend`,
//! `backend`, `listen`, `resolvers`, `peers`, `mailers`, `cache`,
//! `userlist`, `program`, `ring`, `aggregate`) followed by indented
//! directives: `bind`, `acl`, `use_backend`, `default_backend`, `balance`,
//! `server`, `option`, `timeout`, `mode`, `log`, `stats`, `maxconn`,
//! `http-request`/`http-response`, `errorfile`, `redirect`. `parse`
//! counts sections and directive classes.
//!
//! ```rust
//! let h = concat!(
//!     "global\n",
//!     "    maxconn 4000\n",
//!     "defaults\n",
//!     "    mode http\n",
//!     "    timeout connect 5000\n",
//!     "frontend fe\n",
//!     "    bind *:80\n",
//!     "    acl is_api path_beg /api\n",
//!     "    default_backend be\n",
//!     "backend be\n",
//!     "    balance roundrobin\n",
//!     "    server s1 127.0.0.1:8080 check\n",
//! );
//! let c = izanagi_kit::haproxy::Haproxy::parse(h.as_bytes()).unwrap();
//! assert_eq!(c.frontends, 1);
//! assert_eq!(c.backends, 1);
//! ```

const SECTIONS: &[&str] = &[
    "global",
    "defaults",
    "frontend",
    "backend",
    "listen",
    "resolvers",
    "peers",
    "mailers",
    "cache",
    "userlist",
    "program",
    "ring",
    "aggregate",
    "dynamic-cookies",
    "cfgparser",
    "http-errors",
    "multicast",
    "trace",
];

/// HAProxy configuration census.
#[derive(Debug, Clone)]
pub struct Haproxy {
    /// `global`/`defaults`/`resolvers`/`peers`/`mailers`/`cache`/`userlist`/`program`/`ring` sections.
    pub globals: usize,
    /// `frontend <name>` sections.
    pub frontends: usize,
    /// `backend <name>` sections.
    pub backends: usize,
    /// `listen <name>` sections.
    pub listens: usize,
    /// `bind` lines.
    pub binds: usize,
    /// `acl`/`declare`/`capture` lines.
    pub acls: usize,
    /// `server` lines.
    pub servers: usize,
    /// `use_backend`/`default_backend`/`block` lines.
    pub backend_refs: usize,
    /// `option`/`timeout`/`mode`/`balance`/`log`/`stats`/`maxconn`/`redirect`/`http-*`/`tcp-*`/`stick`/`monitor`/`error*`/`unique-id*` lines.
    pub options: usize,
    /// Other directive lines.
    pub directives: usize,
}

/// Whether the buffer looks like an HAProxy configuration.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let has_section = ["frontend", "backend", "listen", "global", "defaults"]
        .iter()
        .any(|k| t.lines().any(|l| l.trim().starts_with(k)));
    has_section
        && (t.contains("bind")
            || t.contains("server ")
            || t.contains("balance")
            || t.contains("maxconn"))
}

impl Haproxy {
    /// Parse an HAProxy configuration into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            globals: 0,
            frontends: 0,
            backends: 0,
            listens: 0,
            binds: 0,
            acls: 0,
            servers: 0,
            backend_refs: 0,
            options: 0,
            directives: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            let is_section =
                SECTIONS.contains(&head) && s.split_whitespace().count() <= 3 && !s.contains('=');
            if is_section {
                match head {
                    "frontend" => c.frontends += 1,
                    "backend" => c.backends += 1,
                    "listen" => c.listens += 1,
                    _ => c.globals += 1,
                }
                continue;
            }
            if s.starts_with("bind ") {
                c.binds += 1;
                continue;
            }
            if s.starts_with("acl ") || s.starts_with("declare") || s.starts_with("capture") {
                c.acls += 1;
                continue;
            }
            if s.starts_with("server ") || s.starts_with("server-template") {
                c.servers += 1;
                continue;
            }
            if s.starts_with("use_backend")
                || s.starts_with("default_backend")
                || s.starts_with("block ")
                || s.starts_with("return ")
            {
                c.backend_refs += 1;
                continue;
            }
            if s.starts_with("option ")
                || s.starts_with("timeout ")
                || s.starts_with("mode ")
                || s.starts_with("balance ")
                || s.starts_with("log ")
                || s.starts_with("stats ")
                || s.starts_with("maxconn")
                || s.starts_with("redirect")
                || s.starts_with("http-request")
                || s.starts_with("http-response")
                || s.starts_with("tcp-request")
                || s.starts_with("tcp-response")
                || s.starts_with("stick")
                || s.starts_with("monitor")
                || s.starts_with("error")
                || s.starts_with("unique-id")
                || s.starts_with("hash-type")
            {
                c.options += 1;
                continue;
            }
            if head
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic())
            {
                c.directives += 1;
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
            "global\n",
            "    log /dev/log local0\n",
            "    maxconn 4000\n",
            "    daemon\n",
            "defaults\n",
            "    log global\n",
            "    mode http\n",
            "    option httplog\n",
            "    timeout connect 5000\n",
            "frontend fe\n",
            "    bind *:80\n",
            "    acl is_api path_beg /api\n",
            "    use_backend api if is_api\n",
            "    default_backend be\n",
            "backend be\n",
            "    balance roundrobin\n",
            "    option httpchk\n",
            "    server s1 127.0.0.1:8080 check\n",
            "    server s2 127.0.0.1:8081 check\n",
            "listen stats\n",
            "    bind :1936\n",
            "    stats enable\n",
            "    stats uri /stats\n",
        );
        let c = Haproxy::parse(b.as_bytes()).unwrap();
        assert_eq!(c.globals, 2);
        assert_eq!(c.frontends, 1);
        assert_eq!(c.backends, 1);
        assert_eq!(c.listens, 1);
        assert_eq!(c.binds, 2);
        assert_eq!(c.acls, 1);
        assert_eq!(c.servers, 2);
        assert_eq!(c.backend_refs, 2);
        assert_eq!(c.options, 10);
        assert_eq!(c.directives, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Haproxy::parse(b"foo = 1").is_none());
    }
}

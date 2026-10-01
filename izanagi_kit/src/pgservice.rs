//! PostgreSQL `pg_service.conf` census.
//!
//! INI sections named after services, each with libpq keyword=value
//! pairs: `host`/`port`/`dbname`/`user`/`password`/`sslmode`/`sslcert`/
//! `sslkey`/`sslrootcert`/`sslcrl`/`application_name`/`connect_timeout`/
//! `keepalives`/`keepalives_idle`/`keepalives_interval`/`keepalives_count`/
//! `options`/`service`/`hostaddr`/`fallback_application_name`/`gssencmode`/
//! `channel_binding`/`target_session_attrs`/`load_balance_hosts`/
//! `max_prepared_statements`/`passfile`/`requiressl`/`sslcompression`/
//! `ssl_max_protocol_version`/`ssl_min_protocol_version`/`sslmode`/
//! `sslnegotiation`/`sslsni`/`tcp_user_timeout`/`client_encoding`/`krbsrvname`.
//!
//! ```rust
//! let p = "[maindb]\nhost=db.internal\nport=5433\ndbname=app\n";
//! let c = izanagi_kit::pgservice::Pgservice::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.services, 1);
//! assert_eq!(c.settings, 3);
//! ```

/// pg_service.conf census.
#[derive(Debug, Clone)]
pub struct Pgservice {
    /// `[service]` section headers.
    pub services: usize,
    /// `key=value` pairs.
    pub settings: usize,
    /// Recognised libpq connection keywords.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "host",
    "hostaddr",
    "port",
    "dbname",
    "user",
    "password",
    "passfile",
    "connect_timeout",
    "client_encoding",
    "options",
    "application_name",
    "fallback_application_name",
    "keepalives",
    "keepalives_idle",
    "keepalives_interval",
    "keepalives_count",
    "tcp_user_timeout",
    "sslmode",
    "sslcompression",
    "sslcert",
    "sslkey",
    "sslrootcert",
    "sslcrl",
    "sslcrldir",
    "sslsni",
    "requiressl",
    "sslnegotiation",
    "ssl_min_protocol_version",
    "ssl_max_protocol_version",
    "gssencmode",
    "krbsrvname",
    "gsslib",
    "service",
    "target_session_attrs",
    "load_balance_hosts",
    "channel_binding",
    "max_prepared_statements",
];

/// Whether the buffer looks like pg_service.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("[")
        && (t.contains("dbname=")
            || t.contains("dbname =")
            || (t.contains("host=") || t.contains("host =")) && t.contains("port"))
        || t.contains("sslmode") && t.contains("dbname")
}

impl Pgservice {
    /// Parse a pg_service.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            services: 0,
            settings: 0,
            named: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.services += 1;
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            let key = s[..eq].trim();
            if key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                c.settings += 1;
                if KEYS.contains(&key) {
                    c.named += 1;
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
    fn parses_services() {
        let b = concat!(
            "# pg_service.conf\n",
            "[maindb]\n",
            "host=db.internal\n",
            "port=5433\n",
            "dbname=app\n",
            "user=svc\n",
            "sslmode=verify-full\n",
            "sslrootcert=/etc/ssl/ca.crt\n",
            "connect_timeout=5\n",
            "[readonly]\n",
            "host=replica.internal\n",
            "dbname=app\n",
            "target_session_attrs=read-only\n",
            "application_name=rpt\n",
        );
        let c = Pgservice::parse(b.as_bytes()).unwrap();
        assert_eq!(c.services, 2);
        assert_eq!(c.settings, 11);
        assert_eq!(c.named, 11);
    }

    #[test]
    fn rejects_other() {
        assert!(Pgservice::parse(b"[x]\nfoo=1").is_none());
    }
}

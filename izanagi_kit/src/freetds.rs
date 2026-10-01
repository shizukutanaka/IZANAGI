//! FreeTDS `freetds.conf` census.
//!
//! INI sections: `[global]` for defaults plus one `[servername]` per
//! server. `key = value` settings: `host`/`port`/`tds version`/
//! `instance`/`database`/`dump file`/`dump file append`/`encryption`/
//! `client charset`/`server charset`/`text size`/`timeout`/
//! `connect timeout`/`read timeout`/`write timeout`/`nt domain`/
//! `use ntlmv2`/`use utf-16`/`lanman`/`packet size`/`asa database`/
//! `workstation`/`app name`/`ca file`/`crl file`/`check certificate hostname`/
//! `openssl cnf`/`openssl flags`/`realm`/`spn`/`mutual authentication`/
//! `disable tls`/`tls min`/`tls max`/`auto-block size`/`block size`/
//! `capabilities`/`conf`/`debug flags`/`dump file`/`debug level`/
//! `emulate little endian`/`ntdomain`/`domain`/`mssql version`/
//! `instance`/`kerberos`/`kerberos keytab`/`service name`/`truststore`/
//! `login timeout`/`query timeout`/`suspend`/`no overlap`.
//!
//! ```rust
//! let f = concat!(
//!     "[global]\ntds version = 7.4\n",
//!     "[prod]\nhost = sql1\nport = 1433\ninstance = MSSQLSERVER\n",
//! );
//! let c = izanagi_kit::freetds::Freetds::parse(f.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.servers, 1);
//! ```

/// freetds.conf census.
#[derive(Debug, Clone)]
pub struct Freetds {
    /// All `[section]` headers.
    pub sections: usize,
    /// Non-`[global]` sections (named servers).
    pub servers: usize,
    /// `key = value` settings.
    pub settings: usize,
    /// Recognised FreeTDS option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "host",
    "port",
    "tds version",
    "instance",
    "database",
    "dump file",
    "dump file append",
    "encryption",
    "client charset",
    "server charset",
    "text size",
    "timeout",
    "connect timeout",
    "read timeout",
    "write timeout",
    "nt domain",
    "use ntlmv2",
    "use utf-16",
    "lanman",
    "packet size",
    "asa database",
    "workstation",
    "app name",
    "ca file",
    "crl file",
    "check certificate hostname",
    "openssl cnf",
    "openssl flags",
    "realm",
    "spn",
    "mutual authentication",
    "disable tls",
    "tls min",
    "tls max",
    "auto-block size",
    "block size",
    "capabilities",
    "conf",
    "debug flags",
    "debug level",
    "emulate little endian",
    "ntdomain",
    "domain",
    "mssql version",
    "kerberos",
    "kerberos keytab",
    "service name",
    "truststore",
    "login timeout",
    "query timeout",
    "suspend",
    "no overlap",
    "sybase version",
    "initial lcid",
    "bind",
    "use_utf_16",
];

/// Whether the buffer looks like freetds.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let l = t.to_lowercase();
    l.contains("tds version")
        || l.contains("freetds")
        || l.contains("[global]") && l.contains("instance")
        || l.contains("use ntlmv2")
        || l.contains("client charset")
        || l.contains("nt domain")
}

impl Freetds {
    /// Parse a freetds.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            servers: 0,
            settings: 0,
            named: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                if !s[1..s.len() - 1].eq_ignore_ascii_case("global") {
                    c.servers += 1;
                }
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            let key = s[..eq].trim().to_lowercase();
            if !key.is_empty() {
                c.settings += 1;
                if KEYS.contains(&key.as_str()) {
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
    fn parses_conf() {
        let b = concat!(
            "[global]\n",
            "tds version = 7.4\n",
            "text size = 64512\n",
            "client charset = UTF-8\n",
            "[prod]\n",
            "host = sql1.internal\n",
            "port = 1433\n",
            "instance = MSSQLSERVER\n",
            "encryption = require\n",
            "[legacy]\n",
            "host = sql2\n",
            "tds version = 5.0\n",
            "use ntlmv2 = yes\n",
        );
        let c = Freetds::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.servers, 2);
        assert_eq!(c.settings, 10);
        assert_eq!(c.named, 10);
    }

    #[test]
    fn rejects_other() {
        assert!(Freetds::parse(b"[x]\nfoo=1").is_none());
    }
}

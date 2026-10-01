//! Oracle `tnsnames.ora` census.
//!
//! `NAME = (DESCRIPTION = (ADDRESS_LIST = (ADDRESS = (PROTOCOL = TCP)
//! (HOST = h)(PORT = 1521)) (ADDRESS = …)) (CONNECT_DATA =
//! (SERVICE_NAME = svc)(SID = sid)))` plus `IFILE = path` includes,
//! `#` comments, and `(KEY = value)` keyword nesting up to depth 4.
//! Aliases may be `NAME.WORLD` and comma-separated alias lists
//! `A, B = (DESCRIPTION = …)`.
//!
//! ```rust
//! let t = concat!(
//!     "ORCL = (DESCRIPTION = (ADDRESS = (PROTOCOL = TCP)(HOST = db1)(PORT = 1521)) ",
//!     "(CONNECT_DATA = (SERVICE_NAME = orcl)))\n",
//! );
//! let c = izanagi_kit::tnsnames::Tnsnames::parse(t.as_bytes()).unwrap();
//! assert_eq!(c.names, 1);
//! ```

/// tnsnames.ora census.
#[derive(Debug, Clone)]
pub struct Tnsnames {
    /// Net service names / aliases (top-level `NAME = (` lines; comma lists count each alias).
    pub names: usize,
    /// `(ADDRESS = …)` entries.
    pub addresses: usize,
    /// `KEY = value` keyword assignments inside descriptors.
    pub params: usize,
    /// `IFILE = path` includes.
    pub ifiles: usize,
}

const DESCRIPTORS: &[&str] = &[
    "DESCRIPTION",
    "DESCRIPTION_LIST",
    "ADDRESS",
    "ADDRESS_LIST",
    "CONNECT_DATA",
    "SERVICE_NAME",
    "SID",
    "SDU",
    "SOURCE_ROUTE",
    "FAILOVER",
    "LOAD_BALANCE",
    "HS",
    "SECURITY",
    "SSL_CLIENT_AUTHENTICATION",
    "RECV_BUF_SIZE",
    "SEND_BUF_SIZE",
    "CONNECT_TIMEOUT",
    "TRANSPORT_CONNECT_TIMEOUT",
    "RETRY_COUNT",
    "ENABLE",
    "PROTOCOL",
    "HOST",
    "PORT",
    "IP",
    "KEY",
    "AUTHENTICATION_SERVICE",
    "TYPE_OF_SERVICE",
    "GLOBAL_NAME",
    "SERVER",
    "INSTANCE_NAME",
    "SERVICE_HANDLER",
    "STATIC_LISTENER",
    "QUEUESIZE",
    "COMPRESSION",
    "COMPRESSION_LEVELS",
    "COMPRESSION_ALGORITHMS",
    "WEIGHT",
    "RECV_TIMEOUT",
    "RATE_LIMIT",
    "EXPIRE_TIME",
    "TIMEOUT",
    "RETRY_DELAY",
    "FAILOVER_MODE",
    "METHOD",
    "TYPE",
    "BACKUP",
    "RETRIES",
];

/// Whether the buffer looks like tnsnames.ora.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let u = t.to_uppercase();
    u.contains("(DESCRIPTION")
        || u.contains("(ADDRESS")
        || u.contains("CONNECT_DATA")
        || u.contains("SERVICE_NAME")
        || u.contains("SID")
}

impl Tnsnames {
    /// Parse a tnsnames.ora into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            names: 0,
            addresses: 0,
            params: 0,
            ifiles: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let u = s.to_uppercase();
            c.addresses += u.matches("(ADDRESS =").count();
            if u.starts_with("IFILE") && u.contains('=') {
                c.ifiles += 1;
                continue;
            }
            let mut segs = u.split('(');
            if let Some(head) = segs.next() {
                // `NAME = (` alias head — each comma-separated alias is a name
                if let Some(eq) = head.find('=') {
                    let key = head[..eq].trim();
                    if !key.is_empty()
                        && head[eq + 1..].trim().is_empty()
                        && key.chars().all(|ch| {
                            ch.is_ascii_alphanumeric()
                                || ch == '_'
                                || ch == '.'
                                || ch == ','
                                || ch == ' '
                                || ch == '-'
                        })
                        && !DESCRIPTORS.contains(&key)
                    {
                        c.names += key.split(',').filter(|a| !a.trim().is_empty()).count();
                    }
                }
            }
            for seg in segs {
                let seg = seg.trim();
                let Some(eq) = seg.find('=') else {
                    continue;
                };
                let key = seg[..eq].trim();
                if !key.is_empty()
                    && key
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
                {
                    c.params += 1;
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
    fn parses_names() {
        let b = concat!(
            "# tnsnames\n",
            "ORCL = (DESCRIPTION = (ADDRESS_LIST = (ADDRESS = (PROTOCOL = TCP)(HOST = db1)(PORT = 1521)) (ADDRESS = (PROTOCOL = TCP)(HOST = db2)(PORT = 1521))) (CONNECT_DATA = (SERVICE_NAME = orcl)))\n",
            "ALIAS1, ALIAS2 = (DESCRIPTION = (ADDRESS = (PROTOCOL = TCP)(HOST = db3)(PORT = 1521)) (CONNECT_DATA = (SID = x)))\n",
            "IFILE = /opt/tns/prod.ora\n",
        );
        let c = Tnsnames::parse(b.as_bytes()).unwrap();
        assert_eq!(c.names, 3);
        assert_eq!(c.addresses, 3);
        assert_eq!(c.ifiles, 1);
        assert!(c.params >= 8);
    }

    #[test]
    fn rejects_other() {
        assert!(Tnsnames::parse(b"foo = 1").is_none());
    }
}

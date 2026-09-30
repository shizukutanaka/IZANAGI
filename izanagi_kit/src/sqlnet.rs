//! Oracle `sqlnet.ora` census.
//!
//! Flat `KEY = value` directives (dotted names like
//! `NAMES.DIRECTORY_PATH`, `SQLNET.AUTHENTICATION_SERVICES`,
//! `SQLNET.EXPIRE_TIME`, `TCP.CONNECT_TIMEOUT`, `SSL_VERSION`,
//! `WALLET_LOCATION` whose value is a `(SOURCE = (METHOD = …)
//! (METHOD_DATA = (DIRECTORY = …)))` descriptor), `#` comments,
//! continuation via open parens.
//!
//! ```rust
//! let s = concat!(
//!     "NAMES.DIRECTORY_PATH = (TNSNAMES, EZCONNECT)\n",
//!     "SQLNET.AUTHENTICATION_SERVICES = (NONE)\n",
//! );
//! let c = izanagi_kit::sqlnet::Sqlnet::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.settings, 2);
//! ```

/// sqlnet.ora census.
#[derive(Debug, Clone)]
pub struct Sqlnet {
    /// `KEY = value` directives.
    pub settings: usize,
    /// Dotted (`A.B.C`) key names.
    pub dotted: usize,
    /// Parenthesised `(METHOD = …)`/`(SOURCE = …)` descriptor segments.
    pub descriptors: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Whether the buffer looks like sqlnet.ora.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let u = t.to_uppercase();
    u.contains("NAMES.DIRECTORY_PATH")
        || u.contains("SQLNET.")
        || u.contains("WALLET_LOCATION")
        || u.contains("AUTHENTICATION_SERVICES")
        || u.contains("SSL_SERVER_DN_MATCH")
        || u.contains("TNSNAMES")
}

impl Sqlnet {
    /// Parse a sqlnet.ora into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            dotted: 0,
            descriptors: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let u = s.to_uppercase();
            c.descriptors += u.matches("(METHOD =").count() + u.matches("(SOURCE").count();
            for seg in u.split('(') {
                let Some(eq) = seg.find('=') else {
                    continue;
                };
                let key = seg[..eq].trim();
                if !key.is_empty()
                    && key
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '$')
                {
                    c.settings += 1;
                    if key.contains('.') {
                        c.dotted += 1;
                    }
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
            "# sqlnet.ora\n",
            "NAMES.DIRECTORY_PATH = (TNSNAMES, EZCONNECT)\n",
            "SQLNET.AUTHENTICATION_SERVICES = (NONE)\n",
            "SQLNET.EXPIRE_TIME = 10\n",
            "TCP.CONNECT_TIMEOUT = 5\n",
            "SSL_SERVER_DN_MATCH = YES\n",
            "WALLET_LOCATION = (SOURCE = (METHOD = FILE) (METHOD_DATA = (DIRECTORY = /u01/wallet)))\n",
        );
        let c = Sqlnet::parse(b.as_bytes()).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.settings >= 9);
        assert!(c.dotted >= 4);
        assert_eq!(c.descriptors, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Sqlnet::parse(b"foo = 1").is_none());
    }
}

//! Census of an EasyRSA `vars` file.
//!
//! Shell-style `set_var EASYRSA_* "…"` and `export KEY_*`/`export EASYRSA_*`
//! lines: `EASYRSA_REQ_COUNTRY`/`EASYRSA_REQ_PROVINCE`/`EASYRSA_REQ_CITY`/
//! `EASYRSA_REQ_ORG`/`EASYRSA_REQ_EMAIL`/`EASYRSA_REQ_OU`/`EASYRSA_REQ_CN`,
//! `EASYRSA_DN`, `EASYRSA_CA_EXPIRE`/`EASYRSA_CERT_EXPIRE`/`EASYRSA_CRL_DAYS`,
//! `EASYRSA_ALGO`/`EASYRSA_CURVE`/`EASYRSA_DIGEST`, `EASYRSA_KEY_SIZE`,
//! `EASYRSA_BATCH`, `EASYRSA_OPENSSL`/`EASYRSA_PKI`/`EASYRSA_TEMP_DIR`/
//! `EASYRSA_SSL_CONF`/`EASYRSA_SAFE_CONF`/`EASYRSA_PKCS11`/`EASYRSA_NS_SUPPORT`,
//! plus legacy `KEY_*` exports (`KEY_SIZE`/`KEY_COUNTRY`/`KEY_PROVINCE`/
//! `KEY_CITY`/`KEY_ORG`/`KEY_EMAIL`/`KEY_OU`/`KEY_NAME`/`KEY_CN`/
//! `KEY_EXPIRE`/`KEY_ALTNAMES`/`PKCS11_*`). `#` comments.
//!
//! ```rust
//! let c = izanagi_kit::easyrsa::EasyRsa::parse(
//!     b"set_var EASYRSA_REQ_COUNTRY \"JP\"\nset_var EASYRSA_ALGO ec\n",
//! ).unwrap();
//! assert_eq!(c.set_vars, 2);
//! ```
#![forbid(unsafe_code)]

/// EasyRSA vars census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EasyRsa {
    /// `set_var EASYRSA_*`/`set_var EASYRSA*` assignments.
    pub set_vars: usize,
    /// `export KEY_*`/`export EASYRSA*`/`export PKCS11*` lines.
    pub exports: usize,
    /// `EASYRSA_REQ_*` distinguished-name fields.
    pub req_fields: usize,
    /// Other `NAME=value` assignments.
    pub assignments: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// DN request fields.
const REQ: &[&str] = &[
    "EASYRSA_REQ_COUNTRY",
    "EASYRSA_REQ_PROVINCE",
    "EASYRSA_REQ_CITY",
    "EASYRSA_REQ_ORG",
    "EASYRSA_REQ_EMAIL",
    "EASYRSA_REQ_OU",
    "EASYRSA_REQ_CN",
];

/// True if `b` looks like EasyRSA vars.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("EASYRSA") || t.contains("set_var ") || t.contains("export KEY_")
}

impl EasyRsa {
    /// Parse a vars file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            set_vars: 0,
            exports: 0,
            req_fields: 0,
            assignments: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('#') {
                c.comments += 1;
            } else if l.starts_with("set_var ") {
                c.set_vars += 1;
                if REQ.iter().any(|r| l.contains(r)) {
                    c.req_fields += 1;
                }
            } else if l.starts_with("export ") {
                c.exports += 1;
                if l.contains("EASYRSA_REQ_") {
                    c.req_fields += 1;
                }
            } else if l.contains('=') {
                c.assignments += 1;
            }
        }
        if c.set_vars + c.exports + c.assignments == 0 {
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
            "# vars\n",
            "set_var EASYRSA_REQ_COUNTRY \"JP\"\n",
            "set_var EASYRSA_REQ_PROVINCE \"Tokyo\"\n",
            "set_var EASYRSA_REQ_CITY \"Shibuya\"\n",
            "set_var EASYRSA_REQ_ORG \"Example\"\n",
            "set_var EASYRSA_REQ_EMAIL \"ca@example.com\"\n",
            "set_var EASYRSA_REQ_OU \"IT\"\n",
            "set_var EASYRSA_ALGO ec\n",
            "set_var EASYRSA_CURVE prime256v1\n",
            "set_var EASYRSA_DIGEST sha256\n",
            "set_var EASYRSA_CA_EXPIRE 3650\n",
            "export KEY_SIZE=2048\n",
            "export KEY_NAME=\"server\"\n",
        )
        .to_string()
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = EasyRsa::parse(b.as_bytes()).unwrap();
        assert_eq!(c.set_vars, 10);
        assert_eq!(c.req_fields, 6);
        assert_eq!(c.exports, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(EasyRsa::parse(b"# none\n").is_none());
    }
}

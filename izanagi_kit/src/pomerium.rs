//! Pomerium `config.yaml` identity-aware proxy census.
//!
//! Pomerium top-level keys: `authenticate_service_url`,
//! `authorize_service_url`, `databroker_service_url`,
//! `databroker_service_token`, `shared_secret`,
//! `cookie_secret`, `idp_provider`, `idp_client_id`,
//! `idp_client_secret`, `signing_key`, `forward_auth_url`,
//! `address`, `insecure_server`, `policies`, `routes`,
//! `dns_lookup_family`, `metrics_address`, `tracing`,
//! `autocert`, `certificates`, `jwt_issuer_format`.
//!
//! ```rust
//! let k = b"address: :443\nauthenticate_service_url: https://authenticate.example.com\nidp_provider: google\nshared_secret: x\n";
//! assert!(izanagi_kit::pomerium::detect(k));
//! ```

/// Pomerium config census.
#[derive(Debug, Clone)]
pub struct Pomerium {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "authenticate_service_url",
    "authorize_service_url",
    "databroker_service_url",
    "databroker_service_token",
    "shared_secret",
    "cookie_secret",
    "idp_provider",
    "idp_client_id",
    "idp_client_secret",
    "signing_key",
    "forward_auth_url",
    "idp_service_account",
    "authenticate_internal_service_url",
    "databroker_storage_type",
    "databroker_storage_connection_string",
    "issuer_key_pair",
];

const WEAK: &[&str] = &[
    "address",
    "insecure_server",
    "policies",
    "routes",
    "dns_lookup_family",
    "metrics_address",
    "tracing",
    "autocert",
    "certificates",
    "jwt_issuer_format",
    "grpc_address",
    "http_redirect_addr",
    "timeout_read",
    "timeout_write",
    "headers",
    "set_response_headers",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect a Pomerium config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Pomerium {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
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
    fn detects() {
        let b = b"address: :443\nauthenticate_service_url: https://authenticate.example.com\nidp_provider: google\nshared_secret: x\n";
        assert!(detect(b));
        let c = Pomerium::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"routes:\n- from: x\n"));
        assert!(!detect(
            b"# authenticate_service_url: x\n# shared_secret: y\n"
        ));
    }
}

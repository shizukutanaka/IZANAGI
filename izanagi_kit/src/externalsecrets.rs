//! External Secrets Operator manifest census (`external-secrets.io`/`generators.external-secrets.io`).
//!
//! `apiVersion` in the `external-secrets.io, generators.external-secrets.io` API group(s) together with a
//! `kind` in `ExternalSecret, SecretStore, ClusterSecretStore, ClusterExternalSecret, PushSecret, ClusterPushSecret, Password, ACRAccessToken, ECRAuthorizationToken, Fake, GCRAccessToken, GithubAccessToken, QuayAccessToken, STSSessionToken, VaultDynamicSecret, UUID, Webhook, ClusterGenerator`.
//!
//! ```rust
//! let k = b"apiVersion: external-secrets.io/v1beta1\nkind: ExternalSecret\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
//! assert!(izanagi_kit::externalsecrets::detect(k));
//! ```

/// external-secrets manifest census.
#[derive(Debug, Clone)]
pub struct Externalsecrets {
    /// `kind` value, when present.
    pub kind: Option<String>,
    /// `apiVersion`/`kind`/`metadata`/`spec` envelope lines.
    pub envelope: usize,
    /// `- ` list items.
    pub items: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const GROUPS: &[&str] = &["external-secrets.io", "generators.external-secrets.io"];

const KINDS: &[&str] = &[
    "ExternalSecret",
    "SecretStore",
    "ClusterSecretStore",
    "ClusterExternalSecret",
    "PushSecret",
    "ClusterPushSecret",
    "Password",
    "ACRAccessToken",
    "ECRAuthorizationToken",
    "Fake",
    "GCRAccessToken",
    "GithubAccessToken",
    "QuayAccessToken",
    "STSSessionToken",
    "VaultDynamicSecret",
    "UUID",
    "Webhook",
    "ClusterGenerator",
];

fn value_of(s: &str) -> &str {
    match s.find(':') {
        Some(i) => s[i + 1..].trim().trim_matches('"').trim_matches('\''),
        None => "",
    }
}

fn api_ok(t: &str) -> bool {
    t.lines().any(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("apiVersion") {
            return false;
        }
        let v = value_of(s);
        GROUPS.iter().any(|g| v.starts_with(g))
    })
}

fn kind_val(t: &str) -> Option<String> {
    t.lines().find_map(|l| {
        let s = l.trim();
        if s.starts_with('#') || !s.starts_with("kind") {
            return None;
        }
        let v = value_of(s);
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    })
}

/// Detect an External Secrets manifest.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if !api_ok(t) {
        return false;
    }
    match kind_val(t) {
        Some(k) => KINDS.contains(&k.as_str()),
        None => false,
    }
}

impl Externalsecrets {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kind: kind_val(t),
            envelope: 0,
            items: 0,
            settings: 0,
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
                let k = s[..s.find(':').unwrap_or(s.len())].trim();
                if matches!(k, "apiVersion" | "kind" | "metadata" | "spec") {
                    c.envelope += 1;
                }
                if !k.is_empty() {
                    c.settings += 1;
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
        let b = b"apiVersion: external-secrets.io/v1beta1\nkind: ExternalSecret\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
        assert!(detect(b));
        let c = Externalsecrets::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("ExternalSecret"));
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"apiVersion: v1\nkind: ExternalSecret\n"));
        assert!(!detect(
            b"apiVersion: external-secrets.io/v1beta1\nkind: Deployment\n"
        ));
        assert!(!detect(
            b"# apiVersion: external-secrets.io/v1beta1\nkind: ExternalSecret\n"
        ));
    }
}

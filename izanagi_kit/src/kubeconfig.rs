//! Kubernetes `kubeconfig` (`.kube/config`) YAML format.
//!
//! kubeconfig is `apiVersion: v1` + `kind: Config` with `clusters:`,
//! `users:`, `contexts:` named lists, `current-context:`, `preferences:`,
//! and per-entry `cluster:`/`user:`/`context:` maps carrying `server:`,
//! `certificate-authority`, `token`, `client-certificate`, `exec:`,
//! `auth-provider:` fields.
//!
//! ```
//! let b = concat!(
//!     "apiVersion: v1\n",
//!     "kind: Config\n",
//!     "current-context: dev\n",
//!     "clusters:\n",
//!     "- name: c1\n",
//!     "  cluster:\n",
//!     "    server: https://192 0 2 1\n",
//!     "users:\n",
//!     "- name: u1\n",
//!     "  user:\n",
//!     "    token: abc\n",
//!     "contexts:\n",
//!     "- name: dev\n",
//!     "  context:\n",
//!     "    cluster: c1\n",
//!     "    user: u1\n"
//! ).as_bytes();
//! assert!(izanagi_kit::kubeconfig::detect(b));
//! let c = izanagi_kit::kubeconfig::Kubeconfig::parse(b).unwrap();
//! assert_eq!(c.clusters, 1);
//! assert_eq!(c.contexts, 1);
//! ```

/// Parsed kubeconfig summary.
#[derive(Debug, Clone)]
pub struct Kubeconfig {
    /// `clusters:` `- name:` entries.
    pub clusters: usize,
    /// `users:` `- name:` entries.
    pub users: usize,
    /// `contexts:` `- name:` entries.
    pub contexts: usize,
    /// `current-context:` occurrences.
    pub current_context: usize,
    /// `server:`/`certificate-authority`/`certificate-authority-data`/`insecure-skip-tls-verify`/`tls-server-name` cluster fields.
    pub cluster_fields: usize,
    /// `token`/`client-certificate`/`client-key`/`client-certificate-data`/`client-key-data`/`username`/`password`/`exec:`/`auth-provider:`/`as:`/`as-groups` user fields.
    pub user_fields: usize,
    /// `namespace`/`cluster`/`user`/`extensions` context fields.
    pub context_fields: usize,
    /// `preferences:` entries.
    pub preferences: usize,
}

/// Whether the buffer looks like kubeconfig.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let has_kind = t.contains("kind: Config") || t.contains("current-context:");
    has_kind
        && (t.contains("clusters:")
            || t.contains("contexts:")
            || t.contains("certificate-authority")
            || t.contains("apiVersion"))
}

impl Kubeconfig {
    /// Parses a kubeconfig summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            clusters: 0,
            users: 0,
            contexts: 0,
            current_context: 0,
            cluster_fields: 0,
            user_fields: 0,
            context_fields: 0,
            preferences: 0,
        };
        // Track which top-level list we're inside: c=clusters,u=users,x=contexts.
        let mut scope: Option<u8> = None;
        for l in t.lines() {
            let tr = l.trim();
            match tr {
                "clusters:" => {
                    scope = Some(b'c');
                    continue;
                }
                "users:" => {
                    scope = Some(b'u');
                    continue;
                }
                "contexts:" => {
                    scope = Some(b'x');
                    continue;
                }
                _ => {}
            }
            if !l.starts_with(' ') && !tr.starts_with('-') && !tr.is_empty() {
                scope = None;
            }
            if tr.starts_with("current-context:") {
                c.current_context += 1;
            }
            if tr.starts_with("preferences:") {
                c.preferences += 1;
            }
            if tr.starts_with("- name:") || tr.starts_with("- name: ") {
                match scope {
                    Some(b'c') => c.clusters += 1,
                    Some(b'u') => c.users += 1,
                    Some(b'x') => c.contexts += 1,
                    _ => {}
                }
            }
            for (k, tag) in [
                ("server:", b'c'),
                ("certificate-authority", b'c'),
                ("insecure-skip-tls-verify", b'c'),
                ("tls-server-name", b'c'),
                ("proxy-url", b'c'),
                ("disable-compression", b'c'),
                ("token", b'u'),
                ("client-certificate", b'u'),
                ("client-key", b'u'),
                ("username", b'u'),
                ("password", b'u'),
                ("exec:", b'u'),
                ("auth-provider:", b'u'),
                ("as:", b'u'),
                ("as-groups", b'u'),
                ("namespace:", b'x'),
                ("extensions:", b'x'),
            ] {
                if tr.starts_with(k) {
                    match tag {
                        b'c' => c.cluster_fields += 1,
                        b'u' => c.user_fields += 1,
                        _ => c.context_fields += 1,
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
    fn parses_kubeconfig() {
        let b = concat!(
            "apiVersion: v1\n",
            "kind: Config\n",
            "current-context: dev\n",
            "clusters:\n",
            "- name: c1\n",
            "  cluster:\n",
            "    server: https://192 0 2 1\n",
            "    certificate-authority: /p/ca.crt\n",
            "- name: c2\n",
            "  cluster:\n",
            "    server: https://192 0 2 2\n",
            "users:\n",
            "- name: u1\n",
            "  user:\n",
            "    token: abc\n",
            "    client-certificate: /p/c.crt\n",
            "contexts:\n",
            "- name: dev\n",
            "  context:\n",
            "    cluster: c1\n",
            "    user: u1\n",
            "    namespace: ns\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Kubeconfig::parse(b).unwrap();
        assert_eq!(c.clusters, 2);
        assert_eq!(c.users, 1);
        assert_eq!(c.contexts, 1);
        assert_eq!(c.current_context, 1);
        assert_eq!(c.cluster_fields, 3);
        assert_eq!(c.user_fields, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"apiVersion: apps/v1\nkind: Deployment\n"));
        assert!(Kubeconfig::parse(b"x").is_none());
    }
}

//! Kubernetes Gateway API manifest census (`gateway.networking.k8s.io`).
//!
//! `apiVersion` in the `gateway.networking.k8s.io` API group(s) together with a
//! `kind` in `GatewayClass, Gateway, HTTPRoute, TLSRoute, TCPRoute, UDPRoute, GRPCRoute, ReferenceGrant, ReferencePolicy, BackendTLSPolicy, BackendTrafficPolicy, ListenerSet, XListenerSet, XHTTPRoute, XBackendTrafficPolicy`.
//!
//! ```rust
//! let k = b"apiVersion: gateway.networking.k8s.io/v1\nkind: HTTPRoute\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
//! assert!(izanagi_kit::gatewayapi::detect(k));
//! ```

use crate::textutil::{kind_val, value_of};
/// Gateway API manifest census.
#[derive(Debug, Clone)]
pub struct Gatewayapi {
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

const GROUPS: &[&str] = &["gateway.networking.k8s.io"];

const KINDS: &[&str] = &[
    "GatewayClass",
    "Gateway",
    "HTTPRoute",
    "TLSRoute",
    "TCPRoute",
    "UDPRoute",
    "GRPCRoute",
    "ReferenceGrant",
    "ReferencePolicy",
    "BackendTLSPolicy",
    "BackendTrafficPolicy",
    "ListenerSet",
    "XListenerSet",
    "XHTTPRoute",
    "XBackendTrafficPolicy",
];

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

/// Detect a Gateway API manifest.
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

impl Gatewayapi {
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
        let b = b"apiVersion: gateway.networking.k8s.io/v1\nkind: HTTPRoute\nmetadata:\n  name: demo\nspec:\n  x: 1\n";
        assert!(detect(b));
        let c = Gatewayapi::parse(b).unwrap();
        assert_eq!(c.kind.as_deref(), Some("HTTPRoute"));
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"apiVersion: v1\nkind: HTTPRoute\n"));
        assert!(!detect(
            b"apiVersion: gateway.networking.k8s.io/v1\nkind: Deployment\n"
        ));
        assert!(!detect(
            b"# apiVersion: gateway.networking.k8s.io/v1\nkind: HTTPRoute\n"
        ));
    }
}

//! Linkerd policy CRDs and proxy-config annotations detection and census.
//!
//! Detects `policy.linkerd.io`/`linkerd.io` apiVersions with known kinds
//! (AuthorizationPolicy, HTTPRoute, GRPCRoute, Server, ServerAuthorization,
//! MeshedTLSAuthentication, NetworkAuthentication, EgressNetwork,
//! ExternalWorkload) or `linkerd.io`/`config.linkerd.io` annotation
//! namespaces, and counts the kinds, policy keys (`targetRef`/`group`/
//! `kind`/`name`/`namespace`/`requiredAuthenticationRefs`/`required`/
//! `authentications`/`meshTLS`/`identities`/`serviceAccounts`/
//! `serviceAccountRef`/`unmeshed`/`networks`/`netCidr`/`except`/
//! `proxyProtocol`/`allowProxyProtocol`/`when`/`path`/`not`/`alpn`/
//! `matches`/`headers`/`queryParams`/`method`/`backendRefs`/`filters`/
//! `requestHeaderModifier`/`requestRedirect`/`timeout`/`retry`/`rules`/
//! `timeouts`/`parentRefs`/`hostnames`/`sectionName`/`sessionPersistence`/
//! `sessionName`/`absoluteTimeout`/`idleTimeout`/`lifetime`/`scope`/
//! `targetRefs`/`egressNetworks`/`egressNetowrkRef`/`externalWorkloads`/
//! `meshTLSIdentity`/`networkRef`/`identityRef`/`routes`/`observability`/
//! `accessLog`/`metadata`/`computed`/`metrics`/`tracing`/`grpcRoute`/
//! `http2`/`protocol`/`port`/`podSelector`/`admin`/`control`/`controlp`/
//! `enableAdmin`/`ignores`/`proxyProtocolVersion`/`pathType`/`sni`/`failOpen`/
//! `httproute`/`retryBudget`/`retryable`/`dstOverrides`/`context`/`authority`/
//! `weight`/`condition`/`pathRegex`/`all`/`any`/`ifAny`/`statuses`/`events`),
//! profile/service keys (`service`/`dst`/`routes`/`retryBudget`/`timeout`/
//! `retryable`/`backend`/`condition`/`pathRegex`/`minRetries`/`retryRatio`/
//! `ttl`/`weight`/`dstOverrides`/`context`/`namespace`/`serviceAccount`/
//! `skipFallback`/`budget`/`window`/`maxRetries`/`backoff`/`jitter`/`ttl`/
//! `totalBudget`/`default`/`opaque`/`opaqueTransport`/`protocols`), and
//! `linkerd.io`/`config.linkerd.io`/`config.alpha.linkerd.io`/
//! `identity.linkerd.io`/`policy.linkerd.io`/`viz.linkerd.io`/
//! `multicluster.linkerd.io`/`gateway`/`proxy`/`inject`/`enable`/`opaque`/
//! `cpu`/`memory`/`ephemeral`/`image`/`version`/`log`/`admin`/`control`/
//! `skip`/`ports`/`timeout`/`close`/`await`/`opaque`/`ingress`/`rtt`/`priority`/
//! `discovery`/`hostname` annotations, plus `#` comment lines.
//!
//! ```
//! let b = b"apiVersion: policy.linkerd.io/v1beta3\nkind: AuthorizationPolicy\nmetadata:\n  name: allow\nspec:\n  targetRef:\n    group: policy.linkerd.io\n    kind: Server\n    name: s\n  requiredAuthenticationRefs:\n  - kind: MeshTLSAuthentication\n";
//! assert!(izanagi_kit::linkerd::detect(b));
//! let c = izanagi_kit::linkerd::Linkerd::parse(b).unwrap();
//! assert_eq!(c.kinds, 2);
//! ```

/// Parsed Linkerd manifest summary.
#[derive(Debug, Clone)]
pub struct Linkerd {
    /// Linkerd `kind:` occurrences (AuthorizationPolicy/HTTPRoute/Server/…).
    pub kinds: usize,
    /// policy CRD keys (`targetRef`/`requiredAuthenticationRefs`/`meshTLS`/…).
    pub policy_keys: usize,
    /// service-profile keys (`routes`/`retryBudget`/`dstOverrides`/…).
    pub profile_keys: usize,
    /// `*linkerd.io` annotation occurrences.
    pub annotations: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KINDS: &[&str] = &[
    "kind: AuthorizationPolicy",
    "kind: HTTPRoute",
    "kind: GRPCRoute",
    "kind: Server",
    "kind: ServerAuthorization",
    "kind: MeshedTLSAuthentication",
    "kind: MeshTLSAuthentication",
    "kind: NetworkAuthentication",
    "kind: EgressNetwork",
    "kind: ExternalWorkload",
];

const POLICY_KEYS: &[&str] = &[
    "targetRef:",
    "targetRefs:",
    "requiredAuthenticationRefs:",
    "required:",
    "authentications:",
    "meshTLS:",
    "identities:",
    "serviceAccounts:",
    "serviceAccountRef:",
    "unmeshed:",
    "networks:",
    "netCidr:",
    "except:",
    "proxyProtocol:",
    "allowProxyProtocol:",
    "when:",
    "path:",
    "not:",
    "alpn:",
    "matches:",
    "headers:",
    "queryParams:",
    "method:",
    "backendRefs:",
    "filters:",
    "requestHeaderModifier:",
    "requestRedirect:",
    "timeout:",
    "retry:",
    "rules:",
    "timeouts:",
    "parentRefs:",
    "hostnames:",
    "sectionName:",
    "sessionPersistence:",
    "sessionName:",
    "absoluteTimeout:",
    "idleTimeout:",
    "lifetime:",
    "scope:",
    "egressNetworks:",
    "externalWorkloads:",
    "meshTLSIdentity:",
    "networkRef:",
    "identityRef:",
    "accessLog:",
    "podSelector:",
    "admin:",
    "control:",
    "enableAdmin:",
    "proxyProtocolVersion:",
    "pathType:",
    "sni:",
    "failOpen:",
    "httproute:",
];

const PROFILE_KEYS: &[&str] = &[
    "service:",
    "dst:",
    "routes:",
    "retryBudget:",
    "timeout:",
    "retryable:",
    "backend:",
    "condition:",
    "pathRegex:",
    "minRetries:",
    "retryRatio:",
    "ttl:",
    "weight:",
    "dstOverrides:",
    "context:",
    "namespace:",
    "serviceAccount:",
    "skipFallback:",
    "budget:",
    "window:",
    "maxRetries:",
    "backoff:",
    "jitter:",
    "totalBudget:",
    "default:",
    "opaque:",
    "opaqueTransport:",
    "protocols:",
];

const ANNOTATION_KEYS: &[&str] = &[
    "linkerd.io",
    "config.linkerd.io",
    "config.alpha.linkerd.io",
    "identity.linkerd.io",
    "policy.linkerd.io",
    "viz.linkerd.io",
    "multicluster.linkerd.io",
    "edge.linkerd.io",
    "gateway.linkerd.io",
    "proxy.",
];

/// Detects Linkerd manifests and annotated resources.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    let crd = (t.contains("policy.linkerd.io") || t.contains("linkerd.io/"))
        && KINDS.iter().any(|k| t.contains(k));
    let ann = t.contains("config.linkerd.io/") || t.contains("linkerd.io/inject");
    crd || ann
}

impl Linkerd {
    /// Parses a Linkerd manifest, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            kinds: t
                .lines()
                .filter(|l| KINDS.iter().any(|k| l.trim() == *k))
                .count(),
            policy_keys: 0,
            profile_keys: 0,
            annotations: ANNOTATION_KEYS.iter().map(|k| t.matches(k).count()).sum(),
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let tr = tr.trim_start_matches("- ").trim_start();
            if POLICY_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.policy_keys += 1;
            }
            if PROFILE_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.profile_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_authz_policy() {
        let b = b"apiVersion: policy.linkerd.io/v1beta3\nkind: AuthorizationPolicy\nmetadata:\n  name: allow\nspec:\n  targetRef:\n    group: policy.linkerd.io\n    kind: Server\n    name: s\n  requiredAuthenticationRefs:\n  - kind: MeshTLSAuthentication\n";
        let c = Linkerd::parse(b).unwrap();
        assert_eq!(c.kinds, 2);
        assert!(c.policy_keys >= 2);
        assert!(c.annotations >= 2);
    }

    #[test]
    fn detects_inject_annotation() {
        let b = b"apiVersion: v1\nkind: Pod\nmetadata:\n  annotations:\n    linkerd.io/inject: enabled\n    config.linkerd.io/opaque-ports: \"3306\"\n";
        assert!(detect(b));
        let c = Linkerd::parse(b).unwrap();
        assert!(c.annotations >= 2);
    }

    #[test]
    fn rejects_other_crds() {
        assert!(Linkerd::parse(b"apiVersion: v1\nkind: Pod\n").is_none());
        assert!(!detect(
            b"apiVersion: networking.k8s.io/v1\nkind: Ingress\n"
        ));
    }
}

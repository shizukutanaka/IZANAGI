//! Istio CRD manifest (`networking.istio.io`/`security.istio.io`/
//! `telemetry.istio.io`/`extensions.istio.io`) detection and census.
//!
//! Detects `istio.io/` apiVersions with known kinds (VirtualService,
//! DestinationRule, Gateway, ServiceEntry, WorkloadEntry, WorkloadGroup,
//! PeerAuthentication, AuthorizationPolicy, RequestAuthentication, Sidecar,
//! EnvoyFilter, ProxyConfig, Telemetry, WasmPlugin) and counts the kinds,
//! traffic keys (`hosts`/`gateways`/`http`/`tls`/`tcp`/`route`/`destination`/
//! `subset`/`weight`/`timeout`/`retries`/`retryOn`/`fault`/`delay`/`abort`/
//! `percentage`/`rewrite`/`match`/`mirror`/`mirrorPercentage`/`corsPolicy`/
//! `headers`/`uri`/`prefix`/`exact`/`regex`/`authority`/`scheme`/`method`/
//! `queryParams`/`port`/`name`/`number`/`delegate`/`faultInjectionType`/
//! `httpStatus`/`grpcStatus`/`http2Error`/`trafficPolicy`/`connectionPool`/
//! `loadBalancer`/`consistentHash`/`httpCookie`/`httpHeaderName`/`useSourceIp`/
//! `minimumRingSize`/`localityLbSetting`/`distribute`/`failover`/`from`/`to`/
//! `outlierDetection`/`consecutive5xxErrors`/`interval`/`baseEjectionTime`/
//! `maxEjectionPercent`/`minHealthPercent`/`tcp`/`connectTimeout`/`maxConnections`/
//! `connectionTimeout`/`connectionPool`/`h2UpgradePolicy`/`useClientProtocol`),
//! security/policy keys (`selector`/`matchLabels`/`rules`/`from`/`to`/`when`/
//! `source`/`operation`/`principals`/`requestPrincipals`/`namespaces`/
//! `notNamespaces`/`ipBlocks`/`notIpBlocks`/`remoteIpBlocks`/`ports`/`notPorts`/
//! `hosts`/`notHosts`/`methods`/`paths`/`notPaths`/`key`/`values`/`action`/
//! `ALLOW`/`DENY`/`AUDIT`/`CUSTOM`/`provider`/`istio`/`extAuthz`/`service`/
//! `targetRef`/`targetRefs`/`target`/`policyTargetRef`/`apiVersion`/`group`),
//! TLS keys (`mode`/`ISTIO_MUTUAL`/`MUTUAL`/`PERMISSIVE`/`STRICT`/`SIMPLE`/
//! `DISABLE`/`OPTIONAL_MUTUAL`/`PASSTHROUGH`/`credentialName`/`caCertificates`/
//! `privateKey`/`serverCertificate`/`subjectAltNames`/`verifyCertificateHash`/
//! `verifyCertificateSpiffe`/`minProtocolVersion`/`maxProtocolVersion`/
//! `cipherSuites`/`ecdhCurves`/`httpsRedirect`/`serverName`/`portLevelSettings`/
//! `mtls`/`insecureSkipVerify`/`sni`/`ca_bundle`/`rootCertificates`/`jwksUri`/
//! `issuer`/`audiences`/`jwtRules`/`forwardOriginalToken`/`outputPayloadToHeader`/
//! `bypassEmptyRequests`/`fromHeaders`/`fromParams`/`fromCookies`),
//! endpoint/mesh keys (`endpoints`/`address`/`ports`/`resolution`/`DNS`/
//! `STATIC`/`NONE`/`STRICT_DNS`/`LOGICAL_DNS`/`EDS`/`ROUND_ROBIN`/
//! `LEAST_REQUEST`/`LEAST_CONN`/`RANDOM`/`PASSTHROUGH`/`serviceAccount`/
//! `network`/`locality`/`labels`/`exportTo`/`subjectAltNames`/`workloadSelector`/
//! `egress`/`ingress`/`defaultEndpoint`/`captureMode`/`outboundTrafficPolicy`/
//! `registriesOnly`/`allowAny`/`clusterLocal`/`externalName`/`dnsRefreshRate`/
//! `connectTimeout`/`meshConfig`/`discoverySelectors`/`configPatches`/`applyTo`/
//! `match`/`context`/`patch`/`operation`/`value`/`listener`/`routeConfiguration`/
//! `cluster`/`virtualHost`/`filterChain`/`networkFilter`/`httpFilter`/
//! `accessLogging`/`tracing`/`randomSamplingPercentage`/`metrics`/
//! `disabled`/`metricsOverrides`/`reportInterval`/`envoy`/`priority`), and `#`
//! comment lines.
//!
//! ```
//! let b = b"apiVersion: networking.istio.io/v1beta1\nkind: VirtualService\nmetadata:\n  name: reviews\nspec:\n  hosts:\n  - reviews\n  http:\n  - route:\n    - destination:\n        host: reviews\n        subset: v1\n";
//! assert!(izanagi_kit::istio::detect(b));
//! let c = izanagi_kit::istio::Istio::parse(b).unwrap();
//! assert_eq!(c.kinds, 1);
//! assert!(c.traffic_keys >= 4);
//! ```

/// Parsed Istio manifest summary.
#[derive(Debug, Clone)]
pub struct Istio {
    /// Istio `kind:` occurrences (VirtualService/DestinationRule/Gateway/…).
    pub kinds: usize,
    /// traffic-management keys (`hosts`/`route`/`destination`/`subset`/`retries`/…).
    pub traffic_keys: usize,
    /// authorization/policy keys (`selector`/`rules`/`principals`/`action`/…).
    pub policy_keys: usize,
    /// TLS/JWT keys (`mode`/`credentialName`/`jwksUri`/…).
    pub tls_keys: usize,
    /// endpoint/mesh keys (`endpoints`/`resolution`/`locality`/`exportTo`/…).
    pub endpoint_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KINDS: &[&str] = &[
    "kind: VirtualService",
    "kind: DestinationRule",
    "kind: Gateway",
    "kind: ServiceEntry",
    "kind: WorkloadEntry",
    "kind: WorkloadGroup",
    "kind: PeerAuthentication",
    "kind: AuthorizationPolicy",
    "kind: RequestAuthentication",
    "kind: Sidecar",
    "kind: EnvoyFilter",
    "kind: ProxyConfig",
    "kind: Telemetry",
    "kind: WasmPlugin",
];

const TRAFFIC_KEYS: &[&str] = &[
    "hosts:",
    "gateways:",
    "http:",
    "tls:",
    "tcp:",
    "route:",
    "destination:",
    "subset:",
    "weight:",
    "timeout:",
    "retries:",
    "retryOn:",
    "fault:",
    "delay:",
    "abort:",
    "percentage:",
    "rewrite:",
    "match:",
    "mirror:",
    "mirrorPercentage:",
    "corsPolicy:",
    "headers:",
    "uri:",
    "prefix:",
    "exact:",
    "regex:",
    "authority:",
    "scheme:",
    "method:",
    "queryParams:",
    "port:",
    "name:",
    "number:",
    "delegate:",
    "trafficPolicy:",
    "connectionPool:",
    "loadBalancer:",
    "consistentHash:",
    "httpCookie:",
    "httpHeaderName:",
    "useSourceIp:",
    "minimumRingSize:",
    "localityLbSetting:",
    "distribute:",
    "failover:",
    "from:",
    "to:",
    "outlierDetection:",
    "consecutive5xxErrors:",
    "interval:",
    "baseEjectionTime:",
    "maxEjectionPercent:",
    "minHealthPercent:",
    "connectTimeout:",
    "maxConnections:",
    "connectionTimeout:",
    "h2UpgradePolicy:",
    "useClientProtocol:",
    "respectExpectedTtl:",
    "respect_dns_ttl:",
];

const POLICY_KEYS: &[&str] = &[
    "selector:",
    "matchLabels:",
    "rules:",
    "when:",
    "source:",
    "operation:",
    "principals:",
    "requestPrincipals:",
    "namespaces:",
    "notNamespaces:",
    "ipBlocks:",
    "notIpBlocks:",
    "remoteIpBlocks:",
    "ports:",
    "notPorts:",
    "notHosts:",
    "methods:",
    "paths:",
    "notPaths:",
    "key:",
    "values:",
    "action:",
    "provider:",
    "targetRef:",
    "targetRefs:",
    "target:",
    "policyTargetRef:",
    "group:",
    "custom:",
    "extAuthz:",
    "service:",
];

const TLS_KEYS: &[&str] = &[
    "mode:",
    "credentialName:",
    "caCertificates:",
    "privateKey:",
    "serverCertificate:",
    "subjectAltNames:",
    "verifyCertificateHash:",
    "verifyCertificateSpiffe:",
    "minProtocolVersion:",
    "maxProtocolVersion:",
    "cipherSuites:",
    "ecdhCurves:",
    "httpsRedirect:",
    "serverName:",
    "portLevelSettings:",
    "mtls:",
    "insecureSkipVerify:",
    "sni:",
    "rootCertificates:",
    "jwksUri:",
    "issuer:",
    "audiences:",
    "jwtRules:",
    "forwardOriginalToken:",
    "outputPayloadToHeader:",
    "bypassEmptyRequests:",
    "fromHeaders:",
    "fromParams:",
    "fromCookies:",
    "tlsmode:",
    "ISTIO_MUTUAL",
    "PERMISSIVE",
    "STRICT",
];

const ENDPOINT_KEYS: &[&str] = &[
    "endpoints:",
    "address:",
    "resolution:",
    "serviceAccount:",
    "network:",
    "locality:",
    "labels:",
    "exportTo:",
    "workloadSelector:",
    "egress:",
    "ingress:",
    "defaultEndpoint:",
    "captureMode:",
    "outboundTrafficPolicy:",
    "externalName:",
    "dnsRefreshRate:",
    "discoverySelectors:",
    "configPatches:",
    "applyTo:",
    "patch:",
    "listener:",
    "routeConfiguration:",
    "cluster:",
    "virtualHost:",
    "filterChain:",
    "networkFilter:",
    "httpFilter:",
    "accessLogging:",
    "tracing:",
    "randomSamplingPercentage:",
    "metrics:",
    "disabled:",
    "metricsOverrides:",
    "reportInterval:",
    "envoy:",
    "priority:",
    "meshConfig:",
    "value:",
];

/// Detects Istio manifests.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    t.contains("istio.io/") && KINDS.iter().any(|k| t.contains(k))
}

impl Istio {
    /// Parses an Istio manifest, counting its structural elements.
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
            traffic_keys: 0,
            policy_keys: 0,
            tls_keys: 0,
            endpoint_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let tr = tr.trim_start_matches("- ").trim_start();
            if TRAFFIC_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.traffic_keys += 1;
            }
            if POLICY_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.policy_keys += 1;
            }
            if TLS_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.tls_keys += 1;
            }
            if ENDPOINT_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.endpoint_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_virtual_service() {
        let b = b"apiVersion: networking.istio.io/v1beta1\nkind: VirtualService\nmetadata:\n  name: reviews\nspec:\n  hosts:\n  - reviews\n  http:\n  - route:\n    - destination:\n        host: reviews\n        subset: v1\n      weight: 100\n";
        let c = Istio::parse(b).unwrap();
        assert_eq!(c.kinds, 1);
        assert!(c.traffic_keys >= 4);
    }

    #[test]
    fn detects_authz_policy() {
        let b = b"apiVersion: security.istio.io/v1beta1\nkind: AuthorizationPolicy\nmetadata:\n  name: allow\nspec:\n  action: ALLOW\n  rules:\n  - from:\n    - source:\n        principals: [\"cluster.local/ns/default/sa/x\"]\n";
        let c = Istio::parse(b).unwrap();
        assert_eq!(c.kinds, 1);
        assert!(c.policy_keys >= 2);
    }

    #[test]
    fn rejects_other_crds() {
        assert!(Istio::parse(b"apiVersion: v1\nkind: Pod\n").is_none());
        assert!(!detect(
            b"apiVersion: networking.k8s.io/v1\nkind: Ingress\n"
        ));
    }
}

//! Traefik static (`entryPoints:`/`providers:`/`api:`/`metrics:`/`tracing:`/
//! `log:`/`accessLog:`/`certificatesResolvers:`) and dynamic (`http:`/`tcp:`/
//! `udp:` + `routers:`/`services:`/`middlewares:`/`tls:`) configuration
//! detection and census.
//!
//! Counts static keys (`entryPoints`/`api`/`dashboard`/`debug`/`insecure`/
//! `metrics`/`tracing`/`log`/`accessLog`/`global`/`checkNewVersion`/
//! `sendAnonymousUsage`/`certificatesResolvers`/`experimental`/`ping`/
//! `serversTransport`/`providers`/`plugins`/`pilot`/`hub`/`telemetry`/`spiffe`/
//! `cluster`/`node`/`hostResolver`/`featureFlags`/`lifecycle`/
//! `requestAcceptGraceTimeout`/`graceTimeOut`/`proxyProtocol`/
//! `insecureSkipVerify`/`forwardedHeaders`/`trustedIPs`/`respondingTimeouts`/
//! `readTimeout`/`writeTimeout`/`idleTimeout`), provider keys (`docker`/
//! `kubernetesCRD`/`kubernetesIngress`/`kubernetesGateway`/`file`/`directory`/
//! `watch`/`filename`/`consulCatalog`/`consul`/`nomad`/`ecs`/`http`/`endpoint`/
//! `pollInterval`/`pollTimeout`/`constraint`/`exposedByDefault`/`network`/
//! `defaultRule`/`useBindPortIP`/`swarmMode`/`username`/`password`/`tls`/`ca`/
//! `cert`/`key`/`api`/`token`/`certAuthFile`/`disableServiceExternalTraffic`/
//! `throttle`/`ingressClass`/`namespaces`/`labelSelector`/`ingressEndpoint`/
//! `publishedService`/`allowEmptyServices`/`allowCrossNamespace`/
//! `experimentalChannel`/`gatewayChannel`/`statusAddress`/`ip`/`service`/
//! `entrypoints`/`address`/`port`/`redirections`/`entryPoint`/`to`/`scheme`/
//! `permanent`/`priority`/`options`/`certResolver`/`domains`/`main`/`sans`/
//! `acme`/`email`/`storage`/`caServer`/`keyType`/`dnsChallenge`/`provider`/
//! `httpChallenge`/`tlsChallenge`/`resolvers`/`delayBeforeCheck`/
//! `disablePropagationCheck`/`certificatesDuration`/`certificates`/
//! `certFile`/`keyFile`/`stores`/`defaultCertificate`/`maxAttempts`/
//! `observability`/`accessLogs`/`addInternals`/`http2`/`maxConcurrentStreams`/
//! `http3`/`advertisedPort`/`encodeQuerySemicolons`/`transport`), router keys
//! (`routers`/`rule`/`service`/`entryPoints`/`middlewares`/`priority`/`tls`/
//! `passthrough`/`services`/`servers`/`url`/`healthCheck`/`interval`/`path`/
//! `hostname`/`status`/`expectation`/`headers`/`loadBalancer`/`passHostHeader`/
//! `responseForwarding`/`flushInterval`/`sticky`/`cookie`/`secure`/`httpOnly`/
//! `sameSite`/`maxIdleConnsPerHost`/`disableHTTP2`/`peerCertURI`/
//! `forwardingTimeouts`/`dialTimeout`/`responseHeaderTimeout`/
//! `idleConnTimeout`/`readIdleTimeout`/`pingTimeout`/`weighted`/`weight`/
//! `mirroring`/`mirror`/`percent`/`maxBodyBytes`/`nativeLB`/`plugin`/
//! `datasource`/`maxWebSocketConnections`/`wrr`/`p2c`/`hrw`/`connectionTuning`),
//! middleware keys (`middlewares`/`basicAuth`/`digestAuth`/`forwardAuth`/
//! `headers`/`customRequestHeaders`/`customResponseHeaders`/`accessControlAllow*`/
//! `allowedHosts`/`hostsProxyHeaders`/`sslRedirect`/`sslTemporaryRedirect`/
//! `sslHost`/`sslProxyHeaders`/`sslForceHost`/`stsSeconds`/`stsIncludeSubdomains`/
//! `stsPreload`/`forceSTSHeader`/`frameDeny`/`customFrameOptionsValue`/
//! `contentTypeNosniff`/`browserXssFilter`/`customBrowserXSSValue`/
//! `contentSecurityPolicy`/`referrerPolicy`/`featurePolicy`/
//! `permissionsPolicy`/`publicKey`/`sslUpgrade`/`accessLog`/`addDateHeader`/
//! `addVaryHeader`/`customHeaders`/`accessControlAllowCredentials`/
//! `accessControlAllowHeaders`/`accessControlAllowMethods`/
//! `accessControlAllowOrigin`/`accessControlAllowOriginList`/
//! `accessControlAllowOriginListRegex`/`accessControlExposeHeaders`/
//! `accessControlMaxAge`/`accessControlPreflightHeaders`/`ipWhiteList`/
//! `ipStrategy`/`inFlightReq`/`average`/`period`/`burst`/`rateLimit`/
//! `sourceCriterion`/`ipWhitelist`/`depth`/`excludedIPs`/`requestCapacity`/
//! `compress`/`excludedContentTypes`/`includedContentTypes`/`minResponseBodyBytes`/
//! `encodings`/`defaultEncoding`/`retry`/`attempts`/`initialInterval`/
//! `circuitBreaker`/`expression`/`checkPeriod`/`fallbackDuration`/
//! `recoveryDuration`/`grpcWeb`/`allowOrigins`/`passTLSClientCert`/`pem`/
//! `info`/`notAfter`/`notBefore`/`sans`/`serialNumber`/`subject`/`commonName`/
//! `country`/`locality`/`organization`/`province`/`issuer`/`errorPage`/`status`/
//! `query`/`service`/`chain`/`replacePath`/`replacePathRegex`/`regex`/
//! `replacement`/`stripPrefix`/`prefixes`/`stripPrefixRegex`/`addPrefix`/
//! `redirectRegex`/`redirectScheme`/`scheme`/`port`/`permanent`/`buffering`/
//! `maxRequestBodyBytes`/`memRequestBodyBytes`/`maxResponseBodyBytes`/
//! `memResponseBodyBytes`/`retryExpression`/`responseCode`/`buffering`/
//! `inflightConn`/`amount`/`sourceCriterion`/`tcp`/`udp`/`terminationDelay`), and `#`
//! comment lines.
//!
//! ```
//! let b = b"entryPoints:\n  web:\n    address: \":80\"\n  websecure:\n    address: \":443\"\nproviders:\n  docker: {}\napi:\n  dashboard: true\n";
//! assert!(izanagi_kit::traefik::detect(b));
//! let c = izanagi_kit::traefik::Traefik::parse(b).unwrap();
//! assert!(c.static_keys >= 3);
//! ```

/// Parsed Traefik config summary.
#[derive(Debug, Clone)]
pub struct Traefik {
    /// static keys (`entryPoints`/`api`/`metrics`/`certificatesResolvers`/…).
    pub static_keys: usize,
    /// provider keys (`docker`/`kubernetesCRD`/`file`/`consul`/…).
    pub provider_keys: usize,
    /// router/service keys (`rule`/`loadBalancer`/`servers`/`healthCheck`/…).
    pub router_keys: usize,
    /// middleware keys (`headers`/`rateLimit`/`compress`/`basicAuth`/…).
    pub middleware_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STATIC_KEYS: &[&str] = &[
    "entryPoints:",
    "api:",
    "dashboard:",
    "debug:",
    "insecure:",
    "metrics:",
    "tracing:",
    "log:",
    "accessLog:",
    "global:",
    "checkNewVersion:",
    "sendAnonymousUsage:",
    "certificatesResolvers:",
    "experimental:",
    "ping:",
    "serversTransport:",
    "providers:",
    "plugins:",
    "pilot:",
    "hub:",
    "telemetry:",
    "spiffe:",
    "cluster:",
    "node:",
    "hostResolver:",
    "featureFlags:",
    "lifecycle:",
    "requestAcceptGraceTimeout:",
    "graceTimeOut:",
    "forwardedHeaders:",
    "trustedIPs:",
    "respondingTimeouts:",
    "readTimeout:",
    "writeTimeout:",
    "idleTimeout:",
];

const PROVIDER_KEYS: &[&str] = &[
    "docker:",
    "kubernetesCRD:",
    "kubernetesIngress:",
    "kubernetesGateway:",
    "file:",
    "directory:",
    "watch:",
    "filename:",
    "consulCatalog:",
    "consul:",
    "nomad:",
    "ecs:",
    "endpoint:",
    "pollInterval:",
    "pollTimeout:",
    "constraint:",
    "exposedByDefault:",
    "network:",
    "defaultRule:",
    "useBindPortIP:",
    "swarmMode:",
    "username:",
    "password:",
    "ca:",
    "cert:",
    "key:",
    "token:",
    "certAuthFile:",
    "disableServiceExternalTraffic:",
    "throttle:",
    "ingressClass:",
    "namespaces:",
    "labelSelector:",
    "ingressEndpoint:",
    "publishedService:",
    "allowEmptyServices:",
    "allowCrossNamespace:",
    "experimentalChannel:",
    "gatewayChannel:",
    "statusAddress:",
    "ip:",
    "address:",
    "port:",
    "redirections:",
    "entryPoint:",
    "to:",
    "scheme:",
    "permanent:",
    "priority:",
    "options:",
    "certResolver:",
    "domains:",
    "main:",
    "sans:",
    "acme:",
    "email:",
    "storage:",
    "caServer:",
    "keyType:",
    "dnsChallenge:",
    "provider:",
    "httpChallenge:",
    "tlsChallenge:",
    "resolvers:",
    "delayBeforeCheck:",
    "disablePropagationCheck:",
    "certificatesDuration:",
    "certificates:",
    "certFile:",
    "keyFile:",
    "stores:",
    "defaultCertificate:",
    "maxAttempts:",
    "observability:",
    "accessLogs:",
    "addInternals:",
    "http2:",
    "maxConcurrentStreams:",
    "http3:",
    "advertisedPort:",
    "encodeQuerySemicolons:",
    "transport:",
];

const ROUTER_KEYS: &[&str] = &[
    "routers:",
    "rule:",
    "service:",
    "middlewares:",
    "passthrough:",
    "services:",
    "servers:",
    "url:",
    "healthCheck:",
    "interval:",
    "path:",
    "hostname:",
    "status:",
    "expectation:",
    "loadBalancer:",
    "passHostHeader:",
    "responseForwarding:",
    "flushInterval:",
    "sticky:",
    "cookie:",
    "secure:",
    "httpOnly:",
    "sameSite:",
    "maxIdleConnsPerHost:",
    "disableHTTP2:",
    "peerCertURI:",
    "forwardingTimeouts:",
    "dialTimeout:",
    "responseHeaderTimeout:",
    "idleConnTimeout:",
    "readIdleTimeout:",
    "pingTimeout:",
    "weighted:",
    "weight:",
    "mirroring:",
    "mirror:",
    "percent:",
    "maxBodyBytes:",
    "nativeLB:",
    "plugin:",
    "datasource:",
    "maxWebSocketConnections:",
    "wrr:",
    "p2c:",
    "hrw:",
    "connectionTuning:",
];

const MIDDLEWARE_KEYS: &[&str] = &[
    "basicAuth:",
    "digestAuth:",
    "forwardAuth:",
    "headers:",
    "customRequestHeaders:",
    "customResponseHeaders:",
    "allowedHosts:",
    "hostsProxyHeaders:",
    "sslRedirect:",
    "sslTemporaryRedirect:",
    "sslHost:",
    "sslProxyHeaders:",
    "sslForceHost:",
    "stsSeconds:",
    "stsIncludeSubdomains:",
    "stsPreload:",
    "forceSTSHeader:",
    "frameDeny:",
    "customFrameOptionsValue:",
    "contentTypeNosniff:",
    "browserXssFilter:",
    "customBrowserXSSValue:",
    "contentSecurityPolicy:",
    "referrerPolicy:",
    "permissionsPolicy:",
    "publicKey:",
    "ipWhiteList:",
    "ipStrategy:",
    "inFlightReq:",
    "average:",
    "period:",
    "burst:",
    "rateLimit:",
    "sourceCriterion:",
    "depth:",
    "excludedIPs:",
    "requestCapacity:",
    "compress:",
    "excludedContentTypes:",
    "includedContentTypes:",
    "minResponseBodyBytes:",
    "encodings:",
    "defaultEncoding:",
    "retry:",
    "attempts:",
    "initialInterval:",
    "circuitBreaker:",
    "expression:",
    "checkPeriod:",
    "fallbackDuration:",
    "recoveryDuration:",
    "grpcWeb:",
    "allowOrigins:",
    "passTLSClientCert:",
    "pem:",
    "info:",
    "notAfter:",
    "notBefore:",
    "serialNumber:",
    "subject:",
    "commonName:",
    "country:",
    "locality:",
    "organization:",
    "province:",
    "issuer:",
    "errorPage:",
    "query:",
    "chain:",
    "replacePath:",
    "replacePathRegex:",
    "regex:",
    "replacement:",
    "stripPrefix:",
    "prefixes:",
    "stripPrefixRegex:",
    "addPrefix:",
    "redirectRegex:",
    "redirectScheme:",
    "buffering:",
    "maxRequestBodyBytes:",
    "memRequestBodyBytes:",
    "maxResponseBodyBytes:",
    "memResponseBodyBytes:",
    "retryExpression:",
    "responseCode:",
    "inflightConn:",
    "amount:",
    "terminationDelay:",
];

/// Detects Traefik static/dynamic config files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    t.contains("entryPoints:")
        || (t.contains("providers:")
            && (t.contains("docker:")
                || t.contains("kubernetesCRD:")
                || t.contains("file:")
                || t.contains("consul:")))
        || (t.contains("http:") && t.contains("routers:") && t.contains("services:"))
}

impl Traefik {
    /// Parses a Traefik config, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            static_keys: 0,
            provider_keys: 0,
            router_keys: 0,
            middleware_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let tr = tr.trim_start_matches("- ").trim_start();
            if STATIC_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.static_keys += 1;
            }
            if PROVIDER_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.provider_keys += 1;
            }
            if ROUTER_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.router_keys += 1;
            }
            if MIDDLEWARE_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.middleware_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_static_config() {
        let b = b"entryPoints:\n  web:\n    address: \":80\"\n  websecure:\n    address: \":443\"\nproviders:\n  docker: {}\napi:\n  dashboard: true\nmetrics:\n  prometheus: {}\n";
        let c = Traefik::parse(b).unwrap();
        assert!(c.static_keys >= 3);
        assert!(c.provider_keys >= 2);
    }

    #[test]
    fn detects_dynamic_config() {
        let b = b"http:\n  routers:\n    web-router:\n      rule: \"Host(`example.com`)\"\n      service: web-svc\n  services:\n    web-svc:\n      loadBalancer:\n        servers:\n        - url: \"http://127:8080\"\n";
        let c = Traefik::parse(b).unwrap();
        assert!(c.router_keys >= 4);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Traefik::parse(b"key: value\n").is_none());
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }
}

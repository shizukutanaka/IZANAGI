//! Dex `config.yaml` census.
//!
//! `issuer:` + `storage:`/`web:`/`grpc:`/`telemetry:`/`oauth2:`/
//! `expiry:`/`logger:`/`frontend:`/`connectors:` + `staticClients:`/
//! `staticPasswords:`/`enablePasswordDB`/`oauth2.alwaysShowLoginScreen`.
//!
//! ```rust
//! let d = "issuer: https://dex.example.com\nstorage:\n  type: memory\nconnectors:\n  - type: github\n    id: gh\nstaticClients:\n  - id: app\n    secret: s\n";
//! let c = izanagi_kit::dexidp::Dexidp::parse(d.as_bytes()).unwrap();
//! assert_eq!(c.connectors, 1);
//! assert_eq!(c.clients, 1);
//! ```

/// Dex config census.
#[derive(Debug, Clone)]
pub struct Dexidp {
    /// Top-level `key:` section headers.
    pub sections: usize,
    /// Nested `key:`/`key: value` settings.
    pub settings: usize,
    /// `connectors` list `- type:` entries.
    pub connectors: usize,
    /// `staticClients` `- id:`/`- name:` entries.
    pub clients: usize,
    /// Recognised Dex option names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "issuer",
    "storage",
    "type",
    "config",
    "web",
    "http",
    "https",
    "telemet",
    "telemetry",
    "oauth2",
    "expiry",
    "logger",
    "frontend",
    "connectors",
    "staticClients",
    "staticPasswords",
    "enablePasswordDB",
    "grpc",
    "addr",
    "reflection",
    "pkgName",
    "certFile",
    "keyFile",
    "clientCAFile",
    "skipApprovalScreen",
    "allowedOrigins",
    "responseTypes",
    "passwordConnector",
    "alwaysShowLoginScreen",
    "authRequests",
    "accessTokens",
    "refreshTokens",
    "offlineSessions",
    "signingKeys",
    "idToken",
    "refreshRequestPolicy",
    "deviceRequests",
    "level",
    "format",
    "dir",
    "issuerURLPath",
    "theme",
    "fromHTTPVersion",
    "databaseURL",
    "ssl",
    "maxOpenConnections",
    "maxIdleConnections",
    "connectionLifetime",
    "host",
    "port",
    "database",
    "user",
    "password",
    "keys",
    "secrets",
    "keysSecrets",
    "inClusterConfig",
    "kubeConfigPath",
    "region",
    "bucket",
    "endpoint",
    "accessKeyID",
    "secretAccessKey",
    "instance",
    "keyFile",
    "keyJSON",
    "projectID",
    "vaultAddr",
    "caCert",
    "clientCert",
    "clientKey",
    "namespace",
    "role",
    "authPath",
    "approle",
    "tokenPath",
    "server",
    "bindDN",
    "bindPW",
    "usernamePrompt",
    "userSearch",
    "groupSearch",
    "baseDN",
    "filter",
    "username",
    "idAttr",
    "emailAttr",
    "nameAttr",
    "preferredUsernameAttr",
    "groups",
    "nameAttr",
    "memberAttr",
    "org",
    "orgs",
    "teams",
    "teamNameField",
    "useLoginAsID",
    "hostName",
    "rootCA",
    "insecureNoSSL",
    "insecureSkipVerify",
    "startTLS",
    "clientID",
    "clientSecret",
    "redirectURI",
    "orgName",
    "loadAllGroups",
    "teamIds",
    "useType",
    "getUserInfo",
    "tenant",
    "domainHint",
    "overrideClaimMapping",
    "scopes",
    "promptType",
    "acrValues",
    "userNameKey",
    "claimMapping",
    "userName",
    "groupsKey",
    "emailKey",
    "nameKey",
    "preferredUsernameKey",
    "issuer",
    "basicAuthUnsupported",
    "tokenURL",
    "authorizationURL",
    "userInfoURL",
    "insecureSkipEmailVerified",
    "claimMutations",
    "newGroupFromClaim",
    "id",
    "name",
    "secret",
    "public",
    "logoURL",
    "redirectURIs",
    "trustedPeers",
    "peer",
    "email",
    "hash",
    "fromConfig",
    "callbacks",
    "deviceRequestsExpiry",
    "keysName",
    "allowedDomains",
    "unixSocket",
    "discovery",
];

/// Whether the buffer looks like a Dex config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("staticClients")
        || t.contains("enablePasswordDB")
        || (t.contains("issuer:") && t.contains("connectors:"))
        || t.contains("dex.coreos.com")
        || t.contains("refreshRequestPolicy")
        || t.contains("skipApprovalScreen")
}

impl Dexidp {
    /// Parse a config.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            connectors: 0,
            clients: 0,
            named: 0,
        };
        let mut ctx = "";
        for l in t.lines() {
            if l.trim().is_empty() || l.trim_start().starts_with('#') {
                continue;
            }
            let s = l.trim_end();
            let indent = s.len() - s.trim_start().len();
            let s = s.trim();
            if indent == 0 {
                if let Some(colon) = s.find(':') {
                    ctx = &s[..colon];
                    c.sections += 1;
                    if KEYS.contains(&ctx) {
                        c.named += 1;
                    }
                }
                continue;
            }
            if let Some(item) = s.strip_prefix('-') {
                let item = item.trim();
                if ctx == "connectors" && item.starts_with("type:") {
                    c.connectors += 1;
                } else if ctx == "staticClients" && item.starts_with("id:") {
                    c.clients += 1;
                }
                c.settings += usize::from(item.contains(':'));
            } else if let Some(colon) = s.find(':') {
                c.settings += 1;
                let key = s[..colon].trim();
                if KEYS.contains(&key) {
                    c.named += 1;
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
    fn parses_config() {
        let b = concat!(
            "issuer: https://dex.example.com\n",
            "storage:\n",
            "  type: postgres\n",
            "  config:\n",
            "    host: db\n",
            "    port: 5432\n",
            "    database: dex\n",
            "web:\n",
            "  http: 0.0.0.0:5556\n",
            "  allowedOrigins: ['*']\n",
            "oauth2:\n",
            "  responseTypes: [code, token, id_token]\n",
            "  skipApprovalScreen: false\n",
            "expiry:\n",
            "  idToken: 24h\n",
            "  refreshRequestPolicy: offline_access\n",
            "logger:\n",
            "  level: debug\n",
            "frontend:\n",
            "  dir: /srv/dex/web\n",
            "connectors:\n",
            "  - type: github\n",
            "    id: github\n",
            "    name: GitHub\n",
            "    config:\n",
            "      clientID: abc\n",
            "      clientSecret: s\n",
            "      redirectURI: https://dex/callback\n",
            "      orgs:\n",
            "        - name: myorg\n",
            "  - type: ldap\n",
            "    id: ldap1\n",
            "    name: LDAP\n",
            "    config:\n",
            "      host: ldap:389\n",
            "staticClients:\n",
            "  - id: example-app\n",
            "    secret: zxcbn\n",
            "    name: 'Example App'\n",
            "    redirectURIs:\n",
            "      - 'http://localhost:5555/callback'\n",
            "  - id: other-app\n",
            "    public: true\n",
            "enablePasswordDB: true\n",
            "staticPasswords:\n",
            "  - email: admin@example.com\n",
            "    hash: $2a$10$xxx\n",
            "    username: admin\n",
        );
        let c = Dexidp::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 11);
        assert_eq!(c.connectors, 2);
        assert_eq!(c.clients, 2);
        assert!(c.settings >= 25);
        assert!(c.named >= 20);
    }

    #[test]
    fn rejects_other() {
        assert!(Dexidp::parse(b"foo: 1").is_none());
    }
}

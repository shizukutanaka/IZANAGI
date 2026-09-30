//! Keycloak realm export JSON census.
//!
//! `"realm": "name"` + top-level arrays `clients`/`users`/`roles`/`groups`/
//! `identityProviders`/`authenticationFlows`/`requiredActions`/`components`/
//! `clientScopes`/`defaultGroups`/`browserSecurityHeaders`/`smtpServer`.
//!
//! ```rust
//! let k = r#"{"realm": "demo", "clients": [{"clientId": "app"}], "users": [{"username": "u"}]}"#;
//! let c = izanagi_kit::keycloak::Keycloak::parse(k.as_bytes()).unwrap();
//! assert_eq!(c.clients, 1);
//! assert_eq!(c.users, 1);
//! ```

/// Keycloak realm export census.
#[derive(Debug, Clone)]
pub struct Keycloak {
    /// `"realm"` field present.
    pub realm: bool,
    /// Top-level collection keys seen (`clients`/`users`/…).
    pub sections: usize,
    /// `"clientId"` fields (client entries).
    pub clients: usize,
    /// `"username"` fields (user entries).
    pub users: usize,
    /// `"providerId"` fields (identity provider entries).
    pub idps: usize,
}

const SECTIONS: &[&str] = &[
    "\"clients\"",
    "\"users\"",
    "\"roles\"",
    "\"groups\"",
    "\"identityProviders\"",
    "\"authenticationFlows\"",
    "\"requiredActions\"",
    "\"components\"",
    "\"clientScopes\"",
    "\"defaultGroups\"",
    "\"browserSecurityHeaders\"",
    "\"smtpServer\"",
    "\"internationalizationEnabled\"",
    "\"supportedLocales\"",
    "\"authenticationBindings\"",
    "\"clientAuthenticationFlow\"",
    "\"keycloakVersion\"",
    "\"realmRoles\"",
    "\"clientRoles\"",
    "\"defaultRoles\"",
];

/// Whether the buffer looks like a Keycloak realm export.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"realm\"")
        && (t.contains("\"clientId\"")
            || t.contains("\"username\"")
            || t.contains("\"identityProviders\"")
            || t.contains("\"authenticationFlows\""))
        || t.contains("\"keycloakVersion\"")
}

impl Keycloak {
    /// Parse a realm export into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        Some(Self {
            realm: t.contains("\"realm\""),
            sections: SECTIONS.iter().filter(|k| t.contains(**k)).count(),
            clients: t.matches("\"clientId\"").count(),
            users: t.matches("\"username\"").count(),
            idps: t.matches("\"providerId\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_realm() {
        let b = concat!(
            "{\n",
            "  \"realm\": \"demo\",\n",
            "  \"clients\": [\n",
            "    {\"clientId\": \"app-a\", \"publicClient\": true},\n",
            "    {\"clientId\": \"app-b\"}\n",
            "  ],\n",
            "  \"users\": [\n",
            "    {\"username\": \"alice\"},\n",
            "    {\"username\": \"bob\"}\n",
            "  ],\n",
            "  \"identityProviders\": [\n",
            "    {\"alias\": \"github\", \"providerId\": \"github\"},\n",
            "    {\"alias\": \"google\", \"providerId\": \"google\"}\n",
            "  ],\n",
            "  \"authenticationFlows\": [],\n",
            "  \"requiredActions\": [],\n",
            "  \"components\": {}\n",
            "}\n",
        );
        let c = Keycloak::parse(b.as_bytes()).unwrap();
        assert!(c.realm);
        assert_eq!(c.clients, 2);
        assert_eq!(c.users, 2);
        assert_eq!(c.idps, 2);
        assert!(c.sections >= 5);
    }

    #[test]
    fn rejects_other() {
        assert!(Keycloak::parse(b"{\"a\":1}").is_none());
    }
}

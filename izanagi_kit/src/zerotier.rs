//! ZeroTier `local.conf` census.
//!
//! JSON object with `settings` member: `primaryPort`,
//! `secondaryPort`, `tertiaryPort`, `portMappingEnabled`,
//! `softwareUpdate`, `softwareUpdateChannel`,
//! `softwareUpdateDist`, `allowManagementFrom`,
//! `allowTcpFallbackRelay`, `forceTcpRelay`,
//! `secondaryPortLock`, `interfacePrefixBlacklist`,
//! `multipathMode`, `multipathPolicy`,
//! `multipathBondPolicy`, `defaultBondingPolicy`,
//! `bondingPolicyUsed`, `policies`,
//! `allowSoDefaultRoute`, `lowBandwidthMode`,
//! `upnpEnabled`, `disablePortMapping`,
//! `ssoEnabled`, `settings`/`trustedPatchPath`,
//! `allowSecondaryPortLocking`, `tcpFallbackRelay`,
//! `enableWebRoot`, `webRoot`, `ssoTimeout`.
//!
//! ```rust
//! let z = "{\n\"settings\": {\n\"primaryPort\": 9994,\n\"softwareUpdate\": \"disable\"\n}\n}\n";
//! let c = izanagi_kit::zerotier::Zerotier::parse(z.as_bytes()).unwrap();
//! assert_eq!(c.settings, 3);
//! ```

/// zerotier local.conf census.
#[derive(Debug, Clone)]
pub struct Zerotier {
    /// `settings`/`virtual`/`physical` objects.
    pub objects: usize,
    /// `key: value` pairs.
    pub settings: usize,
    /// Recognised local.conf keys.
    pub named: usize,
    /// `//` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "settings",
    "primaryPort",
    "secondaryPort",
    "tertiaryPort",
    "portMappingEnabled",
    "softwareUpdate",
    "softwareUpdateChannel",
    "softwareUpdateDist",
    "allowManagementFrom",
    "allowTcpFallbackRelay",
    "forceTcpRelay",
    "secondaryPortLock",
    "interfacePrefixBlacklist",
    "multipathMode",
    "multipathPolicy",
    "multipathBondPolicy",
    "defaultBondingPolicy",
    "allowSecondaryPortLocking",
    "tcpFallbackRelay",
    "allowSoDefaultRoute",
    "lowBandwidthMode",
    "upnpEnabled",
    "disablePortMapping",
    "trustedPatchPath",
    "enableWebRoot",
    "webRoot",
    "ssoEnabled",
    "ssoTimeout",
    "settingsVersion",
    "trustedDevices",
    "virtual",
    "physical",
    "try",
    "blacklist",
    "prefix",
    "routerConf",
    "trustedPeers",
    "moon",
    "roots",
    "controllers",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect zerotier local.conf content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim().trim_start_matches('{').trim_start();
        if s.is_empty() || s.starts_with("//") {
            continue;
        }
        if let Some(rest) = s.strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                let key = &rest[..end];
                if KEYS.contains(&key) {
                    hits += 1;
                }
            }
        }
    }
    hits >= 1 && t.contains('{')
}

impl Zerotier {
    /// Census a local.conf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            objects: 0,
            settings: 0,
            named: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") {
                c.comments += 1;
                continue;
            }
            for seg in s.split(',') {
                let inner = seg.trim().trim_start_matches('{').trim();
                if inner.is_empty() || inner == "{" || inner.starts_with('}') {
                    continue;
                }
                if let Some(rest) = inner.strip_prefix('"') {
                    if let Some(end) = rest.find('"') {
                        let key = &rest[..end];
                        if !key.is_empty()
                            && inner.len() > end + 2
                            && inner[end + 2..].trim_start().starts_with(':')
                        {
                            c.settings += 1;
                            if KEYS.contains(&key) {
                                c.named += 1;
                                if matches!(key, "settings" | "virtual" | "physical" | "routerConf")
                                {
                                    c.objects += 1;
                                }
                            }
                        }
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
    fn detects_conf() {
        let b = b"{\n\"settings\": { \"primaryPort\": 9994 }\n}\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"a\":1}"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_conf() {
        let b = concat!(
            "// zerotier local.conf\n",
            "{\n",
            "  \"settings\": {\n",
            "    \"primaryPort\": 9994,\n",
            "    \"secondaryPort\": 0,\n",
            "    \"tertiaryPort\": 0,\n",
            "    \"portMappingEnabled\": true,\n",
            "    \"softwareUpdate\": \"disable\",\n",
            "    \"softwareUpdateChannel\": \"release\",\n",
            "    \"allowManagementFrom\": [\"192.168.1.0/24\"],\n",
            "    \"allowTcpFallbackRelay\": true,\n",
            "    \"forceTcpRelay\": false,\n",
            "    \"interfacePrefixBlacklist\": [\"zt\"],\n",
            "    \"multipathMode\": 0,\n",
            "    \"disablePortMapping\": true\n",
            "  },\n",
            "  \"virtual\": {\n",
            "    \"deadbeeffe\": { \"try\": [\"10.0.0.1/9993\"] }\n",
            "  }\n",
            "}\n",
        );
        let c = Zerotier::parse(b.as_bytes()).unwrap();
        assert_eq!(c.objects, 2);
        assert!(c.settings >= 14);
        assert!(c.named >= 14);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

//! Census of a smallstep `step-ca` `ca.json` file.
//!
//! JSON config: `root`/`crt`/`key`/`address`/`insecureAddress`,
//! `dnsNames` array, `logger`, `federatedRoots`, `monitoring`,
//! `authority` → `provisioners` (`"type"` JWK/OIDC/X5C/K8sSA/
//! ACME/SSHPOP/NEbula/SCEP/CloudIAP/GCP/AWS/Azure/KMS),
//! `claims` (`minTLSCertDuration`/`maxTLSCertDuration`/
//! `defaultTLSCertDuration`/`disableRenewal`/`allowRenewalAfterExpiry`/
//! `enableSSHCA`), `template`, `policy`, `keys`, `backdate`,
//! `db` (`type`/`dataSource`/`badgerFileLoadingMode`),
//! `tls` (`cipherSuites`/`minVersion`/`maxVersion`/`renegotiation`),
//! `password`, `ssh`/`sshProxy`/`variables`/`identity`.
//! Counts top keys, provisioner objects, types, dnsNames.
//!
//! ```rust
//! let c = izanagi_kit::stepca::StepCa::parse(
//!     b"{\"address\":\":9000\",\"dnsNames\":[\"ca.local\"],\
//!        \"authority\":{\"provisioners\":[{\"type\":\"JWK\",\"name\":\"admin\"}]}}",
//! ).unwrap();
//! assert_eq!(c.provisioners, 1);
//! ```
#![forbid(unsafe_code)]

/// step-ca ca.json census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepCa {
    /// `"type": "<kind>"` provisioner/object type markers.
    pub provisioners: usize,
    /// `"dnsNames"` array entries.
    pub dns_names: usize,
    /// `claims`-section keys.
    pub claims: usize,
    /// Other `"key":` entries.
    pub keys: usize,
}

/// Provisioner type names.
const PROVISIONERS: &[&str] = &[
    "JWK", "OIDC", "X5C", "K8sSA", "ACME", "SSHPOP", "Nebula", "SCEP", "CloudIAP", "GCP", "AWS",
    "Azure", "KMS", "noop",
];

/// Claims keys.
const CLAIMS: &[&str] = &[
    "minTLSCertDuration",
    "maxTLSCertDuration",
    "defaultTLSCertDuration",
    "minUserSSHCertDuration",
    "maxUserSSHCertDuration",
    "defaultUserSSHCertDuration",
    "minHostSSHCertDuration",
    "maxHostSSHCertDuration",
    "defaultHostSSHCertDuration",
    "disableRenewal",
    "allowRenewalAfterExpiry",
    "enableSSHCA",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// True if `b` looks like step-ca ca.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.trim_start().starts_with('{')
        && t.contains("\"provisioners\"")
        && (t.contains("\"authority\"") || t.contains("\"dnsNames\"") || t.contains("\"root\""))
}

impl StepCa {
    /// Parse ca.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        if !t.trim_start().starts_with('{') {
            return None;
        }
        let mut c = Self {
            provisioners: 0,
            dns_names: 0,
            claims: 0,
            keys: 0,
        };
        if let Some(d) = t.find("\"dnsNames\"") {
            let after = &t[d..];
            if let Some(os) = after.find('[') {
                if let Some(ce) = after[os..].find(']') {
                    let arr = &after[os + 1..os + ce];
                    c.dns_names = arr.matches(',').count() + usize::from(arr.contains('"'));
                }
            }
        }
        let mut rest = t;
        while let Some(q) = rest.find('"') {
            let after = &rest[q + 1..];
            let Some(end) = after.find('"') else {
                break;
            };
            let key = &after[..end];
            if after[end + 1..].trim_start().starts_with(':') {
                if key == "type" {
                    c.provisioners += 1;
                } else if CLAIMS.contains(&key) {
                    c.claims += 1;
                } else if key != "dnsNames" && !PROVISIONERS.contains(&key) {
                    c.keys += 1;
                }
            }
            rest = &after[end..];
        }
        if c.provisioners == 0 && c.keys == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        concat!(
            "{\"root\":\"root_ca.crt\",\"crt\":\"inter.crt\",\"key\":\"inter.key\",",
            "\"address\":\":9000\",\"dnsNames\":[\"ca.local\",\"ca2.local\"],",
            "\"authority\":{\"provisioners\":[{\"type\":\"JWK\",\"name\":\"admin\"},",
            "{\"type\":\"OIDC\",\"name\":\"google\"}],",
            "\"claims\":{\"minTLSCertDuration\":\"5m\",\"maxTLSCertDuration\":\"24h\"}},",
            "\"db\":{\"type\":\"badgerv2\",\"dataSource\":\"db\"}}",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = StepCa::parse(b.as_bytes()).unwrap();
        assert_eq!(c.provisioners, 3);
        assert_eq!(c.dns_names, 2);
        assert_eq!(c.claims, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(StepCa::parse(b"x").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

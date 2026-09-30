//! SOPS `.sops.yaml` census.
//!
//! `.sops.yaml` holds `creation_rules:` entries, each selecting files
//! via `path_regex:` and naming key sources —
//! `kms:`/`gcp_kms:`/`azure_kv:`/`hc_vault:`/`age:`/`pgp:`/
//! `key_groups:`/`shamir_threshold`/`unencrypted_suffix`/
//! `encrypted_regex`/`encrypted_suffix`/`mac_only_encrypted`/
//! `stores:`/`destination:`/`oidc:`/`vault:`/
//! `kms_encryption_context:`/`aws_profile`.
//!
//! ```rust
//! let c = izanagi_kit::sops::Sops::parse(b"creation_rules:\n  - path_regex: .*\n    kms: arn\n  - path_regex: x\n    age: age1\n").unwrap();
//! assert_eq!(c.rules, 2);
//! ```

/// `.sops.yaml` census.
#[derive(Debug, Clone)]
pub struct Sops {
    /// `- path_regex:` creation-rule entries.
    pub rules: usize,
    /// Named key sources (`kms:`/`age:`/`pgp:`/`azure_kv:`/`gcp_kms:`/`hc_vault:`).
    pub key_sources: usize,
    /// Other `key:`/`key: value` settings.
    pub settings: usize,
    /// `#` comments.
    pub comments: usize,
}

const KEY_SOURCES: &[&str] = &[
    "kms",
    "gcp_kms",
    "azure_kv",
    "hc_vault",
    "age",
    "pgp",
    "key_groups",
    "kms_encryption_context",
    "aws_profile",
];

/// Whether the buffer looks like a `.sops.yaml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("creation_rules")
        && (t.contains("path_regex") || KEY_SOURCES.iter().any(|k| t.contains(&format!("{k}:"))))
}

impl Sops {
    /// Parse a `.sops.yaml` into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            rules: 0,
            key_sources: 0,
            settings: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") || s == "-" {
                let body = s.trim_start_matches('-').trim();
                if body.starts_with("path_regex:") {
                    c.rules += 1;
                } else if body.ends_with(':') || body.contains(": ") {
                    let key = body.split(':').next().unwrap_or("");
                    if KEY_SOURCES.contains(&key) {
                        c.key_sources += 1;
                    } else if !key.is_empty() {
                        c.settings += 1;
                    }
                }
                continue;
            }
            if s.ends_with(':') || s.contains(": ") {
                let key = s.split(':').next().unwrap_or("");
                if KEY_SOURCES.contains(&key) {
                    c.key_sources += 1;
                } else {
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
    fn parses_sops_yaml() {
        let b = concat!(
            "creation_rules:\n",
            "  - path_regex: .*/staging/.*\n",
            "    kms: arn:aws:kms:us-east-1:123:key/abc\n",
            "    age: age1abcd\n",
            "    encrypted_regex: '^(data|stringData)$'\n",
            "  - path_regex: .*/prod/.*\n",
            "    pgp: ABCDEF\n",
            "    azure_kv: https://vault.example/keys/k1\n",
            "    shamir_threshold: 3\n",
            "stores:\n",
            "  yaml:\n",
            "    indent: 4\n",
            "# comment\n",
        );
        let c = Sops::parse(b.as_bytes()).unwrap();
        assert_eq!(c.rules, 2);
        assert_eq!(c.key_sources, 4);
        assert_eq!(c.settings, 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Sops::parse(b"foo: bar\n").is_none());
    }
}

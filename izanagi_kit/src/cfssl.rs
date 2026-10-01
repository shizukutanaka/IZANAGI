//! Census of a CFSSL `config.json` file.
//!
//! Signing config: `"signing"` → `"default"` + `"profiles"` with
//! per-profile `"expiry"`/`"usages"`/`"issuer_urls"`/`"ocsp_url"`/
//! `"crl_url"`/`"ca_constraint"`/`"not_before"`/`"not_after"`/
//! `"backdate"`/`"auth_key"`/`"remote"`/`"policies"`,
//! `"auth_keys"` → `"key1"` `{ "key": "…", "type": "standard" }`,
//! `"remotes"` → `"name": "host:port"` map.
//! Counts profiles, usages, auth_keys, remotes, keys.
//!
//! ```rust
//! let c = izanagi_kit::cfssl::Cfssl::parse(
//!     b"{\"signing\":{\"profiles\":{\"www\":{\"expiry\":\"168h\"}}},\
//!        \"auth_keys\":{\"key1\":{\"key\":\"abc\"}}}",
//! ).unwrap();
//! assert_eq!(c.profiles, 1);
//! assert_eq!(c.auth_keys, 1);
//! ```
#![forbid(unsafe_code)]

/// cfssl config.json census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cfssl {
    /// `"profiles"` named profile objects.
    pub profiles: usize,
    /// `"usages"` entries.
    pub usages: usize,
    /// `"auth_keys"` named keys.
    pub auth_keys: usize,
    /// `"remotes"` named endpoints.
    pub remotes: usize,
    /// Other `"key":` entries.
    pub keys: usize,
}

/// Field keys inside signing config.
const FIELDS: &[&str] = &[
    "expiry",
    "issuer_urls",
    "ocsp_url",
    "crl_url",
    "ca_constraint",
    "not_before",
    "not_after",
    "backdate",
    "auth_key",
    "remote",
    "policies",
    "default",
    "signing",
    "profiles",
    "auth_keys",
    "remotes",
    "usages",
    "expirys",
    "key",
    "type",
    "url",
];

/// True if `b` looks like a cfssl config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.trim_start().starts_with('{')
        && (t.contains("\"signing\"") || t.contains("\"auth_keys\""))
        && (t.contains("\"expiry\"") || t.contains("\"profiles\"") || t.contains("\"usages\""))
}

/// Count quoted-name objects (`"name": {…}`) directly inside `"section": {…}`.
fn count_named(t: &str, section: &str) -> usize {
    let Some(pos) = t.find(&format!("\"{section}\"")) else {
        return 0;
    };
    let after = &t[pos + section.len() + 2..];
    let Some(open) = after.find('{') else {
        return 0;
    };
    let inner = &after[open + 1..];
    let mut depth = 1usize;
    let mut count = 0usize;
    let mut in_str = false;
    let mut prev = 0u8;
    let bytes = inner.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && depth > 0 {
        let ch = bytes[i];
        if ch == b'"' && prev != b'\\' {
            // find the end of this string
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j] != b'"' || bytes[j - 1] == b'\\') {
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            // is it `"name":` at depth 1 (any value type)?
            if depth == 1 {
                let rest = inner[j + 1..].trim_start();
                if rest.starts_with(':') {
                    count += 1;
                }
            }
            i = j + 1;
            in_str = false;
            continue;
        }
        if !in_str {
            if ch == b'{' {
                depth += 1;
            } else if ch == b'}' {
                depth = depth.saturating_sub(1);
            }
        }
        prev = ch;
        i += 1;
    }
    count
}

impl Cfssl {
    /// Parse config.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !t.trim_start().starts_with('{') {
            return None;
        }
        let c = Self {
            profiles: count_named(t, "profiles"),
            usages: t
                .split("\"usages\"")
                .skip(1)
                .map(|after| {
                    if let Some(os) = after.find('[') {
                        if let Some(ce) = after[os..].find(']') {
                            let arr = &after[os + 1..os + ce];
                            return arr.matches(',').count() + usize::from(arr.contains('"'));
                        }
                    }
                    0
                })
                .sum::<usize>(),
            auth_keys: count_named(t, "auth_keys"),
            remotes: count_named(t, "remotes"),
            keys: {
                let mut n = 0usize;
                let mut rest = t;
                while let Some(q) = rest.find('"') {
                    let after = &rest[q + 1..];
                    let Some(end) = after.find('"') else { break };
                    let key = &after[..end];
                    if after[end + 1..].trim_start().starts_with(':') && !FIELDS.contains(&key) {
                        n += 1;
                    }
                    rest = &after[end..];
                }
                n
            },
        };
        if c.profiles == 0 && c.keys == 0 && c.usages == 0 {
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
            "{\"signing\":{\"default\":{\"expiry\":\"168h\",\"usages\":[\"signing\",\"key encipherment\"]},",
            "\"profiles\":{\"www\":{\"expiry\":\"8760h\",\"usages\":[\"server auth\"]},",
            "\"api\":{\"expiry\":\"1h\"}}},",
            "\"auth_keys\":{\"key1\":{\"key\":\"abc\",\"type\":\"standard\"}},",
            "\"remotes\":{\"ca_server\":\"ca.local:8888\"}}",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Cfssl::parse(b.as_bytes()).unwrap();
        assert_eq!(c.profiles, 2);
        assert_eq!(c.auth_keys, 1);
        assert_eq!(c.remotes, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(Cfssl::parse(b"x").is_none());
    }
}

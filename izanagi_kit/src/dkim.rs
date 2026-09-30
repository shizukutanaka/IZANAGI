//! DKIM key record (RFC 6376 §3.6, the DNS `selector._domainkey`
//! TXT) and `DKIM-Signature:` header fields: semicolon-separated
//! `tag=value` lists — `v=DKIM1`, `k=` key type, `p=` public key
//! (empty = revoked), `n=` notes, `s=` service type, `t=` flags,
//! `h=` hash algorithms.
//!
//! ```
//! let d = izanagi_kit::dkim::parse("v=DKIM1; k=rsa; p=MIIBIjANBg...").unwrap();
//! assert_eq!(d.get("k"), Some("rsa"));
//! assert_eq!(d.pubkey(), Some("MIIBIjANBg..."));
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed DKIM `tag=value` list (key record or signature header).
#[derive(Clone, Debug)]
pub struct Dkim {
    /// Ordered `(tag, value)` pairs, whitespace-trimmed.
    pub tags: Vec<(String, String)>,
}

impl Dkim {
    /// First value of `tag`, trimmed.
    pub fn get(&self, tag: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(k, _)| k == tag)
            .map(|(_, v)| v.as_str())
    }

    /// `p=` value — the base64 public key (empty means revoked).
    pub fn pubkey(&self) -> Option<&str> {
        self.get("p")
    }

    /// `v=` value — `DKIM1` for key records, `1` for signatures.
    pub fn version(&self) -> Option<&str> {
        self.get("v")
    }
}

/// Split a `tag=value` list on `;`, trimming spaces. A term without
/// `=` is ignored (DKIM is forgiving). Returns `None` only when the
/// input yields no tags at all.
pub fn parse(s: &str) -> Option<Dkim> {
    let mut tags = Vec::new();
    for part in s.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim();
            if k.is_empty() {
                continue;
            }
            tags.push((k.to_string(), v.trim().to_string()));
        }
    }
    if tags.is_empty() {
        return None;
    }
    Some(Dkim { tags })
}

/// Parse strictly as a DKIM key record: requires `v=DKIM1` (when `v`
/// is present at all — RFC makes it RECOMMENDED) and a `p=` tag.
pub fn parse_key_record(s: &str) -> Option<Dkim> {
    let d = parse(s)?;
    if let Some(v) = d.get("v") {
        if v != "DKIM1" {
            return None;
        }
    }
    d.get("p")?;
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_record() {
        let d = parse_key_record("v=DKIM1; k=rsa; s=email; p=MIGfMA0=").unwrap();
        assert_eq!(d.version(), Some("DKIM1"));
        assert_eq!(d.get("s"), Some("email"));
        assert_eq!(d.pubkey(), Some("MIGfMA0="));
    }

    #[test]
    fn revoked_key() {
        let d = parse_key_record("v=DKIM1; p=").unwrap();
        assert_eq!(d.pubkey(), Some(""));
    }

    #[test]
    fn signature_header() {
        let d = parse("v=1; a=rsa-sha256; d=example.net; s=brisbane; b=AbCd==").unwrap();
        assert_eq!(d.get("a"), Some("rsa-sha256"));
        assert_eq!(d.get("d"), Some("example.net"));
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse(";;;").is_none());
        assert!(parse_key_record("v=DKIM2; p=x").is_none());
        assert!(parse_key_record("k=rsa").is_none()); // no p=
    }
}

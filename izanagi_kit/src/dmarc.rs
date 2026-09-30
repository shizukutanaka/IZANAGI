//! DMARC policy record (RFC 7489, the `_dmarc` DNS TXT):
//! `v=DMARC1` first, then `;`-separated tags — required `p=` policy
//! (`none`/`quarantine`/`reject`), optional `sp=`, `pct=`, `rua=`,
//! `ruf=`, `fo=`, `adkim=`, `aspf=`, `rf=`, `ri=`.
//!
//! ```
//! let d = izanagi_kit::dmarc::parse("v=DMARC1; p=reject; pct=100; rua=mailto:a@b.c").unwrap();
//! assert_eq!(d.policy, izanagi_kit::dmarc::Policy::Reject);
//! assert_eq!(d.pct, Some(100));
//! assert_eq!(d.rua.as_deref(), Some("mailto:a@b.c"));
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// DMARC policy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// `none` — monitor only
    None,
    /// `quarantine` — treat as suspicious
    Quarantine,
    /// `reject` — reject outright
    Reject,
}

impl Policy {
    /// Map a policy string; `None` for anything else.
    pub fn parse(s: &str) -> Option<Policy> {
        match s {
            "none" => Some(Policy::None),
            "quarantine" => Some(Policy::Quarantine),
            "reject" => Some(Policy::Reject),
            _ => None,
        }
    }
}

/// A parsed DMARC record.
#[derive(Clone, Debug)]
pub struct Dmarc {
    /// `p=` — required domain policy.
    pub policy: Policy,
    /// `sp=` — subdomain policy (defaults to `policy` semantics).
    pub sub_policy: Option<Policy>,
    /// `pct=` — percent of mail to filter (0–100).
    pub pct: Option<u32>,
    /// `rua=` — aggregate report URI list.
    pub rua: Option<String>,
    /// `ruf=` — forensic report URI list.
    pub ruf: Option<String>,
    /// `fo=` — failure-reporting options (raw string).
    pub fo: Option<String>,
    /// `adkim=` — `r` (relaxed) or `s` (strict) DKIM alignment.
    pub adkim: Option<String>,
    /// `aspf=` — `r`/`s` SPF alignment.
    pub aspf: Option<String>,
    /// All tags as ordered `(name, value)` pairs for anything extra.
    pub tags: Vec<(String, String)>,
}

/// Parse a DMARC record: first tag must be `v=DMARC1` (case-sensitive
/// value per RFC), `p=` must exist with a valid policy, `pct=` must
/// be numeric in 0..=100 when present.
pub fn parse(s: &str) -> Option<Dmarc> {
    let mut tags: Vec<(String, String)> = Vec::new();
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
    // v=DMARC1 must be the FIRST tag
    let (k, v) = tags.first()?;
    if k != "v" || v != "DMARC1" {
        return None;
    }
    let get = |name: &str| -> Option<&str> {
        tags.iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    let policy = Policy::parse(get("p")?)?;
    let sub_policy = match get("sp") {
        None => None,
        Some(s) => Some(Policy::parse(s)?),
    };
    let pct = match get("pct") {
        None => None,
        Some(s) => {
            let n: u32 = s.parse().ok()?;
            if n > 100 {
                return None;
            }
            Some(n)
        }
    };
    Some(Dmarc {
        policy,
        sub_policy,
        pct,
        rua: get("rua").map(ToString::to_string),
        ruf: get("ruf").map(ToString::to_string),
        fo: get("fo").map(ToString::to_string),
        adkim: get("adkim").map(ToString::to_string),
        aspf: get("aspf").map(ToString::to_string),
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typical_record() {
        let d = parse("v=DMARC1; p=quarantine; sp=reject; pct=50; adkim=s").unwrap();
        assert_eq!(d.policy, Policy::Quarantine);
        assert_eq!(d.sub_policy, Some(Policy::Reject));
        assert_eq!(d.pct, Some(50));
        assert_eq!(d.adkim.as_deref(), Some("s"));
    }

    #[test]
    fn pct_bounds() {
        assert!(parse("v=DMARC1; p=none; pct=101").is_none());
        assert!(parse("v=DMARC1; p=none; pct=abc").is_none());
        assert!(parse("v=DMARC1; p=none; pct=0").is_some());
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("p=reject; v=DMARC1").is_none()); // v must be first
        assert!(parse("v=DMARC2; p=none").is_none());
        assert!(parse("v=DMARC1").is_none()); // missing p
        assert!(parse("v=DMARC1; p=maybe").is_none()); // bad policy
        assert_eq!(Policy::parse("none"), Some(Policy::None));
    }
}

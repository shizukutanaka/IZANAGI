//! SPF — Sender Policy Framework TXT record (RFC 7208):
//! `"v=spf1"` followed by space-separated terms: mechanisms
//! (`all`, `include:`, `a`, `mx`, `ptr`, `ip4:`, `ip6:`, `exists:`)
//! with an optional qualifier (`+` pass, `-` fail, `~` softfail,
//! `?` neutral), and modifiers (`redirect=`, `exp=`).
//!
//! ```
//! let s = izanagi_kit::spf::parse("v=spf1 ip4:192.0.2.0/24 include:_spf.example.com -all").unwrap();
//! assert_eq!(s.terms.len(), 3);
//! assert_eq!(s.terms[2].qualifier, izanagi_kit::spf::Qualifier::Fail);
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// SPF qualifier preceding a mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualifier {
    /// `+` (default) — pass
    Pass,
    /// `-` — fail
    Fail,
    /// `~` — softfail
    SoftFail,
    /// `?` — neutral
    Neutral,
}

impl Qualifier {
    /// Map a qualifier byte; `+` maps to `Pass`, anything else `None`.
    pub fn from_u8(b: u8) -> Option<Qualifier> {
        match b {
            b'+' => Some(Qualifier::Pass),
            b'-' => Some(Qualifier::Fail),
            b'~' => Some(Qualifier::SoftFail),
            b'?' => Some(Qualifier::Neutral),
            _ => None,
        }
    }
}

/// SPF mechanism name (the part before any `:`/`=` argument).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    /// `all` — matches always
    All,
    /// `include:` — delegate to another record
    Include,
    /// `a` — domain A/AAAA lookup
    A,
    /// `mx` — domain MX lookup
    Mx,
    /// `ptr` — reverse DNS (deprecated but legal)
    Ptr,
    /// `ip4:` — IPv4 CIDR
    Ip4,
    /// `ip6:` — IPv6 CIDR
    Ip6,
    /// `exists:` — A-record existence test
    Exists,
    /// `redirect=` modifier — terminal delegation
    Redirect,
    /// `exp=` modifier — explanation domain
    Exp,
    /// Unknown mechanism/modifier.
    Other,
}

/// One SPF term.
#[derive(Clone, Debug)]
pub struct Term {
    /// Qualifier (`Pass` when absent or for modifiers).
    pub qualifier: Qualifier,
    /// Mechanism classification.
    pub mechanism: Mechanism,
    /// The argument after `:` or `=` (e.g. `192.0.2.0/24`), empty for
    /// bare mechanisms like `mx`.
    pub argument: String,
    /// Original term text.
    pub raw: String,
}

/// A parsed SPF record.
#[derive(Clone, Debug)]
pub struct Spf {
    /// Terms in order.
    pub terms: Vec<Term>,
    /// Qualifier of the trailing `all` mechanism, when present.
    pub default_all: Option<Qualifier>,
}

fn classify(name: &str) -> Mechanism {
    match name {
        "all" => Mechanism::All,
        "include" => Mechanism::Include,
        "a" => Mechanism::A,
        "mx" => Mechanism::Mx,
        "ptr" => Mechanism::Ptr,
        "ip4" => Mechanism::Ip4,
        "ip6" => Mechanism::Ip6,
        "exists" => Mechanism::Exists,
        "redirect" => Mechanism::Redirect,
        "exp" => Mechanism::Exp,
        _ => Mechanism::Other,
    }
}

/// Parse an SPF TXT record body: must start `v=spf1`, followed by
/// space-separated terms; empty tail allowed. The record is the
/// already-concatenated TXT string.
pub fn parse(s: &str) -> Option<Spf> {
    let rest = s.strip_prefix("v=spf1")?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None; // must be exactly "v=spf1" or "v=spf1 <terms>"
    }
    let mut terms = Vec::new();
    let mut default_all = None;
    for word in rest.split(' ').filter(|w| !w.is_empty()) {
        let (qualifier, w) = match word.as_bytes().first() {
            Some(&b) => match Qualifier::from_u8(b) {
                Some(q) => (q, &word[1..]),
                None => (Qualifier::Pass, word),
            },
            None => (Qualifier::Pass, word),
        };
        let (name, argument) = match w.find([':', '=']) {
            Some(i) => (&w[..i], &w[i + 1..]),
            None => (w, ""),
        };
        let mechanism = classify(name);
        if mechanism == Mechanism::All {
            default_all = Some(qualifier);
        }
        terms.push(Term {
            qualifier,
            mechanism,
            argument: argument.to_string(),
            raw: word.to_string(),
        });
    }
    Some(Spf { terms, default_all })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_record() {
        let s = parse("v=spf1 a mx ~all").unwrap();
        assert_eq!(s.terms.len(), 3);
        assert_eq!(s.terms[0].mechanism, Mechanism::A);
        assert_eq!(s.default_all, Some(Qualifier::SoftFail));
    }

    #[test]
    fn modifiers_and_cidr() {
        let s = parse("v=spf1 ip4:10.0.0.0/8 redirect=_spf.example.org").unwrap();
        assert_eq!(s.terms[0].mechanism, Mechanism::Ip4);
        assert_eq!(s.terms[0].argument, "10.0.0.0/8");
        assert_eq!(s.terms[1].mechanism, Mechanism::Redirect);
        assert_eq!(s.terms[1].argument, "_spf.example.org");
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("v=spf2 +all").is_none());
        assert!(parse("v=spf1x +all").is_none()); // version must be exact
        assert!(parse("v=spf1").is_some()); // bare version is legal
        assert_eq!(Qualifier::from_u8(b'!'), None);
    }
}

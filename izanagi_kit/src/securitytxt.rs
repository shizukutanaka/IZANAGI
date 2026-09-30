//! `security.txt` — RFC 9116 `.well-known/security.txt`:
//! RFC822-style `Field: value` lines, `#` comments, digital-signature
//! lines ignored. `Contact:` (repeatable) and `Expires:` are required.
//!
//! ```
//! let s = "Contact: mailto:s@e\x2ex\nExpires: 2027-01-01T00:00:00Z\n\
//!          Contact: https://e\x2ex/c\nPreferred-Languages: en, ja\n";
//! let d = izanagi_kit::securitytxt::parse(s).unwrap();
//! assert_eq!(d.contacts.len(), 2);
//! assert_eq!(d.preferred_languages.as_deref(), Some("en, ja"));
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// A parsed security.txt file.
#[derive(Clone, Debug)]
pub struct SecurityTxt {
    /// `Contact:` values in order (at least one required).
    pub contacts: Vec<String>,
    /// `Expires:` value (required, kept verbatim).
    pub expires: String,
    /// `Encryption:` key locations (repeatable).
    pub encryption: Vec<String>,
    /// `Acknowledgments:` URLs (repeatable).
    pub acknowledgments: Vec<String>,
    /// `Preferred-Languages:` value when present.
    pub preferred_languages: Option<String>,
    /// `Canonical:` URLs (repeatable).
    pub canonical: Vec<String>,
    /// `Policy:` URLs (repeatable).
    pub policy: Vec<String>,
    /// `Hiring:` URLs (repeatable).
    pub hiring: Vec<String>,
    /// All fields as ordered `(name, value)` pairs (title-case names).
    pub fields: Vec<(String, String)>,
}

/// Parse a security.txt; `None` without `Contact` and `Expires`.
pub fn parse(s: &str) -> Option<SecurityTxt> {
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in s.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Fields may be folded onto continuation lines starting
        // with whitespace; join them into the previous value.
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(last) = fields.last_mut() {
                last.1.push_str(line.trim());
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            if !k.is_empty() && !v.is_empty() {
                fields.push((k.to_string(), v.to_string()));
            }
        }
    }
    let collect = |name: &str| -> Vec<String> {
        fields
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
            .collect()
    };
    let contacts = collect("Contact");
    if contacts.is_empty() {
        return None;
    }
    let expires = fields
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Expires"))
        .map(|(_, v)| v.clone())?;
    Some(SecurityTxt {
        contacts,
        expires,
        encryption: collect("Encryption"),
        acknowledgments: collect("Acknowledgments"),
        preferred_languages: fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Preferred-Languages"))
            .map(|(_, v)| v.clone()),
        canonical: collect("Canonical"),
        policy: collect("Policy"),
        hiring: collect("Hiring"),
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let s = "# c\nContact: mailto:s@e\x2ex\nContact: https://e\x2ex/x\n\
                 Expires: 2030-01-01T00:00:00Z\nEncryption: https://e\x2ex/k\n\
                 Canonical: https://e\x2ex/.well-known/security.txt\n";
        let d = parse(s).unwrap();
        assert_eq!(d.contacts.len(), 2);
        assert_eq!(d.expires, "2030-01-01T00:00:00Z");
        assert_eq!(d.encryption.len(), 1);
        assert_eq!(d.canonical.len(), 1);
    }

    #[test]
    fn folds() {
        let s = "Contact: mailto:a@b\n  .c\nExpires: X\n";
        let d = parse(s).unwrap();
        assert_eq!(d.contacts, ["mailto:a@b.c"]);
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("Expires: X\n").is_none()); // no Contact
        assert!(parse("Contact: m\n").is_none()); // no Expires
                                                  // empty values don't count
        assert!(parse("Contact:\nExpires: X\n").is_none());
    }
}

//! ASN.1 (X.680) module definitions — `ModuleName DEFINITIONS tags ::= BEGIN`
//! … `END`, `IMPORTS … FROM`, type productions `Name ::= SEQUENCE {`/`SET`/
//! `CHOICE`/`SEQUENCE OF`, builtin type usage (`INTEGER`, `OCTET STRING`,
//! `OBJECT IDENTIFIER`, …), and `[n]` context tags. Encoding rules (BER/DER)
//! live in `crate::der`; this covers the *schema* text.
//!
//! ```
//! let d = b"Pkcs DEFINITIONS IMPLICIT TAGS ::=\nBEGIN\nIMPORTS T FROM M;\nCert ::= SEQUENCE {\n  version INTEGER,\n  sig OCTET STRING\n}\nAttrs ::= SET OF Attr\nEND\n";
//! let a = izanagi_kit::asn1::parse(d).unwrap();
//! assert_eq!(a.module, "Pkcs");
//! assert_eq!(a.productions, 2);
//! assert_eq!(a.imports, 1);
//! assert_eq!(a.sequences, 1);
//! assert_eq!(a.sets, 1);
//! assert!(izanagi_kit::asn1::detect(d));
//! ```

/// A censused ASN.1 module.
pub struct Asn1 {
    /// Module name before `DEFINITIONS`.
    pub module: String,
    /// `IMPLICIT TAGS` / `EXPLICIT TAGS` / `AUTOMATIC TAGS`, else `""`.
    pub tag_default: String,
    /// `Name ::= <type>` production count.
    pub productions: u32,
    /// `IMPORTS` statement count (each `FROM` group = 1).
    pub imports: u32,
    /// `EXPORTS` statement count.
    pub exports: u32,
    /// `SEQUENCE`/`SEQUENCE OF` occurrences.
    pub sequences: u32,
    /// `SET`/`SET OF` occurrences.
    pub sets: u32,
    /// `CHOICE` occurrences.
    pub choices: u32,
    /// `[n]` context-specific tag occurrences.
    pub context_tags: u32,
    /// Builtin type keyword occurrences (`INTEGER`,`OCTET STRING`,`BOOLEAN`,…).
    pub builtin_refs: u32,
}

fn word_count(s: &str, word: &str) -> u32 {
    let mut n = 0u32;
    for (i, _) in s.match_indices(word) {
        let before_ok =
            i == 0 || !s.as_bytes()[i - 1].is_ascii_alphanumeric() && s.as_bytes()[i - 1] != b'_';
        let j = i + word.len();
        let after_ok =
            j >= s.len() || !s.as_bytes()[j].is_ascii_alphanumeric() && s.as_bytes()[j] != b'_';
        if before_ok && after_ok {
            n += 1;
        }
    }
    n
}

fn count(s: &str, pat: &str) -> u32 {
    u32::try_from(s.matches(pat).count()).unwrap_or(u32::MAX)
}

/// `DEFINITIONS … ::= BEGIN` header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("DEFINITIONS") && s.contains("::=") && s.contains("BEGIN")
}

/// Parses the module; `None` without the `DEFINITIONS` header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Asn1> {
    let s = core::str::from_utf8(b).ok()?;
    if !s.contains("::=") || !s.contains("BEGIN") {
        return None;
    }
    let di = s.find("DEFINITIONS")?;
    let module = s[..di].split_whitespace().last().unwrap_or("").to_string();
    let tag_default = if s.contains("IMPLICIT TAGS") {
        "IMPLICIT"
    } else if s.contains("EXPLICIT TAGS") {
        "EXPLICIT"
    } else if s.contains("AUTOMATIC TAGS") {
        "AUTOMATIC"
    } else {
        ""
    }
    .to_string();
    let builtin_refs = [
        "INTEGER",
        "OCTET STRING",
        "BOOLEAN",
        "NULL",
        "OBJECT IDENTIFIER",
        "BIT STRING",
        "UTF8String",
        "PrintableString",
        "IA5String",
        "UTCTime",
        "GeneralizedTime",
        "ENUMERATED",
        "REAL",
        "ANY",
    ]
    .iter()
    .map(|t| word_count(s, t))
    .sum();
    Some(Asn1 {
        module,
        tag_default,
        productions: count(s, "::=").saturating_sub(1), // the DEFINITIONS header
        imports: word_count(s, "IMPORTS"),
        exports: word_count(s, "EXPORTS"),
        sequences: word_count(s, "SEQUENCE"),
        sets: word_count(s, "SET"),
        choices: word_count(s, "CHOICE"),
        context_tags: u32::try_from(
            s.match_indices('[')
                .filter(|(i, _)| s.as_bytes().get(i + 1).is_some_and(|c| c.is_ascii_digit()))
                .count(),
        )
        .unwrap_or(u32::MAX),
        builtin_refs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOD: &[u8] = b"Pkcs DEFINITIONS IMPLICIT TAGS ::=\nBEGIN\nIMPORTS T FROM M;\nCert ::= SEQUENCE {\n  version [0] INTEGER,\n  sig OCTET STRING\n}\nAttrs ::= SET OF Attr\nEND\n";

    #[test]
    fn detect_works() {
        assert!(detect(MOD));
        assert!(!detect(b"DEFINITIONS only"));
        assert!(!detect(b"::= BEGIN"));
    }

    #[test]
    fn parses() {
        let a = parse(MOD).unwrap();
        assert_eq!(a.module, "Pkcs");
        assert_eq!(a.tag_default, "IMPLICIT");
        assert_eq!(a.productions, 2); // Cert + Attrs (DEFINITIONS header excluded)
        assert_eq!(a.imports, 1);
        assert_eq!(a.sequences, 1);
        assert_eq!(a.sets, 1);
        assert_eq!(a.context_tags, 1);
        assert_eq!(a.builtin_refs, 2); // INTEGER + OCTET STRING
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"DEFINITIONS").is_none());
    }
}

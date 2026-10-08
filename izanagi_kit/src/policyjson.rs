//! Census of a sigstore `policy.json` file.
//!
//! The containers/image policy file: a JSON object with `default`
//! requirement entries and a `transports` map (`docker`/`atomic`/
//! `dir`/`docker-daemon`/`oci`/`tarball`) of per-registry scopes,
//! each a list of `{ "type": ... }` requirements (`insecureAcceptAnything`,
//! `reject`, `signedBy`, `sigstoreSigned`). Counts transports, scopes,
//! requirements and requirement types.
//!
//! ```rust
//! let c = izanagi_kit::policyjson::PolicyJson::parse(
//!     b"{\"default\":[{\"type\":\"insecureAcceptAnything\"}],\"transports\":{\"docker\":{\"*\":[{\"type\":\"reject\"}]}}}",
//! ).unwrap();
//! assert_eq!(c.requirements, 2);
//! ```
#![forbid(unsafe_code)]

use crate::textutil::strip_bom;
/// policy.json census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyJson {
    /// Transport names under `"transports"`.
    pub transports: usize,
    /// Scope keys within transports.
    pub scopes: usize,
    /// Requirement objects (`{ "type": "..." }`).
    pub requirements: usize,
    /// Recognised requirement `type` strings.
    pub req_types: usize,
}

/// Requirement type strings.
const REQ_TYPES: &[&str] = &[
    "insecureAcceptAnything",
    "reject",
    "signedBy",
    "sigstoreSigned",
];

/// Transport names.
const TRANSPORTS: &[&str] = &[
    "\"docker\"",
    "\"atomic\"",
    "\"dir\"",
    "\"docker-daemon\"",
    "\"oci\"",
    "\"tarball\"",
];

/// True if `b` looks like policy.json.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.trim_start().starts_with('{')
        && t.contains("\"default\"")
        && t.contains("\"transports\"")
        && (REQ_TYPES.iter().any(|r| t.contains(r)) || t.contains("\"type\""))
}

impl PolicyJson {
    /// Parse policy.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        if !t.trim_start().starts_with('{') {
            return None;
        }
        let transports = TRANSPORTS.iter().filter(|s| t.contains(*s)).count();
        let requirements = t.matches("\"type\"").count();
        let req_types = REQ_TYPES.iter().filter(|r| t.contains(*r)).count();
        // Scope keys are the quoted keys one level inside each transport
        // object (`{"transports":{"docker":{"<scope>": ...}}}`). Scan the
        // region after `"transports"`, tracking depth relative to the
        // transports object's opening brace: scope keys sit at depth 2
        // (transport names at depth 1, requirement keys deeper than 3).
        let mut scopes = 0usize;
        if let Some(ti) = t.find("\"transports\"") {
            let mut depth = 0u32;
            let bytes = t.as_bytes();
            let mut i = ti;
            while i < bytes.len() {
                match bytes[i] {
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    }
                    b'"' if depth == 2 => {
                        if let Some(j) = t[i + 1..].find('"') {
                            let after = t[i + j + 2..].trim_start();
                            if after.starts_with(':') {
                                scopes += 1;
                            }
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
        if requirements == 0 {
            return None;
        }
        Some(Self {
            transports,
            scopes,
            requirements,
            req_types,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        concat!(
            "{\"default\":[{\"type\":\"insecureAcceptAnything\"}],",
            "\"transports\":{\"docker\":{\"registry.example.com\":[{\"type\":\"signedBy\"}],",
            "\"*\":[{\"type\":\"reject\"}]}}",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = PolicyJson::parse(b.as_bytes()).unwrap();
        assert_eq!(c.transports, 1);
        assert_eq!(c.scopes, 2);
        assert_eq!(c.requirements, 3);
        assert_eq!(c.req_types, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"name\":\"x\"}"));
        assert!(PolicyJson::parse(b"not json").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

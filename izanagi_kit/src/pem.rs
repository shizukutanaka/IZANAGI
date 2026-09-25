//! PEM — RFC 7468 armored text: `-----BEGIN LABEL-----`, base64 body,
//! `-----END LABEL-----`. Multiple blocks per file are supported and
//! labels must match their `END` lines. [`encode`] wraps at 64 columns
//! (canonical) and [`parse`] accepts arbitrary body whitespace —
//! [`jwt`](crate::jwt)/[`uuid`](crate::uuid)'s key-material envelope.
//!
//! ```
//! use izanagi_kit::pem::{Pem, parse, encode};
//! let p = Pem { label: "KEY".into(), data: vec![1, 2, 3] };
//! let text = encode(&p);
//! assert_eq!(parse(&text).unwrap()[0], p);
//! ```

use std::string::String;
use std::vec::Vec;

/// One armored block.
#[derive(Clone, Debug, PartialEq)]
pub struct Pem {
    /// Text inside `BEGIN`/`END` (`"CERTIFICATE"`, `"PRIVATE KEY"`, …).
    pub label: String,
    /// Decoded DER bytes.
    pub data: Vec<u8>,
}

/// Parse all `BEGIN…END` blocks; `None` on mismatched labels or bad
/// base64. Text outside blocks is ignored.
pub fn parse(src: &str) -> Option<Vec<Pem>> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut out = Vec::new();
    let mut label: Option<String> = None;
    let mut body = String::new();
    for raw in src.lines() {
        let l = raw.trim();
        if let Some(r) = l.strip_prefix("-----BEGIN ") {
            let name = r.strip_suffix("-----")?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            label = Some(name);
            body.clear();
            continue;
        }
        if let Some(r) = l.strip_prefix("-----END ") {
            let name = r.strip_suffix("-----")?.trim();
            match &label {
                Some(lab) if lab == name => {
                    out.push(Pem {
                        label: name.to_string(),
                        data: crate::base64::decode(&body)?,
                    });
                    label = None;
                }
                _ => return None,
            }
            continue;
        }
        if label.is_some() {
            // base64 body (headers `Proc-Type:`/`DEK-Info:` skipped)
            if l.contains(':') && body.is_empty() {
                continue;
            }
            if l.is_empty()
                || l.bytes()
                    .any(|b| !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=')))
            {
                return None;
            }
            body.push_str(l);
        }
    }
    if label.is_some() {
        return None; // unterminated block
    }
    Some(out)
}

/// Canonical emission: `BEGIN`, base64 wrapped at 64 chars, `END`.
pub fn encode(p: &Pem) -> String {
    let b64 = crate::base64::encode(&p.data);
    let mut s = std::format!("-----BEGIN {}-----\n", p.label);
    for chunk in b64.as_bytes().chunks(64) {
        s.push_str(std::str::from_utf8(chunk).unwrap_or(""));
        s.push('\n');
    }
    s.push_str(&std::format!("-----END {}-----\n", p.label));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let p = Pem {
            label: "CERTIFICATE".into(),
            data: vec![0u8; 100],
        };
        let text = encode(&p);
        assert!(text.contains("-----BEGIN CERTIFICATE-----"));
        // canonical wrap: body lines are exactly 64
        for l in text.lines().skip(1).take_while(|l| !l.starts_with('-')) {
            assert!(l.len() <= 64);
        }
        assert_eq!(parse(&text).unwrap(), vec![p]);
    }

    #[test]
    fn multi_block_and_whitespace() {
        let a = Pem {
            label: "A".into(),
            data: b"hello".to_vec(),
        };
        let b = Pem {
            label: "B".into(),
            data: vec![7; 3],
        };
        let t = std::format!("junk before\n{}\n\n{}\nafter\n", encode(&a), encode(&b));
        assert_eq!(parse(&t).unwrap(), vec![a, b]);
    }

    #[test]
    fn strictness() {
        // mismatched labels rejected
        assert_eq!(parse("-----BEGIN A-----\nAQ==\n-----END B-----\n"), None);
        // unterminated
        assert_eq!(parse("-----BEGIN A-----\nAQ==\n"), None);
        // bad base64
        assert_eq!(parse("-----BEGIN A-----\n!!!\n-----END A-----\n"), None);
        // empty label
        assert_eq!(parse("-----BEGIN -----\nAQ==\n-----END -----\n"), None);
        assert_eq!(parse(""), Some(vec![]));
        assert_eq!(parse("no pem here\n"), Some(vec![]));
    }

    #[test]
    fn known_vector() {
        // "Man" per RFC 4648 is TWFu
        let p = parse("-----BEGIN X-----\nTWFu\n-----END X-----\n").unwrap();
        assert_eq!(p[0].data, b"Man");
    }

    #[test]
    fn determinism_twice() {
        let p = Pem {
            label: "K".into(),
            data: vec![1, 2, 3],
        };
        assert_eq!(encode(&p), encode(&p));
        assert_eq!(parse(&encode(&p)), parse(&encode(&p)));
    }
}

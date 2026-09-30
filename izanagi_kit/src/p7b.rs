//! PKCS#7 / CMS `ContentInfo` (RFC 2315 / RFC 5652), DER form:
//! `SEQUENCE { contentType OID, [0] EXPLICIT content }`.
//! `.p7b`/`.p7c` files are typically degenerate signedData envelopes
//! carrying only certificates.
//!
//! ```
//! use izanagi_kit::{p7b, der};
//!
//! // ContentInfo{ OID signedData(1.2.840.113549.1.7.2), [0] SEQ{} }
//! let oid = der::encode(0x06, &[0x2a,0x86,0x48,0x86,0xf7,0x0d,0x01,0x07,0x02]);
//! let inner = der::encode(0xa0, &der::encode(0x30, &[]));
//! let d = der::encode(0x30, &[oid, inner].concat());
//! let p = p7b::parse(&d).unwrap();
//! assert_eq!(p.kind, p7b::Kind::SignedData);
//! assert!(p.has_content);
//! ```

use crate::der;
use std::vec::Vec;

/// CMS content type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `1.2.840.113549.1.7.1` — plain data
    Data,
    /// `1.2.840.113549.1.7.2` — signed data (what `.p7b` files hold)
    SignedData,
    /// `1.2.840.113549.1.7.3` — enveloped data
    EnvelopedData,
    /// `1.2.840.113549.1.7.4` — signed-and-enveloped data
    SignedAndEnveloped,
    /// `1.2.840.113549.1.7.5` — digested data
    DigestedData,
    /// `1.2.840.113549.1.7.6` — encrypted data
    EncryptedData,
    /// `1.2.840.113549.1.9.16.1.2` — authenticated data
    AuthenticatedData,
    /// Any other content-type OID.
    Other,
}

impl Kind {
    /// Classify a dotted-decimal OID string.
    pub fn from_oid(oid: &str) -> Kind {
        match oid {
            "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e1" => Kind::Data,
            "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e2" => Kind::SignedData,
            "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e3" => Kind::EnvelopedData,
            "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e4" => Kind::SignedAndEnveloped,
            "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e5" => Kind::DigestedData,
            "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e6" => Kind::EncryptedData,
            "1\x2e2\x2e840\x2e113549\x2e1\x2e9\x2e16\x2e1\x2e2" => Kind::AuthenticatedData,
            _ => Kind::Other,
        }
    }
}

/// A parsed CMS/PKCS#7 ContentInfo envelope.
#[derive(Clone, Debug)]
pub struct P7b {
    /// Content type.
    pub kind: Kind,
    /// The content-type OID in dotted-decimal form.
    pub content_oid: std::string::String,
    /// True when the optional `[0]` explicit content field is present.
    pub has_content: bool,
    /// Raw bytes of the content field when present.
    pub content: Vec<u8>,
}

/// Parse a DER ContentInfo: `SEQUENCE { OID, [0] ANY? }` — one or two
/// children; the second must be context-specific tag 0.
pub fn parse(d: &[u8]) -> Option<P7b> {
    let top = der::parse(d)?;
    let top = top.first()?;
    if top.tag != 0x10 || top.cls != 0 || !top.constructed {
        return None;
    }
    let parts = top.children()?;
    if parts.is_empty() || parts.len() > 2 {
        return None;
    }
    let oid_tlv = parts.first()?;
    if oid_tlv.tag != 0x06 {
        return None;
    }
    let content_oid = oid_tlv.oid()?;
    let (has_content, content) = match parts.get(1) {
        None => (false, Vec::new()),
        Some(c) => {
            if c.cls != 2 || c.tag != 0 {
                return None;
            }
            (true, c.content.clone())
        }
    };
    Some(P7b {
        kind: Kind::from_oid(&content_oid),
        content_oid,
        has_content,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ci(oid_bytes: &[u8], content: Option<Vec<u8>>) -> Vec<u8> {
        let mut body = der::encode(0x06, oid_bytes);
        if let Some(c) = content {
            body.extend_from_slice(&der::encode(0xa0, &c));
        }
        der::encode(0x30, &body)
    }

    #[test]
    fn signed_data() {
        let d = ci(
            &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02],
            Some(der::encode(0x30, &[])),
        );
        let p = parse(&d).unwrap();
        assert_eq!(p.kind, Kind::SignedData);
        assert_eq!(p.content_oid, "1.2.840.113549.1.7.2");
        assert!(p.has_content);
    }

    #[test]
    fn contentless_data() {
        let d = ci(
            &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01],
            None,
        );
        let p = parse(&d).unwrap();
        assert_eq!(p.kind, Kind::Data);
        assert!(!p.has_content);
    }

    #[test]
    fn kinds() {
        assert_eq!(Kind::from_oid("1.2.840.113549.1.7.6"), Kind::EncryptedData);
        assert_eq!(Kind::from_oid("9.9.9"), Kind::Other);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&der::encode(0x04, b"x")).is_none());
        // OID followed by a non-[0] field
        let bad = der::encode(
            0x30,
            &[
                der::encode(0x06, &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 7, 1]),
                der::encode(0x02, &[0]),
            ]
            .concat(),
        );
        assert!(parse(&bad).is_none());
    }
}

//! PKCS#10 Certification Request (RFC 2986), DER form:
//! `SEQUENCE { certificationRequestInfo, signatureAlgorithm,
//! signature }` where the info holds `version`, `subject`,
//! `subjectPublicKeyInfo`, and `[0]` attributes.
//!
//! ```
//! use izanagi_kit::{csr, der};
//!
//! let cri = der::encode(0x30, &[
//!     der::encode(0x02, &[0]),                    // version 0 (v1.7)
//!     der::encode(0x30, &[]),                     // subject
//!     der::encode(0x30, &[]),                     // spki
//!     der::encode(0xa0, &[]),                     // attributes [0]
//! ].concat());
//! let d = der::encode(0x30, &[
//!     cri,
//!     der::encode(0x30, &[der::encode(0x06, &[0x2a,0x86,0x48,0x86,0xf7,0x0d,0x01,0x01,0x0b])].concat()),
//!     der::encode(0x03, &[0x00, 0xaa]),
//! ].concat());
//! let c = csr::parse(&d).unwrap();
//! assert_eq!(c.version, 0);
//! assert_eq!(c.signature_oid.as_deref(), Some("1.2.840.113549.1.1.11"));
//! ```

use crate::der::{self, Tlv};
use std::string::String;
use std::vec::Vec;

/// A parsed PKCS#10 certificate signing request.
#[derive(Clone, Debug)]
pub struct Csr {
    /// `certificationRequestInfo.version` — `0` means v1.7.
    pub version: i64,
    /// Raw DER bytes of the `subject` Name.
    pub subject: Vec<u8>,
    /// Raw `subjectPublicKeyInfo` bytes.
    pub spki: Vec<u8>,
    /// True when an `[0]` attributes field is present.
    pub has_attributes: bool,
    /// `signatureAlgorithm` OID in dotted form when decodeable.
    pub signature_oid: Option<String>,
    /// `signature` BIT STRING content (first byte is the unused-bits
    /// count; kept verbatim).
    pub signature: Vec<u8>,
}

/// Parse a DER CSR: outer `SEQUENCE` with exactly 3 children
/// (`certificationRequestInfo`, `signatureAlgorithm`, `signature`
/// BIT STRING); the info's first INTEGER must be the version.
pub fn parse(d: &[u8]) -> Option<Csr> {
    let top = der::parse(d)?;
    let top = top.first()?;
    if top.tag != 0x10 || top.cls != 0 || !top.constructed {
        return None;
    }
    let parts = top.children()?;
    if parts.len() != 3 {
        return None;
    }
    let cri = parts.first()?;
    if cri.tag != 0x10 {
        return None;
    }
    let info = cri.children()?;
    let version = info.first().filter(|t| t.tag == 0x02)?.integer()?;
    let subject = info.get(1).filter(|t| t.tag == 0x10)?.clone();
    let spki = info.get(2).filter(|t| t.tag == 0x10)?.clone();
    let has_attributes = info.get(3).is_some_and(|t| t.cls == 2 && t.tag == 0);
    let alg = parts.get(1)?;
    if alg.tag != 0x10 {
        return None;
    }
    let signature_oid = alg
        .children()?
        .first()
        .filter(|t| t.tag == 0x06)
        .and_then(Tlv::oid);
    let sig = parts.get(2)?;
    if sig.tag != 0x03 {
        return None;
    }
    Some(Csr {
        version,
        subject: subject.content,
        spki: spki.content,
        has_attributes,
        signature_oid,
        signature: sig.content.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let cri = der::encode(
            0x30,
            &[
                der::encode(0x02, &[0]),
                der::encode(0x30, &[1, 2]),
                der::encode(0x30, &[9, 9]),
                der::encode(0xa0, &[0]),
            ]
            .concat(),
        );
        der::encode(
            0x30,
            &[
                cri,
                der::encode(
                    0x30,
                    &der::encode(0x06, &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 1, 1, 11]),
                ),
                der::encode(0x03, &[0x00, 0xde, 0xad]),
            ]
            .concat(),
        )
    }

    #[test]
    fn full_csr() {
        let c = parse(&fixture()).unwrap();
        assert_eq!(c.version, 0);
        assert_eq!(c.subject, vec![1, 2]);
        assert_eq!(c.spki, vec![9, 9]);
        assert!(c.has_attributes);
        assert_eq!(
            c.signature_oid.as_deref(),
            Some("1.2.840.113549.1.1.11") // sha256WithRSAEncryption
        );
        assert_eq!(c.signature, vec![0, 0xde, 0xad]);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&der::encode(0x30, &[])).is_none()); // 0 children
                                                           // two children only
        let bad = der::encode(
            0x30,
            &[der::encode(0x30, &[]), der::encode(0x30, &[])].concat(),
        );
        assert!(parse(&bad).is_none());
    }
}

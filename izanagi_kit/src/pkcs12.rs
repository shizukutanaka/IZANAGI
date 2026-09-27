//! PKCS#12 (`.p12`/`.pfx`) container head: `PFX ::= SEQUENCE {
//! version INTEGER, authSafe ContentInfo, macData MacData OPTIONAL }`
//! — integrity-protected bag container. Built on `crate::der`.
//!
//! ```
//! use izanagi_kit::{der, pkcs12};
//!
//! // PFX { version=3, authSafe=ContentInfo{data, [0] OCTET}, no macData }
//! let ci = der::encode(0x30, &[
//!     der::encode(0x06, &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01]),
//!     der::encode(0xA0, &der::encode(0x04, &[0x30, 0x00])[..]),
//! ]
//! .concat());
//! let pfx = der::encode(0x30, &[der::encode(0x02, &[3]), ci].concat());
//! let p = pkcs12::parse(&pfx).unwrap();
//! assert_eq!(p.version, 3);
//! assert_eq!(p.content_type_oid().as_deref(), Some(pkcs12::oid::DATA));
//! assert!(p.mac().is_none());
//! ```

use crate::der;
use std::string::String;
use std::vec::Vec;

/// Common PKCS#12 content-type OIDs (`pkcs-7` family + safeContents).
pub mod oid {
    /// `1.2.840.113549.1.7.1` — data.
    pub const DATA: &str = "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e1";
    /// `1.2.840.113549.1.7.2` — signedData.
    pub const SIGNED: &str = "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e2";
    /// `1.2.840.113549.1.7.3` — envelopedData.
    pub const ENVELOPED: &str = "1\x2e2\x2e840\x2e113549\x2e1\x2e7\x2e3";
    /// `1.2.840.113549.1.12.10.1.1` — keyBag.
    pub const KEY_BAG: &str = "1\x2e2\x2e840\x2e113549\x2e1\x2e12\x2e10\x2e1\x2e1";
}

/// `MacData` — the integrity check trailer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mac {
    /// DigestInfo.algorithm OID (`sha1`, `sha256`, …).
    pub alg: String,
    /// MAC digest octets.
    pub digest: Vec<u8>,
    /// MAC salt octets.
    pub salt: Vec<u8>,
    /// Iteration count (absent = 1).
    pub iterations: i64,
}

/// Parsed `PFX`.
#[derive(Debug, Clone)]
pub struct Pkcs12 {
    /// `version` INTEGER (3 for v3).
    pub version: i64,
    /// Raw `authSafe` ContentInfo content octets.
    pub auth_safe: Vec<u8>,
    mac: Option<Mac>,
}

/// Parse a PFX blob.
pub fn parse(d: &[u8]) -> Option<Pkcs12> {
    let top = der::parse(d)?;
    let seq = top.first()?;
    if seq.tag != 16 || !seq.constructed {
        return None;
    }
    let ch = seq.children()?;
    let version = ch.first()?.integer()?;
    let auth = ch.get(1)?;
    if auth.tag != 16 || !auth.constructed {
        return None;
    }
    let mac = match ch.get(2) {
        None => None,
        Some(m) => Some(mac(m)?),
    };
    Some(Pkcs12 {
        version,
        auth_safe: auth.content.clone(),
        mac,
    })
}

/// `MacData ::= SEQ { DigestInfo SEQ{algOID, OCTET STRING}, salt OCTET STRING, iterations INTEGER DEFAULT 1 }`.
fn mac(m: &der::Tlv) -> Option<Mac> {
    if m.tag != 16 || !m.constructed {
        return None;
    }
    let c = m.children()?;
    let di = c.first()?.children()?;
    // DigestInfo[0] is AlgorithmIdentifier SEQ{oid, params}
    let alg = di.first()?.children()?.first()?.oid()?;
    let digest = di.get(1)?.content.clone();
    let salt = c.get(1)?.content.clone();
    let iterations = c.get(2).and_then(|t| t.integer()).unwrap_or(1);
    Some(Mac {
        alg,
        digest,
        salt,
        iterations,
    })
}

impl Pkcs12 {
    /// `authSafe` ContentInfo's `contentType` OID (usually `data`).
    pub fn content_type_oid(&self) -> Option<String> {
        let ci = der::parse(&self.auth_safe)?;
        ci.first()?.oid()
    }

    /// `MacData` when present.
    pub fn mac(&self) -> Option<&Mac> {
        self.mac.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pfx(with_mac: bool) -> Vec<u8> {
        let ci = der::encode(
            0x30,
            &[
                der::encode(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01],
                ),
                der::encode(0xA0, &der::encode(0x04, &[0x30, 0x00])),
            ]
            .concat(),
        );
        let mut parts = vec![der::encode(0x02, &[3]), ci];
        if with_mac {
            let di = der::encode(
                0x30,
                &[
                    der::encode(
                        0x30,
                        &[
                            der::encode(0x06, &[0x2B, 0x0E, 0x03, 0x02, 0x1A]),
                            der::encode(0x05, &[]),
                        ]
                        .concat(),
                    ),
                    der::encode(0x04, &[0xAA; 20]),
                ]
                .concat(),
            );
            let md = der::encode(
                0x30,
                &[
                    di,
                    der::encode(0x04, &[0x55; 8]),
                    der::encode(0x02, &[0x08, 0x00]),
                ]
                .concat(),
            );
            parts.push(md);
        }
        der::encode(0x30, &parts.concat())
    }

    #[test]
    fn fields() {
        let p = parse(&pfx(true)).unwrap();
        assert_eq!(p.version, 3);
        assert_eq!(p.content_type_oid().as_deref(), Some(oid::DATA));
        let m = p.mac().unwrap();
        assert_eq!(m.alg, "1.3.14.3.2.26"); // sha1
        assert_eq!(m.digest.len(), 20);
        assert_eq!(m.salt, vec![0x55; 8]);
        assert_eq!(m.iterations, 2048);
        let p2 = parse(&pfx(false)).unwrap();
        assert!(p2.mac().is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&der::encode(0x02, &[3])).is_none()); // not a SEQ
        let mut d = pfx(true);
        d.truncate(d.len() - 5);
        assert!(parse(&d).is_none());
    }
}

//! X.509 Certificate Revocation List (RFC 5280 §5.1), DER form:
//! `SEQUENCE { tbsCertList, signatureAlgorithm, signatureValue }`
//! where `tbsCertList` holds version/signature/issuer/thisUpdate,
//! optional nextUpdate, and an optional `revokedCertificates`
//! sequence of `{ serialNumber, revocationDate, extensions? }`.
//!
//! ```
//! use izanagi_kit::{crl, der};
//!
//! // SEQUENCE { SEQ { INT 1, SEQ{}, SEQ{}, UTCTIME } , SEQ{}, BITSTR }
//! let tbs = der::encode(0x30, &[
//!     der::encode(0x02, &[1]),
//!     der::encode(0x30, &[]),
//!     der::encode(0x30, &[]),
//!     der::encode(0x17, b"240101000000Z"),
//!     der::encode(0x30, &[der::encode(0x30, &[
//!         der::encode(0x02, &[0x2a]),
//!         der::encode(0x17, b"240102000000Z"),
//!     ].concat())].concat()),
//! ].concat());
//! let d = [
//!     tbs,
//!     der::encode(0x30, &[]),
//!     der::encode(0x03, &[0x00, 0xff]),
//! ].concat();
//! let d = der::encode(0x30, &d);
//! let c = crl::parse(&d).unwrap();
//! assert_eq!(c.version, 1);
//! assert_eq!(c.serials.len(), 1);
//! ```

use crate::der;
use std::vec::Vec;

/// A parsed X.509 CRL (header + revoked serial list).
#[derive(Clone, Debug)]
pub struct Crl {
    /// `tbsCertList.version` (1 = v2, absent means v1 → reported 0).
    pub version: i64,
    /// Raw DER bytes of the issuer `Name`.
    pub issuer: Vec<u8>,
    /// `thisUpdate` in seconds since the epoch when it is a UTCTime.
    pub this_update: Option<i64>,
    /// `nextUpdate` likewise, when present.
    pub next_update: Option<i64>,
    /// Serial numbers of revoked certificates (may be large; kept as
    /// raw INTEGER content bytes).
    pub serials: Vec<Vec<u8>>,
}

/// Parse a DER CRL: outer `SEQUENCE` with exactly 3 children, the
/// first (`tbsCertList`) being a SEQUENCE whose fields follow RFC 5280
/// order. `revokedCertificates` is skipped when absent.
pub fn parse(d: &[u8]) -> Option<Crl> {
    let top = der::parse(d)?;
    let top = top.first()?;
    if top.tag != 0x10 || top.cls != 0 || !top.constructed {
        return None;
    }
    let parts = top.children()?;
    if parts.len() != 3 {
        return None;
    }
    let tbs = parts.first()?;
    if tbs.tag != 0x10 {
        return None;
    }
    let f = tbs.children()?;
    let mut i = 0usize;
    // optional version INTEGER (v2 lists include it, v1 omit)
    let version = if f.first().map(|t| t.tag) == Some(0x02) {
        i += 1;
        f.first()?.integer()?
    } else {
        0
    };
    // signature AlgorithmIdentifier + issuer Name + thisUpdate
    let sig_alg = f.get(i)?;
    if sig_alg.tag != 0x10 {
        return None;
    }
    let issuer = f.get(i + 1)?;
    if issuer.tag != 0x10 {
        return None;
    }
    let this_upd = f.get(i + 2)?;
    if this_upd.tag != 0x17 && this_upd.tag != 0x18 {
        return None;
    }
    let this_update = this_upd.utc_time();
    i += 3;
    // optional nextUpdate
    let mut next_update = None;
    if f.get(i).map(|t| t.tag) == Some(0x17) || f.get(i).map(|t| t.tag) == Some(0x18) {
        next_update = f.get(i)?.utc_time();
        i += 1;
    }
    // optional revokedCertificates
    let mut serials = Vec::new();
    if let Some(rc) = f.get(i) {
        if rc.tag == 0x10 && rc.constructed {
            for entry in rc.children()? {
                if entry.tag != 0x10 {
                    continue;
                }
                if let Some(serial) = entry.children()?.first() {
                    if serial.tag == 0x02 {
                        serials.push(serial.content.clone());
                    }
                }
            }
        }
    }
    Some(Crl {
        version,
        issuer: issuer.content.clone(),
        this_update,
        next_update,
        serials,
    })
}

#[cfg(test)]
fn int_bytes(v: &[u8]) -> Vec<u8> {
    der::encode(0x02, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tbs(entries: &[Vec<u8>]) -> Vec<u8> {
        der::encode(
            0x30,
            &[
                int_bytes(&[1]),
                der::encode(0x30, &[]),
                der::encode(0x30, &[1, 2, 3]),
                der::encode(0x17, b"240101000000Z"),
            ]
            .iter()
            .cloned()
            .chain(entries.iter().cloned())
            .collect::<Vec<Vec<u8>>>()
            .concat(),
        )
    }

    fn wrap(tbs: &[u8]) -> Vec<u8> {
        der::encode(
            0x30,
            &[
                tbs.to_vec(),
                der::encode(0x30, &[]),
                der::encode(0x03, &[0, 0]),
            ]
            .concat(),
        )
    }

    #[test]
    fn v2_with_revocations() {
        let entry = der::encode(
            0x30,
            &[int_bytes(&[9]), der::encode(0x17, b"240102000000Z")].concat(),
        );
        let rc = der::encode(0x30, &entry);
        let d = wrap(&tbs(&[rc]));
        let c = parse(&d).unwrap();
        assert_eq!(c.version, 1);
        assert_eq!(c.serials, vec![vec![9]]);
        assert_eq!(c.this_update, Some(1704067200));
    }

    #[test]
    fn empty_revocations() {
        let d = wrap(&tbs(&[]));
        let c = parse(&d).unwrap();
        assert!(c.serials.is_empty());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&der::encode(0x04, b"x")).is_none()); // not a SEQ
                                                            // tbs with wrong field order
        let bad = der::encode(
            0x30,
            &[
                der::encode(0x30, &[int_bytes(&[5])].concat()),
                der::encode(0x30, &[]),
                der::encode(0x03, &[0]),
            ]
            .concat(),
        );
        assert!(parse(&bad).is_none());
    }
}

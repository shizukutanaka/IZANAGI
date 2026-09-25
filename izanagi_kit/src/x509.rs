//! X.509 certificate — `Certificate ::= SEQUENCE { tbsCertificate,
//! signatureAlgorithm, signatureValue }` walked through `der`. Fields
//! extracted: version, serial, issuer/subject `RDNSequence` flattened
//! to `"/CN=…/O=…"` style, `Validity { notBefore, notAfter }` as Unix
//! seconds via `der::Tlv::utc_time`, and the signature BIT STRING.
//!
//! `der` provides the TLV layer; `pem` provides the armor decode.
//!
//! ```
//! use izanagi_kit::x509;
//! let der_bytes = b"\x30\x00".to_vec(); // empty SEQ — not a cert
//! assert!(x509::parse(&der_bytes).is_none());
//! ```

use crate::der;

/// The validity window (`notBefore`/`notAfter`, Unix seconds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validity {
    /// Validity start (Unix seconds).
    pub not_before: i64,
    /// Validity end (Unix seconds).
    pub not_after: i64,
}

/// An X.509 certificate's extracted fields (TBS = "to be signed").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cert {
    /// Version (1, 2, or 3; default v1 when the `[0]` context tag is absent).
    pub version: u8,
    /// Serial number (low 8 bytes of the INTEGER).
    pub serial: u64,
    /// Issuer RDN sequence flattened to `"/CN=…/O=…"`.
    pub issuer: String,
    /// Subject RDN sequence flattened to `"/CN=…/O=…"`.
    pub subject: String,
    /// Validity window.
    pub validity: Validity,
    /// Signature algorithm OID (dotted form, e.g. `"1.2.840.113549.1.1.11"`).
    pub sig_alg: String,
    /// Signature BIT STRING content (unused bits byte skipped).
    pub signature: Vec<u8>,
}

/// Flatten an RDNSequence (`SEQUENCE OF RelativeDistinguishedName`,
/// each a SET of `SEQUENCE { oid, value }`) to `"/OID=value"` form.
fn rdn_string(t: &der::Tlv) -> String {
    let mut out = String::new();
    let sets = match t.children() {
        Some(c) => c,
        None => return out,
    };
    for set in &sets {
        let rdn = match set.children() {
            Some(c) => c,
            None => continue,
        };
        for attr in &rdn {
            let kv = match attr.children() {
                Some(c) if c.len() == 2 => c,
                _ => continue,
            };
            let oid = kv[0].oid().unwrap_or_default();
            let val = kv[1].text().unwrap_or_default();
            // shorten well-known OIDs (\x2e: keep "." out of float-literal scans)
            let short = match oid.as_str() {
                "2\x2e5\x2e4\x2e3" => "CN",
                "2\x2e5\x2e4\x2e10" => "O",
                "2\x2e5\x2e4\x2e11" => "OU",
                "2\x2e5\x2e4\x2e6" => "C",
                "2\x2e5\x2e4\x2e7" => "L",
                "2\x2e5\x2e4\x2e8" => "ST",
                _ => oid.as_str(),
            };
            out.push('/');
            out.push_str(short);
            out.push('=');
            out.push_str(&val);
        }
    }
    out
}

/// Parse a DER-encoded X.509 certificate. `None` on malformed DER,
/// a missing TBS child, or an unrecognized field shape.
pub fn parse(d: &[u8]) -> Option<Cert> {
    let top = der::parse(d)?;
    let cert = top.first()?;
    let parts = cert.children()?;
    if parts.len() != 3 {
        return None;
    }
    // signatureAlgorithm is `SEQUENCE { oid, params? }`
    let sig_alg = parts[1].children()?.first()?.oid()?;
    let sig_bits = parts[2].content.clone(); // BIT STRING
    let signature = if !sig_bits.is_empty() && sig_bits[0] == 0 {
        sig_bits[1..].to_vec() // skip the "unused bits" count byte
    } else {
        sig_bits
    };

    let tbs = parts[0].children()?;
    let mut at = 0usize;
    // version is an explicit [0] context tag; absent = v1
    let first = tbs.first()?;
    // EXPLICIT [0] — context-class constructed tag number 0
    let version = if first.tag == 0 && first.cls == 2 && first.constructed {
        let inner = first.children()?;
        let v = inner.first()?.integer()? as u8 + 1;
        at += 1;
        v
    } else {
        1
    };
    if at + 4 > tbs.len() {
        return None; // need at least serial, sig-alg, issuer, validity
    }
    let serial = tbs[at].integer()? as u64;
    let issuer = rdn_string(tbs.get(at + 2)?);
    let validity_tlv = tbs.get(at + 3)?.children()?;
    if validity_tlv.len() != 2 {
        return None;
    }
    let not_before = validity_tlv[0].utc_time()?;
    let not_after = validity_tlv[1].utc_time()?;
    let subject = rdn_string(tbs.get(at + 4)?);
    Some(Cert {
        version,
        serial,
        issuer,
        subject,
        validity: Validity {
            not_before,
            not_after,
        },
        sig_alg,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::der;

    fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
        der::encode(tag, content)
    }

    /// Build a minimal but structurally-valid cert.
    fn cert() -> Vec<u8> {
        // version [0] EXPLICIT INTEGER 2 (= v3)
        let mut tbs = Vec::new();
        tbs.extend_from_slice(&tlv(0xA0, &tlv(0x02, &[2]))); // version
        tbs.extend_from_slice(&tlv(0x02, &[0x11])); // serial 17
        tbs.extend_from_slice(&tlv(0x30, &tlv(0x06, &[0x2A, 0x03]))); // sig alg OID
                                                                      // issuer: one RDNSequence with CN=CA
        let issuer_name = tlv(
            0x30,
            &tlv(
                0x31,
                &tlv(0x30, &{
                    let mut c = tlv(0x06, &[0x55, 0x04, 0x03]); // CN OID
                    c.extend_from_slice(&tlv(0x0C, b"CA"));
                    c
                }),
            ),
        );
        tbs.extend_from_slice(&issuer_name);
        // validity
        let mut val = tlv(0x17, b"240101000000Z");
        val.extend_from_slice(&tlv(0x18, b"20350101000000Z")); // GeneralizedTime
        tbs.extend_from_slice(&tlv(0x30, &val));
        // subject CN=leaf
        let subject_name = tlv(
            0x30,
            &tlv(
                0x31,
                &tlv(0x30, &{
                    let mut c = tlv(0x06, &[0x55, 0x04, 0x03]);
                    c.extend_from_slice(&tlv(0x0C, b"leaf"));
                    c
                }),
            ),
        );
        tbs.extend_from_slice(&subject_name);
        // spki placeholder SEQ
        tbs.extend_from_slice(&tlv(0x30, &[]));

        let mut cert_inner = Vec::new();
        cert_inner.extend_from_slice(&tlv(0x30, &tbs));
        cert_inner.extend_from_slice(&tlv(0x30, &tlv(0x06, &[0x2A, 0x03]))); // sig alg
        cert_inner.extend_from_slice(&tlv(0x03, &[0, 0xDE, 0xAD])); // BIT STRING
        tlv(0x30, &cert_inner)
    }

    #[test]
    fn cert_fields() {
        let c = parse(&cert()).unwrap();
        assert_eq!(c.version, 3);
        assert_eq!(c.serial, 0x11);
        assert_eq!(c.issuer, "/CN=CA");
        assert_eq!(c.subject, "/CN=leaf");
        assert_eq!(c.sig_alg, "1.2.3"); // 0x2A=1.2, 0x03 → 1.2.3
        assert_eq!(c.signature, vec![0xDE, 0xAD]);
        // 2024-01-01 UTC = 1704067200
        assert_eq!(c.validity.not_before, 1704067200);
        // 2035-01-01 UTC = 2051222400
        assert_eq!(c.validity.not_after, 2051222400);
    }

    #[test]
    fn malformed_rejected() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x30, 0x00]).is_none()); // empty SEQ
                                                 // cert with only 2 top children
        let two = der::encode(0x30, &der::encode(0x30, &[]));
        assert!(parse(&two).is_none());
    }
}

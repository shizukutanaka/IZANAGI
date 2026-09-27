//! PKCS#8 `PrivateKeyInfo` (`.key` PEM `PRIVATE KEY` body):
//! `SEQUENCE { version INTEGER, algorithm SEQ{oid,params},
//! privateKey OCTET STRING [, attributes] }`. Built on `crate::der`.
//!
//! ```
//! use izanagi_kit::{der, pkcs8};
//!
//! let alg = der::encode(
//!     0x30,
//!     &[
//!         der::encode(0x06, &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01]),
//!         der::encode(0x05, &[]),
//!     ]
//!     .concat(),
//! );
//! let p8 = der::encode(
//!     0x30,
//!     &[der::encode(0x02, &[0]), alg, der::encode(0x04, &[1, 2, 3])].concat(),
//! );
//! let k = pkcs8::parse(&p8).unwrap();
//! assert_eq!(k.version, 0);
//! assert_eq!(k.algorithm_oid(), Some(pkcs8::oid::RSA.to_string()));
//! assert_eq!(k.private_key(), &[1, 2, 3]);
//! ```

use crate::der;
use std::string::String;
use std::vec::Vec;

/// Common `privateKeyAlgorithm` OIDs.
pub mod oid {
    /// `1.2.840.113549.1.1.1` — rsaEncryption.
    pub const RSA: &str = "1\x2e2\x2e840\x2e113549\x2e1\x2e1\x2e1";
    /// `1.2.840.10045.2.1` — ecPublicKey.
    pub const EC: &str = "1\x2e2\x2e840\x2e10045\x2e2\x2e1";
    /// `1.3.101.110` — X25519; `1.3.101.112` — Ed25519.
    pub const ED25519: &str = "1\x2e3\x2e101\x2e112";
}

/// Parsed `PrivateKeyInfo`.
#[derive(Debug, Clone)]
pub struct Pkcs8 {
    /// `version` INTEGER (0 = v1).
    pub version: i64,
    /// Algorithm OID.
    pub algorithm: String,
    /// Raw AlgorithmIdentifier parameters (`NULL` kept as `05 00`).
    pub params: Vec<u8>,
    /// The wrapped private key octets.
    pub private_key: Vec<u8>,
}

/// Parse a DER `PrivateKeyInfo`.
pub fn parse(d: &[u8]) -> Option<Pkcs8> {
    let top = der::parse(d)?;
    let seq = top.first()?;
    if seq.tag != 16 || !seq.constructed {
        return None;
    }
    let ch = seq.children()?;
    let version = ch.first()?.integer()?;
    let alg_id = ch.get(1)?;
    if alg_id.tag != 16 {
        return None;
    }
    let alg_ch = alg_id.children()?;
    let algorithm = alg_ch.first()?.oid()?;
    let params = alg_ch
        .get(1)
        .map(|t| {
            der::encode(
                (t.cls << 6) | if t.constructed { 0x20 } else { 0 } | t.tag.min(31) as u8,
                &t.content,
            )
        })
        .unwrap_or_default();
    let key_tlv = ch.get(2)?;
    if key_tlv.tag != 4 {
        return None;
    }
    Some(Pkcs8 {
        version,
        algorithm,
        params,
        private_key: key_tlv.content.clone(),
    })
}

/// Decode a PEM `PRIVATE KEY` block, then parse the DER body.
pub fn from_pem(src: &str) -> Option<Pkcs8> {
    let blocks = crate::pem::parse(src)?;
    let b = blocks
        .iter()
        .find(|b| b.label == "PRIVATE KEY" || b.label == "ENCRYPTED PRIVATE KEY")?;
    parse(&b.data)
}

impl Pkcs8 {
    /// Algorithm OID.
    pub fn algorithm_oid(&self) -> Option<String> {
        Some(self.algorithm.clone())
    }

    /// Private key octets.
    pub fn private_key(&self) -> &[u8] {
        &self.private_key
    }

    /// True when the outer label was `ENCRYPTED PRIVATE KEY`
    /// (caller's `from_pem` keeps the same shape: version+alg+opaque
    /// key blob).
    pub fn is_rsa(&self) -> bool {
        self.algorithm == oid::RSA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let alg = der::encode(
            0x30,
            &[
                der::encode(0x06, &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01]),
                der::encode(0x05, &[]),
            ]
            .concat(),
        );
        der::encode(
            0x30,
            &[der::encode(0x02, &[0]), alg, der::encode(0x04, &[9, 9])].concat(),
        )
    }

    #[test]
    fn fields() {
        let k = parse(&fixture()).unwrap();
        assert_eq!(k.version, 0);
        assert_eq!(k.private_key(), &[9, 9]);
        assert_eq!(k.params, vec![0x05, 0x00]);
        assert!(!k.is_rsa());
        assert_eq!(k.algorithm_oid().unwrap(), "1.3.6.1.5.5.7.3.1");
    }

    #[test]
    fn pem_path() {
        let body = crate::base64::encode(&fixture());
        let pem = format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n");
        let k = from_pem(&pem).unwrap();
        assert_eq!(k.version, 0);
        assert!(
            from_pem("-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n").is_none()
        );
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&der::encode(0x02, &[0])).is_none());
    }
}

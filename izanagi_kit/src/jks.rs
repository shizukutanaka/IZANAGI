//! Java KeyStore (JKS) file walking — read-only, no crypto.
//!
//! Big-endian: `0xFEEDFEED` magic, `version` u32 (=2), `count` u32,
//! then `count` entries: `tag` u32 (1=private key, 2=trusted cert),
//! `alias` (u16 len + UTF-8), `timestamp` u64 (millis); tag 1 then has
//! `key_len u32` + key bytes + `cert_count u32`; every entry ends with
//! `cert_count` cert records `{type u16len+utf, len u32, data}`.
//! The trailing 20-byte SHA-1 integrity digest is left unchecked.
//!
//! ```
//! use izanagi_kit::jks;
//! let mut d = Vec::new();
//! d.extend_from_slice(&0xFEEDFEEDu32.to_be_bytes());
//! d.extend_from_slice(&2u32.to_be_bytes()); // version
//! d.extend_from_slice(&1u32.to_be_bytes()); // count
//! d.extend_from_slice(&2u32.to_be_bytes()); // tag: cert
//! d.extend_from_slice(&3u16.to_be_bytes()); // alias len
//! d.extend_from_slice(b"key");
//! d.extend_from_slice(&0u64.to_be_bytes()); // timestamp
//! d.extend_from_slice(&5u16.to_be_bytes()); // cert type len
//! d.extend_from_slice(b"X.509");
//! d.extend_from_slice(&4u32.to_be_bytes()); // cert len
//! d.extend_from_slice(&[0x30, 0x82, 0, 0]);
//! d.extend_from_slice(&[0u8; 20]); // integrity digest
//! let j = jks::parse(&d).unwrap();
//! assert_eq!(j.entries.len(), 1);
//! ```

use std::vec::Vec;

/// File magic.
pub const MAGIC: u32 = 0xFEED_FEED;

/// One keystore entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// `tag`: 1 = private key, 2 = trusted certificate.
    pub tag: u32,
    /// Alias string (raw UTF-8 bytes).
    pub alias: Vec<u8>,
    /// Entry timestamp (epoch millis).
    pub timestamp: u64,
    /// Private-key entries only: DER key bytes.
    pub key: Option<Vec<u8>>,
    /// Certificate records `(type, data)`.
    pub certs: Vec<(Vec<u8>, Vec<u8>)>,
}

/// A parsed keystore.
#[derive(Clone, Debug, PartialEq)]
pub struct Jks {
    /// Format version (2 for JKS).
    pub version: u32,
    /// Entries.
    pub entries: Vec<Entry>,
    /// Offset of the trailing integrity digest (file len − 20).
    pub digest_offset: usize,
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    Some((*d.get(at)? as u16) << 8 | *d.get(at + 1)? as u16)
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

fn u64be(d: &[u8], at: usize) -> Option<u64> {
    Some((u32be(d, at)? as u64) << 32 | u32be(d, at + 4)? as u64)
}

fn utf<'a>(d: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
    let n = u16be(d, *at)? as usize;
    *at += 2;
    let s = d.get(*at..at.checked_add(n)?)?;
    *at += n;
    Some(s)
}

fn bytes<'a>(d: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
    let n = u32be(d, *at)? as usize;
    *at += 4;
    let s = d.get(*at..at.checked_add(n)?)?;
    *at += n;
    Some(s)
}

/// Parses a JKS file; requires magic, version 1/2, exact tiling, and
/// the trailing 20-byte integrity digest space.
pub fn parse(d: &[u8]) -> Option<Jks> {
    if d.len() < 12 + 20 {
        return None;
    }
    if u32be(d, 0)? != MAGIC {
        return None;
    }
    let version = u32be(d, 4)?;
    if !(1..=2).contains(&version) {
        return None;
    }
    let count = u32be(d, 8)? as usize;
    let mut at = 12;
    let mut entries = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let tag = u32be(d, at)?;
        at += 4;
        if !(1..=2).contains(&tag) {
            return None;
        }
        let alias = utf(d, &mut at)?.to_vec();
        let timestamp = u64be(d, at)?;
        at += 8;
        let mut key = None;
        let cert_count;
        if tag == 1 {
            let k = bytes(d, &mut at)?.to_vec();
            key = Some(k);
            cert_count = u32be(d, at)? as usize;
            at += 4;
        } else {
            cert_count = 1;
        }
        let mut certs = Vec::with_capacity(cert_count.min(64));
        for _ in 0..cert_count {
            let ty = utf(d, &mut at)?.to_vec();
            let data = bytes(d, &mut at)?.to_vec();
            certs.push((ty, data));
        }
        entries.push(Entry {
            tag,
            alias,
            timestamp,
            key,
            certs,
        });
    }
    // Integrity digest occupies the last 20 bytes.
    if at + 20 != d.len() {
        return None;
    }
    Some(Jks {
        version,
        entries,
        digest_offset: at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&MAGIC.to_be_bytes());
        d.extend_from_slice(&2u32.to_be_bytes());
        d.extend_from_slice(&1u32.to_be_bytes());
        d.extend_from_slice(&1u32.to_be_bytes()); // private key
        d.extend_from_slice(&5u16.to_be_bytes());
        d.extend_from_slice(b"mykey");
        d.extend_from_slice(&0u64.to_be_bytes());
        d.extend_from_slice(&4u32.to_be_bytes()); // key len
        d.extend_from_slice(&[0x30, 0x82, 1, 2]);
        d.extend_from_slice(&1u32.to_be_bytes()); // cert count
        d.extend_from_slice(&5u16.to_be_bytes());
        d.extend_from_slice(b"X.509");
        d.extend_from_slice(&4u32.to_be_bytes());
        d.extend_from_slice(&[0x30, 0x82, 0, 0]);
        d.extend_from_slice(&[0u8; 20]);
        d
    }

    #[test]
    fn parses_entries() {
        let j = parse(&fixture()).unwrap();
        assert_eq!(j.entries.len(), 1);
        assert_eq!(j.entries[0].alias, b"mykey".to_vec());
        assert_eq!(j.entries[0].certs.len(), 1);
        assert!(j.entries[0].key.is_some());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = fixture();
        d[0] = 0x00;
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d.pop(); // digest truncated
        assert!(parse(&d).is_none());
    }
}

//! KDBX — KeePass 2 database head: `0x9AA2D903` + `0xB54BFB65/66/67`
//! signature pair, u32LE version, then `{u8 id, u16LE len}` (KDBX3)
//! or `{u8 id, u32LE len}` (KDBX4) header fields until `id == 0`.
//!
//! ```
//! use izanagi_kit::kdbx::{parse, field};
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(&0x9AA2D903u32.to_le_bytes());
//! d.extend_from_slice(&0xB54BFB65u32.to_le_bytes());
//! d.extend_from_slice(&0x0003_0000u32.to_le_bytes()); // v3.0
//! d.push(field::CIPHER_ID);
//! d.extend_from_slice(&16u16.to_le_bytes());
//! d.extend_from_slice(&[0x31u8; 16]); // AES256 UUID
//! d.push(field::END);
//! d.extend_from_slice(&4u16.to_le_bytes());
//! d.extend_from_slice(&[0x0D, 0x0A, 0x0D, 0x0A]);
//! let k = parse(&d).unwrap();
//! assert_eq!(k.major_version, 3);
//! assert_eq!(k.field(field::CIPHER_ID).unwrap().len(), 16);
//! ```

use std::vec::Vec;

/// Signature part 1.
pub const SIG1: u32 = 0x9AA2_D903;
/// Signature part 2 — KDBX3 (`0xB54BFB65` / `0xB54BFB66`) or KDBX4 (`0xB54BFB67`).
pub const SIG2_V3: u32 = 0xB54B_FB65;
/// Pre-release 1.x form of the signature.
pub const SIG2_V3_ALT: u32 = 0xB54B_FB66;
/// KDBX4 form.
pub const SIG2_V4: u32 = 0xB54B_FB67;

/// Header field ids.
pub mod field {
    /// End of header.
    pub const END: u8 = 0;
    /// Comment.
    pub const COMMENT: u8 = 1;
    /// Cipher UUID (16 bytes; AES128/AES256/ChaCha20).
    pub const CIPHER_ID: u8 = 2;
    /// Compression flag (`0` none, `1` gzip).
    pub const COMPRESSION: u8 = 3;
    /// Master seed (32 bytes).
    pub const MASTER_SEED: u8 = 4;
    /// Transform seed (KDBX3).
    pub const TRANSFORM_SEED: u8 = 5;
    /// Transform rounds u64LE (KDBX3).
    pub const TRANSFORM_ROUNDS: u8 = 6;
    /// Encryption IV.
    pub const ENCRYPTION_IV: u8 = 7;
    /// Protected-stream key (KDBX3).
    pub const PROTECTED_KEY: u8 = 8;
    /// Stream start bytes (KDBX3).
    pub const STREAM_START: u8 = 9;
    /// Inner random-stream cipher id u32LE (KDBX3: 1=ARC4, 2=Salsa20).
    pub const INNER_STREAM: u8 = 10;
    /// KDBX4 inner header fields live in the encrypted payload.
    pub const KDF_PARAMETERS: u8 = 11;
}

/// One raw header field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Field id byte.
    pub id: u8,
    /// Field value bytes.
    pub value: Vec<u8>,
}

/// Parsed KDBX head.
#[derive(Debug, Clone)]
pub struct Kdbx {
    /// Version u32 (high = major, low = minor): `0x0003_0000` = 3.0.
    pub version: u32,
    /// Major version (`version >> 16`).
    pub major_version: u32,
    /// True for KDBX4 (u32 field lengths, inner header inside payload).
    pub v4: bool,
    /// Header fields, in order (excluding `END`).
    pub fields: Vec<Field>,
    /// Byte offset where the encrypted payload begins.
    pub payload_at: usize,
}

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// Parse a KDBX head.
pub fn parse(d: &[u8]) -> Option<Kdbx> {
    if le32(d, 0)? != SIG1 {
        return None;
    }
    let sig2 = le32(d, 4)?;
    let v4 = match sig2 {
        SIG2_V3 | SIG2_V3_ALT => false,
        SIG2_V4 => true,
        _ => return None,
    };
    let version = le32(d, 8)?;
    let mut fields = Vec::new();
    let mut at = 12usize;
    loop {
        let id = *d.get(at)?;
        let len = if v4 {
            le32(d, at + 1)? as usize
        } else {
            le16(d, at + 1)? as usize
        };
        let hdr = 1 + if v4 { 4 } else { 2 };
        let start = at + hdr;
        let end = start.checked_add(len)?;
        let value = d.get(start..end)?.to_vec();
        at = end;
        if id == field::END {
            break;
        }
        fields.push(Field { id, value });
    }
    Some(Kdbx {
        version,
        major_version: version >> 16,
        v4,
        fields,
        payload_at: at,
    })
}

impl Kdbx {
    /// First field with `id`.
    pub fn field(&self, id: u8) -> Option<&[u8]> {
        self.fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| &f.value[..])
    }

    /// Compression flag (`0` none, `1` gzip).
    pub fn compression(&self) -> Option<u32> {
        self.field(field::COMPRESSION)
            .and_then(|b| b.first().map(|&v| u32::from(v)))
    }

    /// KDBX3 transform rounds (u64LE) when present.
    pub fn transform_rounds(&self) -> Option<u64> {
        let b = self.field(field::TRANSFORM_ROUNDS)?;
        if b.len() < 8 {
            return None;
        }
        let mut v = 0u64;
        for (i, &x) in b[..8].iter().enumerate() {
            v |= u64::from(x) << (8 * i);
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(v4: bool) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&SIG1.to_le_bytes());
        d.extend_from_slice(&(if v4 { SIG2_V4 } else { SIG2_V3 }).to_le_bytes());
        d.extend_from_slice(&0x0004_0001u32.to_le_bytes());
        let push = |d: &mut Vec<u8>, id: u8, val: &[u8]| {
            d.push(id);
            if v4 {
                d.extend_from_slice(&(val.len() as u32).to_le_bytes());
            } else {
                d.extend_from_slice(&(val.len() as u16).to_le_bytes());
            }
            d.extend_from_slice(val);
        };
        push(&mut d, field::CIPHER_ID, &[0x31; 16]);
        push(&mut d, field::COMPRESSION, &[1]);
        push(&mut d, field::TRANSFORM_ROUNDS, &50_000u64.to_le_bytes());
        push(&mut d, field::END, &[0x0D, 0x0A, 0x0D, 0x0A]);
        d
    }

    #[test]
    fn v3() {
        let d = fixture(false);
        let k = parse(&d).unwrap();
        assert!(!k.v4);
        assert_eq!(k.major_version, 4); // version field is format version
        assert_eq!(k.fields.len(), 3);
        assert_eq!(k.compression(), Some(1));
        assert_eq!(k.transform_rounds(), Some(50_000));
        assert_eq!(k.payload_at, d.len());
    }

    #[test]
    fn v4() {
        let d = fixture(true);
        let k = parse(&d).unwrap();
        assert!(k.v4);
        assert_eq!(k.field(field::CIPHER_ID).unwrap().len(), 16);
        assert_eq!(k.field(99), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain").is_none());
        let mut d = fixture(false);
        d[4] = 0x99;
        assert!(parse(&d).is_none());
        let mut t = fixture(false);
        t.truncate(15);
        assert!(parse(&t).is_none());
    }
}

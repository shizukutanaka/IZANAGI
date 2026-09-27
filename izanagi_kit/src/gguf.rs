//! GGUF — llama.cpp's GGML Universal Format: `GGUF` magic +
//! u32 version + u64 tensor count + u64 metadata-kv count, then
//! typed key/value records (u64-prefixed string key, u32 value
//! type, type-shaped payload), then tensor infos.
//!
//! ```
//! use izanagi_kit::gguf::{parse, Value};
//!
//! let mut d = b"GGUF".to_vec();
//! d.extend_from_slice(&[3, 0, 0, 0]);           // version 3
//! d.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]); // tensor count
//! d.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]); // kv count
//! d.extend_from_slice(&[3, 0, 0, 0, 0, 0, 0, 0]); // key len 3
//! d.extend_from_slice(b"key");
//! d.extend_from_slice(&[4, 0, 0, 0]);           // type u32
//! d.extend_from_slice(&[42, 0, 0, 0]);          // u32 42
//! let g = parse(&d).unwrap();
//! let kv: Vec<_> = g.kv().collect();
//! assert_eq!(kv[0].0, "key");
//! assert_eq!(kv[0].1, Value::U64(42));
//! ```

use std::string::String;
use std::vec::Vec;

/// File magic.
pub const MAGIC: &[u8; 4] = b"GGUF";

/// A metadata value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Unsigned byte-width ints (u8/u16/u32/u64).
    U64(u64),
    /// Signed byte-width ints (i8/i16/i32/i64).
    I64(i64),
    /// 32/64-bit float, stored as raw bits.
    FloatBits(u64),
    /// Boolean.
    Bool(u8),
    /// String.
    Str(String),
    /// Typed array payload (element type id + raw bytes after
    /// the 12-byte array header: u32 elem type + u64 count).
    Arr(u32, u64),
}

/// Parsed header.
#[derive(Debug, Clone)]
pub struct Gguf<'a> {
    /// Format version (2/3 common).
    pub version: u32,
    /// Declared tensor-info count.
    pub tensor_count: u64,
    /// Declared metadata-kv count.
    pub kv_count: u64,
    /// Offset where the kv records begin.
    pub kv_at: usize,
    /// Offset past the last parsed kv record (`None` if a record
    /// overran the buffer).
    pub tensors_at: Option<usize>,
    kvs: Vec<(String, Value)>,
    d: &'a [u8],
}

fn le32(d: &[u8], i: usize) -> Option<u32> {
    let mut v = 0u32;
    for k in 0..4 {
        v |= u32::from(*d.get(i + k)?) << (8 * k);
    }
    Some(v)
}
fn le64(d: &[u8], i: usize) -> Option<u64> {
    let mut v = 0u64;
    for k in 0..8 {
        v |= u64::from(*d.get(i + k)?) << (8 * k);
    }
    Some(v)
}

fn fixed_width(ty: u32) -> Option<u64> {
    match ty {
        0 | 1 | 7 => Some(1),
        2 | 3 => Some(2),
        4..=6 => Some(4),
        10..=12 => Some(8),
        _ => None,
    }
}

/// Parse; requires the magic and the 24-byte head.
pub fn parse(d: &[u8]) -> Option<Gguf<'_>> {
    if d.get(..4)? != MAGIC {
        return None;
    }
    let version = le32(d, 4)?;
    let tensor_count = le64(d, 8)?;
    let kv_count = le64(d, 16)?;
    let mut at = 24usize;
    let mut kvs = Vec::new();
    for _ in 0..kv_count {
        let klen = usize::try_from(le64(d, at)?).ok()?;
        let key = String::from_utf8_lossy(d.get(at + 8..at + 8 + klen)?).into_owned();
        at = at.checked_add(8 + klen)?;
        let ty = le32(d, at)?;
        at += 4;
        let v = match ty {
            8 => {
                let slen = usize::try_from(le64(d, at)?).ok()?;
                let s = String::from_utf8_lossy(d.get(at + 8..at + 8 + slen)?).into_owned();
                at = at.checked_add(8 + slen)?;
                Value::Str(s)
            }
            9 => {
                let et = le32(d, at)?;
                let n = le64(d, at + 4)?;
                at = at.checked_add(4 + 8)?;
                if et == 8 {
                    // array of strings: walk each u64-prefixed string
                    for _ in 0..n {
                        let sl = usize::try_from(le64(d, at)?).ok()?;
                        at = at.checked_add(8 + sl)?;
                    }
                } else {
                    let w = usize::try_from(fixed_width(et)?).ok()?;
                    let bytes = usize::try_from(n).ok()?.checked_mul(w)?;
                    at = at.checked_add(bytes)?;
                }
                Value::Arr(et, n)
            }
            ty => {
                let w = usize::try_from(fixed_width(ty)?).ok()?;
                let mut raw = 0u64;
                for k in 0..w {
                    raw |= u64::from(*d.get(at + k)?) << (8 * k);
                }
                at = at.checked_add(w)?;
                match ty {
                    0 | 2 | 4 | 10 => Value::U64(raw),
                    1 | 3 | 5 | 11 => {
                        let sign = 1u64 << (8 * w - 1);
                        let x = if raw & sign != 0 && w < 8 {
                            raw.wrapping_sub(1 << (8 * w))
                        } else {
                            raw
                        };
                        Value::I64(x as i64)
                    }
                    6 | 12 => Value::FloatBits(raw),
                    _ => Value::Bool((raw & 1) as u8),
                }
            }
        };
        kvs.push((key, v));
    }
    Some(Gguf {
        version,
        tensor_count,
        kv_count,
        kv_at: 24,
        tensors_at: Some(at),
        kvs,
        d,
    })
}

impl<'a> Gguf<'a> {
    /// Parsed kv records.
    pub fn kv(&self) -> impl Iterator<Item = &(String, Value)> {
        self.kvs.iter()
    }

    /// Look up a metadata value by key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.kvs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Raw bytes of a tensor-info record `i` (best effort: returns
    /// the 4-byte name length of tensor `i`'s name field).
    pub fn raw(&self) -> &'a [u8] {
        self.d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(version: u32, tensors: u64, kv: u64) -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&u32::to_le_bytes(version));
        d.extend_from_slice(&u64::to_le_bytes(tensors));
        d.extend_from_slice(&u64::to_le_bytes(kv));
        d
    }

    #[test]
    fn kv_types() {
        let mut d = header(3, 0, 4);
        // "s" = "hi"
        d.extend_from_slice(&u64::to_le_bytes(1));
        d.push(b's');
        d.extend_from_slice(&u32::to_le_bytes(8));
        d.extend_from_slice(&u64::to_le_bytes(2));
        d.extend_from_slice(b"hi");
        // "n" = u32 7
        d.extend_from_slice(&u64::to_le_bytes(1));
        d.push(b'n');
        d.extend_from_slice(&u32::to_le_bytes(4));
        d.extend_from_slice(&u32::to_le_bytes(7));
        // "i" = i32 -2
        d.extend_from_slice(&u64::to_le_bytes(1));
        d.push(b'i');
        d.extend_from_slice(&u32::to_le_bytes(5));
        d.extend_from_slice(&u32::to_le_bytes(u32::MAX - 1));
        // "a" = array of two u16
        d.extend_from_slice(&u64::to_le_bytes(1));
        d.push(b'a');
        d.extend_from_slice(&u32::to_le_bytes(9));
        d.extend_from_slice(&u32::to_le_bytes(2)); // elem u16
        d.extend_from_slice(&u64::to_le_bytes(2)); // count 2
        d.extend_from_slice(&u16::to_le_bytes(1));
        d.extend_from_slice(&u16::to_le_bytes(2));
        let g = parse(&d).unwrap();
        assert_eq!(g.version, 3);
        assert_eq!(g.get("s"), Some(&Value::Str(String::from("hi"))));
        assert_eq!(g.get("n"), Some(&Value::U64(7)));
        assert_eq!(g.get("i"), Some(&Value::I64(-2)));
        assert_eq!(g.get("a"), Some(&Value::Arr(2, 2)));
        assert!(g.tensors_at.is_some());
        assert_eq!(g.kv().count(), 4);
        assert!(g.raw().starts_with(MAGIC));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"GGUF").is_none());
        // kv record overruns buffer
        let mut d = header(3, 0, 1);
        d.extend_from_slice(&u64::to_le_bytes(100));
        assert!(parse(&d).is_none());
    }
}

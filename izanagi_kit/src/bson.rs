//! BSON (binary JSON, MongoDB wire): `i32` total length + typed
//! elements + `\x00`. Element values decode into `Val`; IEEE-754
//! payloads (`Double`, `Decimal128`) keep raw bits — this crate
//! forbids float types in production code. `emit` re-serializes a
//! `Val::Doc` in field order (BSON is order-sensitive).
//!
//! ```
//! use izanagi_kit::bson::{self, Val};
//! let doc = vec![
//!     ("a".to_string(), Val::I32(42)),
//!     ("s".to_string(), Val::Str("hi".to_string())),
//! ];
//! let bytes = bson::emit(&Val::Doc(doc));
//! let (v, used) = bson::parse(&bytes).unwrap();
//! assert_eq!(used, bytes.len());
//! assert_eq!(v.get("a"), Some(&Val::I32(42)));
//! assert_eq!(v.get("s"), Some(&Val::Str("hi".to_string())));
//! ```

fn r32(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at + 4)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}
fn r64(d: &[u8], at: usize) -> Option<u64> {
    let mut v = 0u64;
    for i in 0..8 {
        v |= (*d.get(at + i)? as u64) << (i * 8);
    }
    Some(v)
}

/// A BSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    /// 0x01 — IEEE-754 binary64, kept as raw bits.
    Double(u64),
    /// 0x02 — UTF-8 string.
    Str(String),
    /// 0x03 — embedded document.
    Doc(Vec<(String, Val)>),
    /// 0x04 — array (document with numeric keys).
    Arr(Vec<(String, Val)>),
    /// 0x05 — binary blob: (subtype, bytes).
    Bin(u8, Vec<u8>),
    /// 0x07 — ObjectId (12 bytes).
    Oid([u8; 12]),
    /// 0x08 — boolean.
    Bool(bool),
    /// 0x09 — UTC datetime, ms since epoch.
    Date(i64),
    /// 0x0A — null.
    Null,
    /// 0x10 — int32.
    I32(i32),
    /// 0x11 — timestamp (increment, seconds).
    Ts(u64),
    /// 0x12 — int64.
    I64(i64),
    /// 0x13 — IEEE-754 decimal128, kept as raw bits.
    Dec128(u128),
    /// 0xFF — min-key sentinel.
    MinKey,
    /// 0x7F — max-key sentinel.
    MaxKey,
}

impl Val {
    /// Look up a field in a document.
    pub fn get(&self, name: &str) -> Option<&Val> {
        match self {
            Val::Doc(kv) | Val::Arr(kv) => kv.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }
}

fn cstring(d: &[u8], at: usize) -> Option<(String, usize)> {
    let end = d.get(at..)?.iter().position(|&b| b == 0)? + at;
    let s = std::str::from_utf8(&d[at..end]).ok()?.to_string();
    Some((s, end + 1))
}

/// One element payload starting at `at`. Returns (value, next offset).
fn value(d: &[u8], ty: u8, at: usize) -> Option<(Val, usize)> {
    let (v, next) = match ty {
        0x01 => (Val::Double(r64(d, at)?), at + 8),
        0x02 => {
            let n = r32(d, at)? as usize;
            let end = at.checked_add(4)?.checked_add(n)?;
            if n == 0 || end > d.len() || d[end - 1] != 0 {
                return None;
            }
            (
                Val::Str(std::str::from_utf8(&d[at + 4..end - 1]).ok()?.to_string()),
                end,
            )
        }
        0x03 | 0x04 => {
            let (v, used) = doc(d, at)?;
            let target = at.checked_add(used)?;
            (if ty == 0x03 { Val::Doc(v) } else { Val::Arr(v) }, target)
        }
        0x05 => {
            let n = r32(d, at)? as usize;
            let sub = *d.get(at + 4)?;
            let end = at.checked_add(5)?.checked_add(n)?;
            (Val::Bin(sub, d.get(at + 5..end)?.to_vec()), end)
        }
        0x07 => {
            let s = d.get(at..at + 12)?;
            let mut id = [0u8; 12];
            id.copy_from_slice(s);
            (Val::Oid(id), at + 12)
        }
        0x08 => match d.get(at)? {
            0 => (Val::Bool(false), at + 1),
            1 => (Val::Bool(true), at + 1),
            _ => return None,
        },
        0x09 => (Val::Date(r64(d, at)? as i64), at + 8),
        0x0A => (Val::Null, at),
        0x10 => (Val::I32(r32(d, at)? as i32), at + 4),
        0x11 => (Val::Ts(r64(d, at)?), at + 8),
        0x12 => (Val::I64(r64(d, at)? as i64), at + 8),
        0x13 => (
            Val::Dec128(r64(d, at)? as u128 | (r64(d, at + 8)? as u128) << 64),
            at + 16,
        ),
        0xFF => (Val::MinKey, at),
        0x7F => (Val::MaxKey, at),
        _ => return None,
    };
    Some((v, next))
}

/// A document body at `at` (length field included). Returns the
/// field list and bytes consumed.
fn doc(d: &[u8], at: usize) -> Option<(Vec<(String, Val)>, usize)> {
    let len = r32(d, at)? as usize;
    if len < 5 || at.checked_add(len)? > d.len() {
        return None;
    }
    let end = at + len;
    if d[end - 1] != 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut i = at + 4;
    while i < end - 1 {
        let ty = *d.get(i)?;
        let (name, after) = cstring(d, i + 1)?;
        let (v, next) = value(d, ty, after)?;
        if next > end - 1 {
            return None;
        }
        out.push((name, v));
        i = next;
    }
    if i != end - 1 {
        return None;
    }
    Some((out, len))
}

/// Parse a BSON document at the start of `d`. Returns the value and
/// the consumed byte count (trailing bytes are the caller's concern —
/// MongoDB streams pack documents back-to-back).
pub fn parse(d: &[u8]) -> Option<(Val, usize)> {
    let (fields, used) = doc(d, 0)?;
    Some((Val::Doc(fields), used))
}

/// Serialize a `Val::Doc`/`Val::Arr` (a bare scalar emits as its
/// type byte is not representable — returns an empty vector).
pub fn emit(v: &Val) -> Vec<u8> {
    match v {
        Val::Doc(kv) | Val::Arr(kv) => {
            let mut body = Vec::new();
            for (k, val) in kv {
                emit_elem(&mut body, k, val);
            }
            body.push(0);
            let len = body.len() + 4;
            let mut out = Vec::with_capacity(len);
            out.extend_from_slice(&[
                len as u8,
                (len >> 8) as u8,
                (len >> 16) as u8,
                (len >> 24) as u8,
            ]);
            out.extend_from_slice(&body);
            out
        }
        _ => Vec::new(),
    }
}

fn emit_elem(out: &mut Vec<u8>, name: &str, v: &Val) {
    let (ty, payload): (u8, Vec<u8>) = match v {
        Val::Double(bits) => (0x01, le64(*bits)),
        Val::Str(s) => {
            let mut p = Vec::with_capacity(s.len() + 5);
            p.extend_from_slice(&le32(s.len() as u32 + 1));
            p.extend_from_slice(s.as_bytes());
            p.push(0);
            (0x02, p)
        }
        Val::Doc(_) | Val::Arr(_) => (if matches!(v, Val::Doc(_)) { 0x03 } else { 0x04 }, emit(v)),
        Val::Bin(sub, b) => {
            let mut p = Vec::with_capacity(b.len() + 5);
            p.extend_from_slice(&le32(b.len() as u32));
            p.push(*sub);
            p.extend_from_slice(b);
            (0x05, p)
        }
        Val::Oid(id) => (0x07, id.to_vec()),
        Val::Bool(b) => (0x08, vec![*b as u8]),
        Val::Date(ms) => (0x09, le64(*ms as u64)),
        Val::Null => (0x0A, Vec::new()),
        Val::I32(v) => (0x10, le32(*v as u32)),
        Val::Ts(v) => (0x11, le64(*v)),
        Val::I64(v) => (0x12, le64(*v as u64)),
        Val::Dec128(v) => {
            let mut p = le64(*v as u64);
            p.extend_from_slice(&le64((*v >> 64) as u64));
            (0x13, p)
        }
        Val::MinKey => (0xFF, Vec::new()),
        Val::MaxKey => (0x7F, Vec::new()),
    };
    out.push(ty);
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out.extend_from_slice(&payload);
}

fn le32(v: u32) -> Vec<u8> {
    vec![v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8]
}
fn le64(v: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    for i in 0..8 {
        out.push((v >> (i * 8)) as u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_doc_roundtrips() {
        // {"hello": "world"} — MongoDB's own example
        let bytes = [
            0x16, 0x00, 0x00, 0x00, // len = 22
            0x02, b'h', b'e', b'l', b'l', b'o', 0x00, // type str, name
            0x06, 0x00, 0x00, 0x00, b'w', b'o', b'r', b'l', b'd', 0x00, // "world"
            0x00,
        ];
        let (v, used) = parse(&bytes).unwrap();
        assert_eq!(used, 22);
        assert_eq!(v.get("hello"), Some(&Val::Str("world".to_string())));
        assert_eq!(emit(&v), bytes.to_vec()); // canonical re-emit
    }

    #[test]
    fn all_types_roundtrip() {
        let doc = vec![
            ("d".into(), Val::Double(0x3FF0_0000_0000_0000)),
            ("arr".into(), Val::Arr(vec![("0".into(), Val::I32(1))])),
            ("sub".into(), Val::Doc(vec![("x".into(), Val::Null)])),
            ("bin".into(), Val::Bin(0x80, vec![1, 2, 3])),
            ("oid".into(), Val::Oid([7; 12])),
            ("b".into(), Val::Bool(true)),
            ("dt".into(), Val::Date(-12345)),
            ("i64".into(), Val::I64(-2)),
            ("ts".into(), Val::Ts(0xAAAA_BBBB)),
            (
                "dec".into(),
                Val::Dec128(0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF),
            ),
            ("min".into(), Val::MinKey),
            ("max".into(), Val::MaxKey),
        ];
        let bytes = emit(&Val::Doc(doc.clone()));
        let (v, used) = parse(&bytes).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(v, Val::Doc(doc));
    }

    #[test]
    fn malformed_rejected() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[4, 0, 0, 0]).is_none()); // len<5
        let mut d = emit(&Val::Doc(vec![("a".into(), Val::I32(1))]));
        *d.last_mut().unwrap() = 1; // bad terminator
        assert!(parse(&d).is_none());
        d.truncate(d.len() - 3); // truncated element
        assert!(parse(&d).is_none());
        let mut e = emit(&Val::Doc(vec![("a".into(), Val::Null)]));
        e[4] = 0x42; // unknown type
        assert!(parse(&e).is_none());
    }

    #[test]
    fn trailing_bytes_reported() {
        let mut d = emit(&Val::Doc(vec![("a".into(), Val::Null)]));
        let n = d.len();
        d.extend_from_slice(&[0xFF; 9]);
        let (_, used) = parse(&d).unwrap();
        assert_eq!(used, n);
    }
}

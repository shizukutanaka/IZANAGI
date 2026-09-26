//! Amazon Ion 1.0 **binary** format — the typed TLV stream of the
//! Ion spec (ion-docs «binary» chapter).
//!
//! A binary stream opens with the version marker `E0 01 00 EA`
//! (that's a *value* of type 14 ann of length 3). Each value is
//! `{typedesc: u8, [varuint len], payload}`: high nibble is the
//! type id, low nibble the length — `0..=13` literal, `14` means a
//! varuint length follows, `15` marks null/EOF-container forms.
//!
//! ```
//! use izanagi_kit::ion::{parse, values, kind_name};
//! // BVM + int value 0x21 0x05 (type 2 pos-int, len 1, byte 0x05)
//! let d = [0xE0, 0x01, 0x00, 0xEA, 0x21, 0x05];
//! let i = parse(&d).unwrap();
//! assert_eq!(i.data_at, 4);
//! let vs = values(&d, i.data_at, d.len());
//! assert_eq!(vs.len(), 1);
//! assert_eq!(vs[0].type_id, 2);
//! assert_eq!(kind_name(2), "pos_int");
//! assert_eq!(&d[vs[0].at..vs[0].at + vs[0].len], &[5]);
//! ```

/// The 4-byte binary version marker.
pub const MAGIC: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

/// A decoded Ion value header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Value {
    /// Type id (high nibble): 0 null … 14 annotation.
    pub type_id: u8,
    /// Payload byte length (`0` for nulls).
    pub len: usize,
    /// Payload offset.
    pub at: usize,
    /// Offset just past this value.
    pub next: usize,
    /// `true` when the low nibble was `15` (null/terminator form).
    pub is_null: bool,
}

/// A parsed stream head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ion {
    /// Offset of the first top-level value (past the BVM).
    pub data_at: usize,
}

/// Ion type id → symbolic name (`null`, `bool`, `pos_int`,
/// `neg_int`, `float`, `decimal`, `timestamp`, `symbol`, `string`,
/// `clob`, `blob`, `list`, `sexp`, `struct`, `annotation`, other).
pub fn kind_name(id: u8) -> &'static str {
    match id {
        0 => "null",
        1 => "bool",
        2 => "pos_int",
        3 => "neg_int",
        4 => "float",
        5 => "decimal",
        6 => "timestamp",
        7 => "symbol",
        8 => "string",
        9 => "clob",
        10 => "blob",
        11 => "list",
        12 => "sexp",
        13 => "struct",
        14 => "annotation",
        _ => "unknown",
    }
}

/// Ion varuint: 7-bit groups, high bit terminates. Returns
/// `(value, bytes)`. `None` on truncation or > 9 bytes.
pub fn varuint(d: &[u8], at: usize) -> Option<(u64, usize)> {
    let mut v: u64 = 0;
    let mut i = at;
    loop {
        let b = *d.get(i)?;
        v = (v << 7) | (b & 0x7F) as u64;
        i += 1;
        if b & 0x80 != 0 {
            return Some((v, i - at));
        }
        if i - at >= 9 {
            return None;
        }
    }
}

/// Require the binary version marker.
pub fn parse(d: &[u8]) -> Option<Ion> {
    if d.get(0..4)? != MAGIC {
        return None;
    }
    Some(Ion { data_at: 4 })
}

/// Decode one value header at `at`. `None` on truncation.
pub fn value_at(d: &[u8], at: usize) -> Option<Value> {
    let td = *d.get(at)?;
    let type_id = td >> 4;
    let lo = td & 0x0F;
    let mut at = at + 1;
    let (len, is_null) = match lo {
        15 => (0, true),
        14 => {
            let (l, n) = varuint(d, at)?;
            at += n;
            (l as usize, false)
        }
        n => (n as usize, false),
    };
    let end = at.checked_add(len)?;
    if end > d.len() {
        return None;
    }
    Some(Value {
        type_id,
        len,
        at,
        next: end,
        is_null,
    })
}

/// Walk top-level values between `at` and `end`, stopping at the
/// first malformed header or a null padding terminator.
pub fn values(d: &[u8], mut at: usize, end: usize) -> Vec<Value> {
    let mut out = Vec::new();
    while at < end {
        match value_at(d, at) {
            Some(v) if v.next <= end => {
                if v.is_null {
                    break; // null pad/terminator ends the walk
                }
                out.push(v);
                at = v.next;
            }
            _ => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        // pos_int len 1
        d.extend_from_slice(&[0x21, 0x05]);
        // string len 3
        d.extend_from_slice(&[0x83, b'f', b'o', b'o']);
        // list len varuint: 14-len form, len=2 (varuint 0x82), 2 bytes
        d.extend_from_slice(&[0xBE, 0x82, 0x20, 0x01]);
        d
    }

    #[test]
    fn header_and_walk() {
        let d = fixture();
        let i = parse(&d).unwrap();
        assert_eq!(i.data_at, 4);
        let vs = values(&d, 4, d.len());
        assert_eq!(vs.len(), 3);
        assert_eq!(vs[0].type_id, 2);
        assert_eq!(kind_name(vs[0].type_id), "pos_int");
        assert_eq!(vs[1].type_id, 8);
        assert_eq!(&d[vs[1].at..vs[1].at + vs[1].len], b"foo");
        assert_eq!(vs[2].type_id, 11);
        assert_eq!(vs[2].len, 2);
        assert_eq!(vs[2].next, d.len());
    }

    #[test]
    fn varuint_roundtrip() {
        assert_eq!(varuint(&[0x85], 0), Some((5, 1)));
        assert_eq!(varuint(&[0x01, 0x80], 0), Some((128, 2)));
        assert_eq!(varuint(&[0x00], 0), None); // never terminates
        assert_eq!(varuint(&[], 0), None);
    }

    #[test]
    fn null_terminates_walk() {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&[0x21, 0x05]);
        d.push(0x0F); // null.null
        d.extend_from_slice(&[0x21, 0x09]); // beyond terminator
        let vs = values(&d, 4, d.len());
        assert_eq!(vs.len(), 1);
        let v = value_at(&d, vs[0].next).unwrap();
        assert!(v.is_null);
        assert_eq!(kind_name(14), "annotation");
        assert_eq!(kind_name(15), "unknown");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0xE0, 0x01, 0x00, 0xEB]).is_none());
        // truncated value
        let mut d = MAGIC.to_vec();
        d.push(0x21); // declares 1 byte
        let vs = values(&d, 4, d.len());
        assert!(vs.is_empty());
        assert!(value_at(&d, 4).is_none());
    }
}

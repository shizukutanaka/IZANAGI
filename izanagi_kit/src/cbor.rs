//! Deterministic CBOR (RFC 7049, canonical form per §4.2) — a compact binary
//! codec as the binary sibling of [`crate::json`].
//!
//! [`Cbor`] covers the core data model: unsigned and negative integers (stored
//! by wire value `n`), byte/text strings, arrays, maps, booleans, and null —
//! no floats, no tags, no indefinite lengths, which keeps values
//! integer-exact and replay-safe.
//!
//! [`encode`] always emits shortest-form lengths and sorts map keys
//! canonically; [`decode`] accepts any definite-length item and reports the
//! consumed byte count via [`decode_prefix`], so trailing data is detectable.
//!
//! ```
//! use izanagi_kit::cbor::{self, Cbor};
//! let v = Cbor::Map(vec![(Cbor::Text("a".into()), Cbor::UInt(1))]);
//! assert_eq!(cbor::encode(&v), vec![0xa1, 0x61, 0x61, 0x01]);
//! assert_eq!(cbor::decode(&[0xa1, 0x61, 0x61, 0x01]), Some(v));
//! ```

/// A CBOR data item (definite-length, tag- and float-free subset).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cbor {
    /// Major type 0 — non-negative integer.
    UInt(u64),
    /// Major type 1 — `-1 - n`; stores `n` (the wire value), so `NInt(0)` is
    /// the integer -1.
    NInt(u64),
    /// Major type 2 — byte string.
    Bytes(Vec<u8>),
    /// Major type 3 — UTF-8 text string.
    Text(String),
    /// Major type 4 — array.
    Arr(Vec<Cbor>),
    /// Major type 5 — key/value pairs.
    Map(Vec<(Cbor, Cbor)>),
    /// Simple value `false`.
    False,
    /// Simple value `true`.
    True,
    /// Simple value `null`.
    Null,
}

fn head(major: u8, n: u64, out: &mut Vec<u8>) {
    let mt = major << 5;
    if n < 24 {
        out.push(mt | n as u8);
    } else if n <= 0xff {
        out.push(mt | 24);
        out.push(n as u8);
    } else if n <= 0xffff {
        out.push(mt | 25);
        push_u16be(out, n as u16);
    } else if n <= 0xffff_ffff {
        out.push(mt | 26);
        push_u32be(out, n as u32);
    } else {
        out.push(mt | 27);
        push_u64be(out, n);
    }
}

fn push_u16be(out: &mut Vec<u8>, v: u16) {
    out.push((v >> 8) as u8);
    out.push(v as u8);
}
fn push_u32be(out: &mut Vec<u8>, v: u32) {
    for i in (0..4).rev() {
        out.push((v >> (i * 8)) as u8);
    }
}
fn push_u64be(out: &mut Vec<u8>, v: u64) {
    for i in (0..8).rev() {
        out.push((v >> (i * 8)) as u8);
    }
}

/// Append the canonical encoding of `v` to `out`.
pub fn encode_to(v: &Cbor, out: &mut Vec<u8>) {
    match v {
        Cbor::UInt(n) => head(0, *n, out),
        Cbor::NInt(n) => head(1, *n, out),
        Cbor::Bytes(b) => {
            head(2, b.len() as u64, out);
            out.extend_from_slice(b);
        }
        Cbor::Text(t) => {
            head(3, t.len() as u64, out);
            out.extend_from_slice(t.as_bytes());
        }
        Cbor::Arr(a) => {
            head(4, a.len() as u64, out);
            for x in a {
                encode_to(x, out);
            }
        }
        Cbor::Map(m) => {
            head(5, m.len() as u64, out);
            // Canonical order: shortest encoding first, then byte order.
            let mut pairs: Vec<(Vec<u8>, &Cbor, &Cbor)> = m
                .iter()
                .map(|(k, val)| {
                    let mut kb = Vec::new();
                    encode_to(k, &mut kb);
                    (kb, k, val)
                })
                .collect();
            pairs.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then(a.0.cmp(&b.0)));
            for (kb, _, val) in pairs {
                out.extend_from_slice(&kb);
                encode_to(val, out);
            }
        }
        Cbor::False => out.push(0xf4),
        Cbor::True => out.push(0xf5),
        Cbor::Null => out.push(0xf6),
    }
}

/// Return the canonical encoding of `v`.
pub fn encode(v: &Cbor) -> Vec<u8> {
    let mut out = Vec::new();
    encode_to(v, &mut out);
    out
}

fn read_len(b: &[u8], i: &mut usize, info: u8) -> Option<u64> {
    let take = |n: usize, i: &mut usize, b: &[u8]| -> Option<u64> {
        if b.len() - *i < n {
            return None;
        }
        let mut v = 0u64;
        for _ in 0..n {
            v = (v << 8) | b[*i] as u64;
            *i += 1;
        }
        Some(v)
    };
    match info {
        0..=23 => Some(info as u64),
        24 => take(1, i, b),
        25 => take(2, i, b),
        26 => take(4, i, b),
        27 => take(8, i, b),
        _ => None, // 28-30 reserved, 31 = indefinite
    }
}

fn item(b: &[u8], i: &mut usize, depth: usize) -> Option<Cbor> {
    if depth > 64 || *i >= b.len() {
        return None;
    }
    let ib = b[*i];
    *i += 1;
    let (major, info) = (ib >> 5, ib & 31);
    match major {
        0 => Some(Cbor::UInt(read_len(b, i, info)?)),
        1 => Some(Cbor::NInt(read_len(b, i, info)?)),
        2 | 3 => {
            let n = read_len(b, i, info)? as usize;
            if b.len() - *i < n {
                return None;
            }
            let s = &b[*i..*i + n];
            *i += n;
            if major == 2 {
                Some(Cbor::Bytes(s.to_vec()))
            } else {
                core::str::from_utf8(s)
                    .ok()
                    .map(|t| Cbor::Text(t.to_string()))
            }
        }
        4 => {
            let n = read_len(b, i, info)?;
            let mut a = Vec::new();
            for _ in 0..n {
                a.push(item(b, i, depth + 1)?);
            }
            Some(Cbor::Arr(a))
        }
        5 => {
            let n = read_len(b, i, info)?;
            let mut m = Vec::new();
            for _ in 0..n {
                let k = item(b, i, depth + 1)?;
                let v = item(b, i, depth + 1)?;
                m.push((k, v));
            }
            Some(Cbor::Map(m))
        }
        _ => match ib {
            0xf4 => Some(Cbor::False),
            0xf5 => Some(Cbor::True),
            0xf6 => Some(Cbor::Null),
            _ => None,
        },
    }
}

/// Decode one item from the front of `b`, returning `(value, bytes_used)`.
/// `None` on truncation, malformed items, floats, tags, indefinite lengths,
/// or nesting deeper than 64.
pub fn decode_prefix(b: &[u8]) -> Option<(Cbor, usize)> {
    let mut i = 0;
    let v = item(b, &mut i, 0)?;
    Some((v, i))
}

/// Decode `b` as exactly one item — trailing bytes are an error.
pub fn decode(b: &[u8]) -> Option<Cbor> {
    match decode_prefix(b) {
        Some((v, n)) if n == b.len() => Some(v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(v: Cbor, want: &[u8]) {
        assert_eq!(encode(&v), want);
        assert_eq!(decode(want), Some(v));
    }

    #[test]
    fn rfc7049_integers() {
        rt(Cbor::UInt(0), &[0x00]);
        rt(Cbor::UInt(1), &[0x01]);
        rt(Cbor::UInt(10), &[0x0a]);
        rt(Cbor::UInt(23), &[0x17]);
        rt(Cbor::UInt(24), &[0x18, 0x18]);
        rt(Cbor::UInt(100), &[0x18, 0x64]);
        rt(Cbor::UInt(255), &[0x18, 0xff]);
        rt(Cbor::UInt(256), &[0x19, 0x01, 0x00]);
        rt(Cbor::UInt(1000), &[0x19, 0x03, 0xe8]);
        rt(Cbor::UInt(1_000_000), &[0x1a, 0x00, 0x0f, 0x42, 0x40]);
        rt(
            Cbor::UInt(1_000_000_000_000),
            &[0x1b, 0x00, 0x00, 0x00, 0xe8, 0xd4, 0xa5, 0x10, 0x00],
        );
        rt(
            Cbor::UInt(u64::MAX),
            &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );
        rt(Cbor::NInt(0), &[0x20]); // -1
        rt(Cbor::NInt(9), &[0x29]); // -10
        rt(Cbor::NInt(99), &[0x38, 0x63]); // -100
        rt(Cbor::NInt(999), &[0x39, 0x03, 0xe7]); // -1000
    }

    #[test]
    fn rfc7049_strings() {
        rt(Cbor::Bytes(vec![]), &[0x40]);
        rt(Cbor::Bytes(vec![1, 2, 3, 4]), &[0x44, 1, 2, 3, 4]);
        rt(Cbor::Text("".into()), &[0x60]);
        rt(Cbor::Text("a".into()), &[0x61, 0x61]);
        rt(Cbor::Text("IETF".into()), &[0x64, b'I', b'E', b'T', b'F']);
    }

    #[test]
    fn rfc7049_compound() {
        rt(Cbor::Arr(vec![]), &[0x80]);
        rt(
            Cbor::Arr(vec![Cbor::UInt(1), Cbor::UInt(2), Cbor::UInt(3)]),
            &[0x83, 1, 2, 3],
        );
        rt(
            Cbor::Arr(vec![
                Cbor::UInt(1),
                Cbor::Arr(vec![Cbor::UInt(2), Cbor::UInt(3)]),
                Cbor::Arr(vec![Cbor::UInt(4), Cbor::UInt(5)]),
            ]),
            &[0x83, 1, 0x82, 2, 3, 0x82, 4, 5],
        );
        rt(Cbor::Map(vec![]), &[0xa0]);
        rt(
            Cbor::Map(vec![
                (Cbor::UInt(1), Cbor::UInt(2)),
                (Cbor::UInt(3), Cbor::UInt(4)),
            ]),
            &[0xa2, 1, 2, 3, 4],
        );
        rt(Cbor::False, &[0xf4]);
        rt(Cbor::True, &[0xf5]);
        rt(Cbor::Null, &[0xf6]);
    }

    #[test]
    fn canonical_map_order() {
        // Encoding sorts keys by (length, bytes): 10 < "aa" < -1? length of
        // 10 is 1 byte, "aa" is 3, NInt(0) is 1 → 0x0a then 0x20 then 0x626161.
        let v = Cbor::Map(vec![
            (Cbor::Text("aa".into()), Cbor::UInt(1)),
            (Cbor::NInt(0), Cbor::UInt(2)),
            (Cbor::UInt(10), Cbor::UInt(3)),
        ]);
        assert_eq!(
            encode(&v),
            vec![0xa3, 0x0a, 3, 0x20, 2, 0x62, b'a', b'a', 1]
        );
    }

    #[test]
    fn decode_accepts_noncanonical_lengths() {
        // Long-form encoding of a small number still decodes.
        assert_eq!(decode(&[0x18, 0x00]), Some(Cbor::UInt(0)));
        assert_eq!(decode(&[0x1a, 0, 0, 0, 5]), Some(Cbor::UInt(5)));
    }

    #[test]
    fn decode_prefix_reports_consumed() {
        let mut b = encode(&Cbor::UInt(7));
        b.extend_from_slice(&[0xf6]);
        assert_eq!(decode_prefix(&b), Some((Cbor::UInt(7), 1)));
        assert_eq!(decode(&b), None); // trailing data
    }

    #[test]
    fn decode_rejects() {
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[0x1b, 1, 2]), None); // truncated u64
        assert_eq!(decode(&[0x5f]), None); // indefinite bytes
        assert_eq!(decode(&[0x9f]), None); // indefinite array
        assert_eq!(decode(&[0xc0]), None); // tag
        assert_eq!(decode(&[0xf9, 0, 0]), None); // float16
        assert_eq!(decode(&[0x62, b'a', 0xff]), None); // bad UTF-8
        assert_eq!(decode(&[0x82, 0x01]), None); // truncated array
    }

    #[test]
    fn encode_to_appends() {
        let mut out = vec![0xaa];
        encode_to(&Cbor::UInt(1), &mut out);
        assert_eq!(out, vec![0xaa, 0x01]);
    }

    #[test]
    fn round_trip_varied() {
        let v = Cbor::Arr(vec![
            Cbor::UInt(42),
            Cbor::NInt(1_000),
            Cbor::Bytes(b"\x00\xff".to_vec()),
            Cbor::Text("日本".into()),
            Cbor::Arr(vec![Cbor::True, Cbor::False, Cbor::Null]),
            Cbor::Map(vec![(Cbor::Text("k".into()), Cbor::UInt(1))]),
        ]);
        assert_eq!(decode(&encode(&v)), Some(v));
    }
}

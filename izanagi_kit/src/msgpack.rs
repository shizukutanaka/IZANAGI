//! MessagePack codec (msgpack.org spec) — the sibling binary codec of
//! [`crate::cbor`] and [`crate::json`].
//!
//! [`Msg`] covers the deterministic core: nil, booleans, integers (signed or
//! unsigned, `u64`/`i64` range), binary and UTF-8 strings, arrays, and maps —
//! no floats, extension types, or timestamps, so values stay integer-exact.
//! [`encode`] emits the smallest legal encoding (fixed-width forms for small
//! values); unlike CBOR it keeps map insertion order. [`decode`] accepts any
//! well-formed encoding and [`decode_prefix`] reports bytes consumed.
//!
//! ```
//! use izanagi_kit::msgpack::{self, Msg};
//! let v = Msg::Arr(vec![Msg::Int(1), Msg::Str("a".into())]);
//! assert_eq!(msgpack::encode(&v), vec![0x92, 0x01, 0xa1, 0x61]);
//! // Positive ints decode as `Msg::UInt` — the wire keeps no signedness.
//! let back = msgpack::decode(&[0x92, 0x01, 0xa1, 0x61]).unwrap();
//! assert_eq!(back, Msg::Arr(vec![Msg::UInt(1), Msg::Str("a".into())]));
//! ```

/// A MessagePack value (float- and extension-free subset).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Msg {
    /// `nil` (0xc0).
    Nil,
    /// `false` / `true` (0xc2/0xc3).
    Bool(bool),
    /// Signed integer — emits the smallest signed form.
    Int(i64),
    /// Unsigned integer — emits the smallest unsigned form.
    UInt(u64),
    /// Binary data (`bin 8/16/32`).
    Bin(Vec<u8>),
    /// UTF-8 text (`fixstr`/`str 8/16/32`).
    Str(String),
    /// Array (`fixarray`/`array 16/32`).
    Arr(Vec<Msg>),
    /// Key/value pairs — insertion order preserved on encode.
    Map(Vec<(Msg, Msg)>),
}

fn push_u16be(out: &mut Vec<u8>, v: u32) {
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

fn str_head(fix_base: u8, tag8: u8, tag16: u8, tag32: u8, n: usize, out: &mut Vec<u8>) {
    if n < 32 && tag8 == 0xd9 {
        out.push(fix_base | n as u8); // fixstr
    } else if n <= 0xff {
        out.push(tag8);
        out.push(n as u8);
    } else if n <= 0xffff {
        out.push(tag16);
        push_u16be(out, n as u32);
    } else {
        out.push(tag32);
        push_u32be(out, n as u32);
    }
}

fn coll_head(fix_base: u8, tag16: u8, tag32: u8, n: usize, out: &mut Vec<u8>) {
    if n < 16 {
        out.push(fix_base | n as u8);
    } else if n <= 0xffff {
        out.push(tag16);
        push_u16be(out, n as u32);
    } else {
        out.push(tag32);
        push_u32be(out, n as u32);
    }
}

/// Append the smallest-form encoding of `v` to `out`.
pub fn encode_to(v: &Msg, out: &mut Vec<u8>) {
    match v {
        Msg::Nil => out.push(0xc0),
        Msg::Bool(false) => out.push(0xc2),
        Msg::Bool(true) => out.push(0xc3),
        Msg::UInt(n) => {
            let n = *n;
            if n < 128 {
                out.push(n as u8);
            } else if n <= 0xff {
                out.push(0xcc);
                out.push(n as u8);
            } else if n <= 0xffff {
                out.push(0xcd);
                push_u16be(out, n as u32);
            } else if n <= 0xffff_ffff {
                out.push(0xce);
                push_u32be(out, n as u32);
            } else {
                out.push(0xcf);
                push_u64be(out, n);
            }
        }
        Msg::Int(i) => {
            let i = *i;
            if i >= 0 {
                encode_to(&Msg::UInt(i as u64), out);
            } else if i >= -32 {
                out.push(i as i8 as u8); // negative fixint
            } else if i >= i8::MIN as i64 {
                out.push(0xd0);
                out.push(i as i8 as u8);
            } else if i >= i16::MIN as i64 {
                out.push(0xd1);
                push_u16be(out, i as i16 as u16 as u32 & 0xffff);
            } else if i >= i32::MIN as i64 {
                out.push(0xd2);
                push_u32be(out, i as i32 as u32);
            } else {
                out.push(0xd3);
                push_u64be(out, i as u64);
            }
        }
        Msg::Bin(b) => {
            if b.len() <= 0xff {
                out.push(0xc4);
                out.push(b.len() as u8);
            } else if b.len() <= 0xffff {
                out.push(0xc5);
                push_u16be(out, b.len() as u32);
            } else {
                out.push(0xc6);
                push_u32be(out, b.len() as u32);
            }
            out.extend_from_slice(b);
        }
        Msg::Str(s) => {
            str_head(0xa0, 0xd9, 0xda, 0xdb, s.len(), out);
            out.extend_from_slice(s.as_bytes());
        }
        Msg::Arr(a) => {
            coll_head(0x90, 0xdc, 0xdd, a.len(), out);
            for x in a {
                encode_to(x, out);
            }
        }
        Msg::Map(m) => {
            coll_head(0x80, 0xde, 0xdf, m.len(), out);
            for (k, v) in m {
                encode_to(k, out);
                encode_to(v, out);
            }
        }
    }
}

/// Return the encoding of `v` (smallest legal form).
///
/// Canonical-form note: a positive [`Msg::Int`] encodes as an unsigned form,
/// so it decodes as [`Msg::UInt`] — round-trip `decode∘encode` preserves the
/// *number*, not the variant. Use `UInt` in pins where the distinction matters.
pub fn encode(v: &Msg) -> Vec<u8> {
    let mut out = Vec::new();
    encode_to(v, &mut out);
    out
}

fn take<'a>(b: &'a [u8], i: &mut usize, n: usize) -> Option<&'a [u8]> {
    if b.len() - *i < n {
        return None;
    }
    let s = &b[*i..*i + n];
    *i += n;
    Some(s)
}

fn read_int(b: &[u8], i: &mut usize, n: usize) -> Option<u64> {
    let mut v = 0u64;
    for _ in 0..n {
        v = (v << 8) | *take(b, i, 1)?.first()? as u64;
    }
    Some(v)
}

fn item(b: &[u8], i: &mut usize, depth: usize) -> Option<Msg> {
    if depth > 64 {
        return None;
    }
    let tag = *take(b, i, 1)?.first()?;
    match tag {
        0x00..=0x7f => Some(Msg::UInt(tag as u64)),
        0xe0..=0xff => Some(Msg::Int(tag as i8 as i64)),
        0xa0..=0xbf => {
            let n = (tag & 0x1f) as usize;
            core::str::from_utf8(take(b, i, n)?)
                .ok()
                .map(|s| Msg::Str(s.to_string()))
        }
        0x90..=0x9f => {
            let n = (tag & 0x0f) as usize;
            let mut a = Vec::with_capacity(n);
            for _ in 0..n {
                a.push(item(b, i, depth + 1)?);
            }
            Some(Msg::Arr(a))
        }
        0x80..=0x8f => {
            let n = (tag & 0x0f) as usize;
            let mut m = Vec::with_capacity(n);
            for _ in 0..n {
                let k = item(b, i, depth + 1)?;
                let v = item(b, i, depth + 1)?;
                m.push((k, v));
            }
            Some(Msg::Map(m))
        }
        0xc0 => Some(Msg::Nil),
        0xc2 => Some(Msg::Bool(false)),
        0xc3 => Some(Msg::Bool(true)),
        0xc4..=0xc6 => {
            let n = match tag {
                0xc4 => *take(b, i, 1)?.first()? as usize,
                0xc5 => read_int(b, i, 2)? as usize,
                _ => read_int(b, i, 4)? as usize,
            };
            Some(Msg::Bin(take(b, i, n)?.to_vec()))
        }
        0xcc..=0xcf => {
            let n = 1usize << (tag - 0xcc);
            Some(Msg::UInt(read_int(b, i, n)?))
        }
        0xd0..=0xd3 => {
            let n = 1usize << (tag - 0xd0);
            let u = read_int(b, i, n)?;
            let s = (u as i64) << (64 - n * 8) >> (64 - n * 8); // sign-extend
            Some(Msg::Int(s))
        }
        0xd9..=0xdb => {
            let n = match tag {
                0xd9 => *take(b, i, 1)?.first()? as usize,
                0xda => read_int(b, i, 2)? as usize,
                _ => read_int(b, i, 4)? as usize,
            };
            core::str::from_utf8(take(b, i, n)?)
                .ok()
                .map(|s| Msg::Str(s.to_string()))
        }
        0xdc | 0xdd => {
            let n = if tag == 0xdc {
                read_int(b, i, 2)? as usize
            } else {
                read_int(b, i, 4)? as usize
            };
            let mut a = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                a.push(item(b, i, depth + 1)?);
            }
            Some(Msg::Arr(a))
        }
        0xde | 0xdf => {
            let n = if tag == 0xde {
                read_int(b, i, 2)? as usize
            } else {
                read_int(b, i, 4)? as usize
            };
            let mut m = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                let k = item(b, i, depth + 1)?;
                let v = item(b, i, depth + 1)?;
                m.push((k, v));
            }
            Some(Msg::Map(m))
        }
        _ => None, // ext, float, reserved
    }
}

/// Decode one value from the front of `b`, returning `(value, bytes_used)`.
/// `None` on truncation, unsupported tags (floats, ext), or nesting > 64.
pub fn decode_prefix(b: &[u8]) -> Option<(Msg, usize)> {
    let mut i = 0;
    let v = item(b, &mut i, 0)?;
    Some((v, i))
}

/// Decode `b` as exactly one value — trailing bytes are an error.
pub fn decode(b: &[u8]) -> Option<Msg> {
    match decode_prefix(b) {
        Some((v, n)) if n == b.len() => Some(v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(v: Msg, want: &[u8]) {
        assert_eq!(encode(&v), want);
        assert_eq!(decode(want), Some(v));
    }

    #[test]
    fn spec_vectors_scalars() {
        rt(Msg::Nil, &[0xc0]);
        rt(Msg::Bool(false), &[0xc2]);
        rt(Msg::Bool(true), &[0xc3]);
        rt(Msg::UInt(0), &[0x00]);
        rt(Msg::UInt(127), &[0x7f]);
        rt(Msg::UInt(128), &[0xcc, 0x80]);
        rt(Msg::UInt(255), &[0xcc, 0xff]);
        rt(Msg::UInt(256), &[0xcd, 0x01, 0x00]);
        rt(Msg::UInt(65535), &[0xcd, 0xff, 0xff]);
        rt(Msg::UInt(65536), &[0xce, 0x00, 0x01, 0x00, 0x00]);
        rt(Msg::UInt(4_294_967_295), &[0xce, 0xff, 0xff, 0xff, 0xff]);
        rt(
            Msg::UInt(u64::MAX),
            &[0xcf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );
        rt(Msg::Int(-1), &[0xff]);
        rt(Msg::Int(-32), &[0xe0]);
        rt(Msg::Int(-33), &[0xd0, 0xdf]);
        rt(Msg::Int(-128), &[0xd0, 0x80]);
        rt(Msg::Int(-129), &[0xd1, 0xff, 0x7f]);
        rt(Msg::Int(-32768), &[0xd1, 0x80, 0x00]);
        rt(Msg::Int(-32769), &[0xd2, 0xff, 0xff, 0x7f, 0xff]);
        rt(Msg::Int(-2147483648), &[0xd2, 0x80, 0x00, 0x00, 0x00]);
        rt(
            Msg::Int(-2147483649),
            &[0xd3, 0xff, 0xff, 0xff, 0xff, 0x7f, 0xff, 0xff, 0xff],
        );
        rt(
            Msg::Int(i64::MIN),
            &[0xd3, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        );
        // Positive Int encodes unsigned (smallest form) and so decodes as
        // UInt — the canonical-form asymmetry documented on `encode`.
        assert_eq!(encode(&Msg::Int(42)), vec![0x2a]);
        assert_eq!(decode(&[0x2a]), Some(Msg::UInt(42)));
    }

    #[test]
    fn spec_vectors_strings_and_compound() {
        rt(Msg::Str("".into()), &[0xa0]);
        rt(Msg::Str("a".into()), &[0xa1, 0x61]);
        rt(
            Msg::Str("abc".repeat(11)), // 33 chars → str8
            [0xd9, 33]
                .iter()
                .chain("abc".repeat(11).as_bytes())
                .copied()
                .collect::<Vec<u8>>()
                .as_slice(),
        );
        rt(Msg::Bin(vec![1, 2, 3]), &[0xc4, 3, 1, 2, 3]);
        rt(Msg::Arr(vec![]), &[0x90]);
        rt(
            Msg::Arr(vec![Msg::UInt(1), Msg::UInt(2), Msg::UInt(3)]),
            &[0x93, 1, 2, 3],
        );
        rt(Msg::Map(vec![]), &[0x80]);
        rt(
            Msg::Map(vec![(Msg::Str("a".into()), Msg::UInt(1))]),
            &[0x81, 0xa1, 0x61, 0x01],
        );
        rt(
            Msg::Arr(vec![
                Msg::Arr(vec![Msg::UInt(1)]),
                Msg::Arr(vec![Msg::UInt(2)]),
            ]),
            &[0x92, 0x91, 1, 0x91, 2],
        );
    }

    #[test]
    fn decode_accepts_long_forms() {
        assert_eq!(decode(&[0xce, 0, 0, 0, 5]), Some(Msg::UInt(5)));
        assert_eq!(
            decode(&[0xd3, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x9c]),
            Some(Msg::Int(-100))
        );
        assert_eq!(
            decode(&[0xdc, 0, 2, 1, 2]),
            Some(Msg::Arr(vec![Msg::UInt(1), Msg::UInt(2)]))
        );
        assert_eq!(decode(&[0xda, 0, 1, b'x']), Some(Msg::Str("x".into())));
    }

    #[test]
    fn map_preserves_order() {
        let v = Msg::Map(vec![
            (Msg::Str("z".into()), Msg::UInt(1)),
            (Msg::Str("a".into()), Msg::UInt(2)),
        ]);
        let e = encode(&v);
        assert_eq!(decode(&e), Some(v)); // order z,a preserved, not sorted
    }

    #[test]
    fn decode_rejects() {
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[0xc1]), None); // reserved
        assert_eq!(decode(&[0xca, 0, 0, 0, 0]), None); // float32
        assert_eq!(decode(&[0xd4, 1, 0]), None); // fixext1
        assert_eq!(decode(&[0xd9, 5, b'a']), None); // truncated
        assert_eq!(decode(&[0x92, 0x01]), None); // truncated array
    }

    #[test]
    fn decode_prefix_consumed() {
        let mut b = encode(&Msg::Int(-7));
        b.push(0xc0);
        assert_eq!(decode_prefix(&b), Some((Msg::Int(-7), 1)));
        assert_eq!(decode(&b), None);
    }
}

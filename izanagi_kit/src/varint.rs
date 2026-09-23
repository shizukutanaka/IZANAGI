//! LEB128 variable-length integer codec — the `u64`/`i64` wire
//! format used by Protocol Buffers, DWARF, and WASM. Unsigned
//! values go 7 bits per byte, little-endian groups, high bit
//! set on every byte but the last; signed values first map
//! through zigzag so small negatives stay small
//! (`0→0, -1→1, 1→2, -2→3, …`).
//!
//! The decoder is strict: it rejects truncation, values that
//! need more than 10 bytes (the `u64` ceiling), a tenth byte
//! carrying bits outside `u64` range, and non-canonical
//! encodings with a padded zero group. Rejecting padding keeps
//! the encoding a *function* — every value has exactly one
//! byte string, so wire data round-trips byte-identically.
//!
//! ```
//! use izanagi_kit::varint;
//! let buf = varint::encode_u64(300);
//! assert_eq!(buf, vec![0xAC, 0x02]);
//! assert_eq!(varint::decode_u64(&buf), Some((300, 2)));
//! assert_eq!(varint::encode_i64(-1), vec![0x01]);
//! ```
//!
//! References: W3C WASM spec §5.2 (LEB128), Google Protocol
//! Buffers "Encoding" doc (zigzag).

/// Append the LEB128 encoding of `v` to a fresh `Vec` — at most
/// 10 bytes (`⌈64/7⌉` groups, the last holding a single bit).
pub fn encode_u64(mut v: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let mut b = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        out.push(b);
        if v == 0 {
            return out;
        }
    }
}

/// Zigzag-map `v` then LEB128-encode it — signed values with
/// small magnitude stay one byte.
pub fn encode_i64(v: i64) -> Vec<u8> {
    let zz = ((v << 1) ^ (v >> 63)) as u64;
    encode_u64(zz)
}

/// Decode one LEB128 `u64` at `buf[0..]`, returning
/// `(value, bytes_consumed)`. Strictly canonical: `None` on
/// truncation, on a sequence longer than 10 bytes, on a tenth
/// byte with bits above the `u64` range, or on an overlong
/// encoding that pads with zero groups.
pub fn decode_u64(buf: &[u8]) -> Option<(u64, usize)> {
    let mut v: u64 = 0;
    let mut i = 0usize;
    loop {
        let &b = buf.get(i)?;
        let payload = (b & 0x7f) as u64;
        if i == 9 {
            // tenth byte may carry only the low bit without
            // overflowing u64, and must be the terminator —
            // anything else is out of range or non-canonical
            if b & 0x80 != 0 || payload != 1 {
                return None;
            }
            v |= payload << 63;
            return Some((v, i + 1));
        }
        v |= payload << (7 * i);
        i += 1;
        if b & 0x80 == 0 {
            // canonical check: a multi-byte encoding's last
            // group must carry a nonzero payload — a zero here
            // is padding, not data
            if i > 1 && payload == 0 {
                return None;
            }
            return Some((v, i));
        }
    }
}

/// Decode one zigzag+LEB128 `i64` at `buf[0..]` — same strict
/// rules as [`decode_u64`].
pub fn decode_i64(buf: &[u8]) -> Option<(i64, usize)> {
    let (zz, n) = decode_u64(buf)?;
    let v = ((zz >> 1) as i64) ^ -((zz & 1) as i64);
    Some((v, n))
}

/// Decode a sequence of `u64` varints that fills `buf`
/// exactly — `None` on any malformed element or trailing
/// partial varint.
pub fn decode_all(buf: &[u8]) -> Option<Vec<u64>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < buf.len() {
        let (v, n) = decode_u64(&buf[i..])?;
        out.push(v);
        i += n;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn vectors() {
        assert_eq!(encode_u64(0), vec![0x00]);
        assert_eq!(encode_u64(1), vec![0x01]);
        assert_eq!(encode_u64(127), vec![0x7f]);
        assert_eq!(encode_u64(128), vec![0x80, 0x01]);
        assert_eq!(encode_u64(300), vec![0xac, 0x02]);
        assert_eq!(
            encode_u64(u64::MAX),
            vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]
        );
        assert_eq!(decode_u64(&[0xac, 0x02]), Some((300, 2)));
        assert_eq!(decode_u64(&[0x00]), Some((0, 1)));
    }

    #[test]
    fn zigzag_map() {
        // the zigzag bijection, byte for byte
        let cases = [
            (0i64, 0u64),
            (-1, 1),
            (1, 2),
            (-2, 3),
            (2, 4),
            (i64::MAX, u64::MAX - 1),
            (i64::MIN, u64::MAX),
        ];
        for (i, z) in cases {
            assert_eq!(decode_i64(&encode_i64(i)), Some((i, encode_i64(i).len())));
            assert_eq!(decode_u64(&encode_i64(i)).map(|(v, _)| v), Some(z));
        }
    }

    #[test]
    fn malformed_rejected() {
        // truncation
        assert_eq!(decode_u64(&[]), None);
        assert_eq!(decode_u64(&[0x80]), None);
        assert_eq!(decode_u64(&[0x80, 0x80]), None);
        // 11+ bytes
        assert_eq!(decode_u64(&[0x80; 11]), None);
        // tenth byte overflowing u64
        assert_eq!(
            decode_u64(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02]),
            None
        );
        // non-canonical zero-group padding
        assert_eq!(decode_u64(&[0x80, 0x00]), None);
        assert_eq!(decode_u64(&[0xff, 0x00]), None);
        assert_eq!(decode_u64(&[0x81, 0x80, 0x00]), None);
        // canonical zero itself is fine
        assert_eq!(decode_u64(&[0x00]), Some((0, 1)));
    }

    #[test]
    fn round_trip_oracle() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..2000 {
            let kind = rng.below(4);
            let v = match kind {
                0 => rng.below(0x80) as u64,
                1 => rng.below(1 << 21) as u64,
                2 => rng.next_u64() & 0x7fff_ffff_ffff,
                _ => rng.next_u64(),
            };
            let enc = encode_u64(v);
            assert_eq!(decode_u64(&enc), Some((v, enc.len())), "v={v}");
            // i64 same bit pattern through zigzag
            let s = v as i64;
            assert_eq!(decode_i64(&encode_i64(s)), Some((s, encode_i64(s).len())));
        }
        // every boundary: 1/2/9/10-byte edges
        for bits in 0u32..64 {
            let v = if bits == 0 { 0 } else { 1u64 << bits };
            assert_eq!(decode_u64(&encode_u64(v)), Some((v, encode_u64(v).len())));
            if v != 0 {
                assert_eq!(
                    decode_u64(&encode_u64(v - 1)),
                    Some((v - 1, encode_u64(v - 1).len()))
                );
            }
        }
    }

    #[test]
    fn decode_all_stream() {
        let mut rng = SplitMix64::new(8);
        let mut buf = Vec::new();
        let mut want = Vec::new();
        for _ in 0..500 {
            let v = rng.next_u64();
            want.push(v);
            buf.extend(encode_u64(v));
        }
        assert_eq!(decode_all(&buf), Some(want));
        // a trailing partial varint fails the whole stream
        buf.push(0x80);
        assert_eq!(decode_all(&buf), None);
    }
}

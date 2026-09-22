//! Run-length encoding for byte grids and integer streams — the
//! cheapest lossless layer before [`crate::bits`] wire packing, and
//! the canonical compaction for sparse `Dungeon`/`Terrain` slabs in
//! savefiles.
//!
//! Format is byte-oriented and self-delimiting: a run is
//! `(count: u8 (1..=255), value)`. Streams of non-repeating bytes cost
//! one tag each — never use this when runs are rare. No sentinel bytes,
//! no escaping: every output is a pure function of the input, and
//! `decode(encode(x)) == x` for all `x`.
//!
//! ```
//! use izanagi_kit::rle::{encode, decode, encode_u32, decode_u32};
//! let src = b"aaabbbccdddddde".as_slice();
//! let enc = encode(src);
//! assert_eq!(decode(&enc), Some(src.to_vec()));
//! // 255+ runs split automatically.
//! let big = vec![7u8; 300];
//! assert_eq!(decode(&encode(&big)), Some(big));
//! ```

/// Encode `src` as `(count, byte)` pairs. `count` is 1..=255; a run of
/// 256+ emits multiple pairs. Encoded length ≤ `2·src.len()`.
pub fn encode(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len().min(64));
    let mut i = 0usize;
    while i < src.len() {
        let v = src[i];
        let mut j = i + 1;
        while j < src.len() && src[j] == v && j - i < 255 {
            j += 1;
        }
        out.push((j - i) as u8);
        out.push(v);
        i = j;
    }
    out
}

/// Decode `(count, byte)` pairs back to the original bytes.
/// `None` on malformed input (odd length or a zero count — `encode`
/// never produces either, so `None` is an authenticity check, not a
/// panic path).
pub fn decode(enc: &[u8]) -> Option<Vec<u8>> {
    if enc.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(enc.len() * 4);
    for pair in enc.chunks_exact(2) {
        let (n, v) = (pair[0] as usize, pair[1]);
        if n == 0 {
            return None;
        }
        out.resize(out.len() + n, v);
    }
    Some(out)
}

/// Encode a `u32` stream with the same run scheme — output is a compact
/// `(count: u8, value: u32)` vec rather than a flat byte stream (avoids
/// bytes the codec layer would re-pack anyway). Pair with
/// [`crate::bits`]'s varint writer for wire form.
pub fn encode_u32(src: &[u32]) -> Vec<(u8, u32)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < src.len() {
        let v = src[i];
        let mut j = i + 1;
        while j < src.len() && src[j] == v && j - i < 255 {
            j += 1;
        }
        out.push(((j - i) as u8, v));
        i = j;
    }
    out
}

/// Decode `encode_u32` output. `None` on a zero count.
pub fn decode_u32(enc: &[(u8, u32)]) -> Option<Vec<u32>> {
    let mut out = Vec::new();
    for &(n, v) in enc {
        if n == 0 {
            return None;
        }
        out.resize(out.len() + n as usize, v);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn round_trip_on_run_heavy_and_random_streams() {
        let mut rng = SplitMix64::new(0xE1E7);
        for _ in 0..300 {
            // Mixed model: 40% chance of a 1..20 run, else a scatter byte.
            let mut src = Vec::new();
            for _ in 0..(1 + rng.below(60)) {
                if rng.below(5) < 2 {
                    let n = 1 + rng.below(20) as usize;
                    let v = rng.below(4) as u8;
                    src.extend(std::iter::repeat(v).take(n));
                } else {
                    src.push(rng.below(4) as u8);
                }
            }
            let enc = encode(&src);
            assert_eq!(decode(&enc), Some(src.clone()));
            // Length bound.
            assert!(enc.len() <= 2 * src.len());
        }
    }

    #[test]
    fn long_runs_split_at_255() {
        let src = vec![9u8; 300];
        let enc = encode(&src);
        // 255 + 45.
        assert_eq!(enc, vec![255, 9, 45, 9]);
        assert_eq!(decode(&enc), Some(src));
    }

    #[test]
    fn decode_rejects_malformed_input() {
        assert_eq!(decode(&[]), Some(vec![]));
        assert_eq!(decode(&[3]), None); // odd length
        assert_eq!(decode(&[0, b'a']), None); // zero count
        assert_eq!(decode(&[2, b'a', 0, b'b']), None);
    }

    #[test]
    fn u32_layer_round_trips() {
        let mut rng = SplitMix64::new(0x32);
        for _ in 0..200 {
            let src: Vec<u32> = (0..rng.below(200))
                .map(|_| {
                    if rng.below(3) == 0 {
                        7 // heavy run byte
                    } else {
                        rng.below(100)
                    }
                })
                .collect();
            assert_eq!(decode_u32(&encode_u32(&src)), Some(src));
        }
        assert_eq!(decode_u32(&[(0, 5)]), None);
        assert_eq!(decode_u32(&[]), Some(vec![]));
    }

    #[test]
    fn encoding_is_a_pure_function() {
        let src = b"xxxxxyyzz".as_slice();
        assert_eq!(encode(src), encode(src));
        assert_eq!(encode(src), vec![5, b'x', 2, b'y', 2, b'z']);
    }
}

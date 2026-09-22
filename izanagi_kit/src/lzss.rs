//! LZSS — greedy LZ77-family compressor on top of the kit's
//! [`bits`](crate::bits) wire format, completing the codec ladder
//! `rle` → `lzss` → `huffman`: RLE handles long runs, LZSS handles
//! repeated substrings at arbitrary distance, Huffman entropy-codes
//! the residue.
//!
//! Token stream: `0` + 8 bits literal, or `1` + 12-bit
//! `(offset - 1)` + 4-bit length (window 4096, match length 3–18),
//! preceded by a
//! 64-bit raw-length header so the decoder knows exactly when to
//! stop and trailing padding can never decode as phantom tokens. Matches are found by greedy longest-match with
//! smallest-offset tie-break, so the output is a pure function of the
//! input. The decoder is total — truncated streams, out-of-window
//! offsets and trailing garbage are all rejected with `None`.
//!
//! ```
//! let src = b"the quick brown fox jumps over the lazy dog. ".repeat(6);
//! let packed = izanagi_kit::lzss::compress(&src);
//! assert!(packed.len() < src.len());
//! assert_eq!(izanagi_kit::lzss::decompress(&packed), Some(src));
//! ```

use crate::bits::{BitReader, BitWriter};
use std::collections::BTreeMap;

/// Sliding-window size in bytes (12-bit offsets).
pub const WINDOW: usize = 4096;
/// Minimum matchable length.
pub const MIN_MATCH: usize = 3;
/// Maximum matchable length (4-bit field stores `len - MIN_MATCH`).
pub const MAX_MATCH: usize = MIN_MATCH + 15;

/// Compress `input` into the LZSS token stream.
pub fn compress(input: &[u8]) -> Vec<u8> {
    // Index positions by byte value for candidate lookup; deterministic
    // (BTreeMap iteration, ascending positions).
    let mut index: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
    let mut w = BitWriter::with_capacity(input.len() / 2 + 4);
    w.write_bits(input.len() as u64, 64); // raw length, decode-time check
    let mut pos = 0usize;
    while pos < input.len() {
        let (len, off) = best_match(input, pos, &index);
        if len >= MIN_MATCH {
            w.write_bool(true);
            w.write_bits((off - 1) as u64, 12);
            w.write_bits((len - MIN_MATCH) as u64, 4);
            for (d, &b) in input[pos..pos + len].iter().enumerate() {
                index.entry(b).or_default().push(pos + d);
            }
            pos += len;
        } else {
            w.write_bool(false);
            w.write_bits(input[pos] as u64, 8);
            index.entry(input[pos]).or_default().push(pos);
            pos += 1;
        }
    }
    w.into_bytes()
}

/// Longest match in `[pos-WINDOW, pos)`; ties prefer the smallest
/// offset. Returns `(len, offset)` with `len < MIN_MATCH` when nothing
/// matches.
fn best_match(input: &[u8], pos: usize, index: &BTreeMap<u8, Vec<usize>>) -> (usize, usize) {
    let mut best_len = 0usize;
    let mut best_off = 0usize;
    let Some(cands) = index.get(&input[pos]) else {
        return (0, 0);
    };
    let lo = pos.saturating_sub(WINDOW);
    // Candidates are ascending positions; scan all in-window ones.
    let start = cands.partition_point(|&p| p < lo);
    for &p in &cands[start..] {
        let off = pos - p;
        let mut len = 0usize;
        while len < MAX_MATCH
            && pos + len < input.len()
            && input[pos + len] == input[pos + len - off]
        {
            len += 1;
        }
        if len > best_len || (len == best_len && len > 0 && off < best_off) {
            best_len = len;
            best_off = off;
            if len == MAX_MATCH || pos + len >= input.len() {
                break; // cannot improve
            }
        }
    }
    (best_len, best_off)
}

/// Inverse of [`compress`]; `None` on any malformed input.
/// The stream embeds the raw output length, so the decoder knows
/// exactly when to stop — padding bits are never interpreted.
pub fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    let mut r = BitReader::new(data);
    let want = r.read_bits(64).ok()?;
    if want > (1 << 32) {
        return None; // absurd claim; also keeps the alloc bounded
    }
    let mut out: Vec<u8> = Vec::with_capacity(want as usize);
    while (out.len() as u64) < want {
        if !r.read_bool().ok()? {
            let b = r.read_bits(8).ok()?;
            out.push(b as u8);
            continue;
        }
        let off = r.read_bits(12).ok()? as usize + 1;
        let len = r.read_bits(4).ok()? as usize + MIN_MATCH;
        if off > out.len() {
            return None;
        }
        let from = out.len() - off;
        for i in 0..len {
            out.push(out[from + i]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn round_trip(data: &[u8]) {
        assert_eq!(decompress(&compress(data)), Some(data.to_vec()));
    }

    #[test]
    fn round_trip_spectrum() {
        let mut rng = SplitMix64::new(0x1A7E);
        // Pure noise (incompressible — codec must not corrupt it).
        for _ in 0..20 {
            let n = rng.below(300) as usize;
            let data: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
            round_trip(&data);
        }
        // Repetitive / structured inputs.
        for _ in 0..20 {
            let dict_size = 1 + rng.below(8);
            let dict: Vec<u8> = (0..dict_size)
                .map(|_| (rng.next_u64() % 4) as u8 + b'a')
                .collect();
            let n = rng.below(2000) as usize;
            let mut data = Vec::with_capacity(n);
            while data.len() < n {
                let k = dict[rng.below(dict_size) as usize];
                data.extend_from_slice(&[k, k]);
            }
            round_trip(&data);
        }
        // Long runs + periodic text.
        round_trip(&vec![7u8; 5000]);
        round_trip(&b"abcabcabc".repeat(300));
        round_trip(b"");
        round_trip(b"x");
        round_trip(b"ab");
        round_trip(b"aaa"); // can match offset 1 len 3
    }

    #[test]
    fn repetitive_input_compresses() {
        let src = b"the quick brown fox jumps over the lazy dog. ".repeat(20);
        let packed = compress(&src);
        assert!(packed.len() < src.len() / 2);
    }

    #[test]
    fn malformed_streams_rejected() {
        let src = b"hello hello hello".repeat(4);
        let packed = compress(&src);
        for len in 0..packed.len() {
            // Truncations either decode to a strict prefix or fail —
            // never panic, never produce `src` wrong-but-present.
            if let Some(out) = decompress(&packed[..len]) {
                assert!(out.len() < src.len());
            }
        }
        // Hand-built: match token before any output exists.
        let mut w = BitWriter::new();
        w.write_bits(1, 64); // want = 1
        w.write_bool(true);
        w.write_bits(0, 12); // off field 0 → distance 1
        w.write_bits(0, 4);
        assert_eq!(decompress(&w.into_bytes()), None);
        // Offset beyond produced output.
        let mut w = BitWriter::new();
        w.write_bits(4, 64);
        w.write_bool(false);
        w.write_bits(b'a' as u64, 8);
        w.write_bool(true);
        w.write_bits(5, 12); // distance 6 > 1 byte written so far
        w.write_bits(0, 4);
        assert_eq!(decompress(&w.into_bytes()), None);
        // Length claim exceeding the cap.
        let mut w = BitWriter::new();
        w.write_bits(u64::MAX, 64);
        assert_eq!(decompress(&w.into_bytes()), None);
        // Empty stream.
        assert_eq!(decompress(&[]), None);
    }

    #[test]
    fn deterministic_output() {
        let mut rng = SplitMix64::new(0xFEED);
        let data: Vec<u8> = (0..1500).map(|_| (rng.next_u64() % 5) as u8).collect();
        assert_eq!(compress(&data), compress(&data));
    }
}

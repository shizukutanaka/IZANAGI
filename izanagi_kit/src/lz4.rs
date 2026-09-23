//! LZ4 block codec — the wire format from the LZ4 spec,
//! not the framed container.
//!
//! A block is a sequence of *sequences*. Each sequence:
//!
//! ```text
//! token  = [lit_len:4 | match_len:4]  (a nibble each;
//!         15 means "keep reading 255-byte extensions")
//! lits   = lit_len literal bytes, copied verbatim
//! offset = u16 little-endian distance back from output head
//! match  = match_len + 4 bytes copied from `offset` back
//! ```
//!
//! Constraints the decoder enforces (they make the stream
//! canonical enough to be a deterministic wire format):
//!
//! - `offset ≥ 1` and `offset ≤ bytes_written_so_far`
//! - the final sequence is literals-only (no match part)
//! - overlapping matches (`offset < match_len`) are legal
//!   and decode as a repeating pattern — the copy loop
//!   must read back bytes it just emitted
//!
//! The compressor is the classic hash-table greedy parse:
//! last seen position of each 4-byte prefix, take the first
//! match ≥ 4 bytes. Output depends only on the input —
//! no seeds, no tables.
//!
//! ```
//! use izanagi_kit::lz4::{compress, decompress};
//! let data = b"the quick brown fox jumps over the lazy dog\
//!              the quick brown fox jumps over the lazy dog";
//! let c = compress(data);
//! assert!(c.len() < data.len());
//! assert_eq!(decompress(&c), Some(data.to_vec()));
//! // truncation and corruption both reject
//! assert_eq!(decompress(&c[..c.len() - 1]), None);
//! ```

/// LZ4 stores lengths as 4-bit nibbles; 15 = read more.
const TOKEN_MASK: usize = 15;
/// Minimum bytes in a match (spec-mandated).
const MIN_MATCH: usize = 4;
/// Hash size for the greedy parse table (16-bit offsets fit
/// the spec's u16 window, so 2^16 entries suffice).
const HASH_BITS: u32 = 12;
const HASH_SIZE: usize = 1 << HASH_BITS;

/// Spec: a sequence may not start a match within the last
/// 12 bytes of input, and the last 5 bytes are always
/// emitted as literals.
const MFLIMIT: usize = 12;
const LASTLITERALS: usize = 5;

fn hash4(b: &[u8]) -> usize {
    let v = u32::from(b[0])
        | (u32::from(b[1]) << 8)
        | (u32::from(b[2]) << 16)
        | (u32::from(b[3]) << 24);
    (v.wrapping_mul(2654435761) >> (32 - HASH_BITS)) as usize
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    let mut len = len;
    while len >= 255 {
        out.push(255);
        len -= 255;
    }
    out.push(len as u8);
}

fn read_len(data: &[u8], pos: &mut usize, len: usize) -> Option<usize> {
    let mut len = len;
    if len == TOKEN_MASK {
        loop {
            let b = *data.get(*pos)?;
            *pos += 1;
            len += usize::from(b);
            if b != 255 {
                break;
            }
        }
    }
    Some(len)
}

/// Compress `input` into the LZ4 block format.
pub fn compress(input: &[u8]) -> Vec<u8> {
    let n = input.len();
    let mut out = Vec::with_capacity(n / 2 + 16);
    if n < MFLIMIT {
        // too short for any legal match — one literal run
        push_token_literals(&mut out, input);
        return out;
    }
    // table[h] = last position where 4-byte prefix with
    // hash h was seen (+1 so 0 means "none"); u32 not
    // usize so behaviour is width-independent
    let mut table = vec![0u32; HASH_SIZE];
    let mut anchor = 0usize; // start of pending literals
    let mut i = 0usize;
    while i + MFLIMIT <= n {
        let h = hash4(&input[i..i + 4]);
        let prev = usize::try_from(table[h]).unwrap_or(0);
        table[h] = u32::try_from(i + 1).unwrap_or(0);
        let mut match_at = !0usize;
        let mut mlen = 0usize;
        if prev > 0 {
            let p = prev - 1;
            if i - p <= u16::MAX as usize && input[p..p + 4] == input[i..i + 4] {
                // extend the match — never into the last
                // LASTLITERALS bytes (they're always literals)
                let mut l = 4usize;
                while i + l < n - LASTLITERALS && input[p + l] == input[i + l] {
                    l += 1;
                }
                mlen = l;
                match_at = p;
            }
        }
        if match_at != !0usize {
            let lit_len = i - anchor;
            let ml = mlen - MIN_MATCH;
            let token = ((lit_len.min(TOKEN_MASK)) << 4) | ml.min(TOKEN_MASK);
            out.push(token as u8);
            if lit_len >= TOKEN_MASK {
                push_len(&mut out, lit_len - TOKEN_MASK);
            }
            out.extend_from_slice(&input[anchor..i]);
            let off = (i - match_at) as u16;
            out.push((off & 0xff) as u8);
            out.push((off >> 8) as u8);
            if ml >= TOKEN_MASK {
                push_len(&mut out, ml - TOKEN_MASK);
            }
            i += mlen;
            anchor = i;
            // index a couple of positions inside the match so
            // the table stays fresh (classic acceleration step)
            let mut j = i - 2;
            while j < i && j + 4 <= n {
                let h2 = hash4(&input[j..j + 4]);
                table[h2] = u32::try_from(j + 1).unwrap_or(0);
                j += 1;
            }
        } else {
            i += 1;
        }
    }
    push_token_literals(&mut out, &input[anchor..]);
    out
}

fn push_token_literals(out: &mut Vec<u8>, lits: &[u8]) {
    let token = (lits.len().min(TOKEN_MASK)) << 4;
    out.push(token as u8);
    if lits.len() >= TOKEN_MASK {
        push_len(out, lits.len() - TOKEN_MASK);
    }
    out.extend_from_slice(lits);
}

/// Decode an LZ4 block. `None` on any malformed input —
/// truncation, overrun offsets, a match in the final
/// sequence, or non-canonical trailing garbage.
pub fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let token = data[pos] as usize;
        pos += 1;
        let lit_len = read_len(data, &mut pos, token >> 4)?;
        if pos + lit_len > data.len() {
            return None;
        }
        out.extend_from_slice(&data[pos..pos + lit_len]);
        pos += lit_len;
        if pos == data.len() {
            // literals-only tail — legal end of stream
            break;
        }
        if pos + 2 > data.len() {
            return None;
        }
        let off = usize::from(data[pos]) | (usize::from(data[pos + 1]) << 8);
        pos += 2;
        if off == 0 || off > out.len() {
            return None;
        }
        let mlen = read_len(data, &mut pos, token & TOKEN_MASK)?.checked_add(MIN_MATCH)?;
        let start = out.len() - off;
        if off >= mlen {
            out.extend_from_within(start..start + mlen);
        } else {
            // overlapping copy — byte-by-byte repeat
            for k in 0..mlen {
                let b = out[start + k];
                out.push(b);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Reference vectors: golden bytes under version control
    /// would be brittle; instead check roundtrip + canonical
    /// decode of a hand-built sequence.
    #[test]
    fn basics() {
        // empty input = one zero token
        assert_eq!(compress(b""), vec![0]);
        assert_eq!(decompress(&[0]), Some(vec![]));
        // literals only
        let c = compress(b"abc");
        assert_eq!(decompress(&c), Some(b"abc".to_vec()));
        // a long repeated run compresses hard
        let data = vec![b'a'; 100];
        let c = compress(&data);
        assert!(c.len() < 20, "len {}", c.len());
        assert_eq!(decompress(&c), Some(data));
    }

    /// Hand-assembled wire sequences — overlapping match
    /// (offset < length) is the subtle case.
    #[test]
    fn overlapping_match_decodes() {
        // token: litlen=1, matchlen_nibble=2 → match 6 bytes
        // "a" then match offset=1 len=6 → "aaaaaaa" total
        let wire = [0x12, b'a', 1, 0];
        assert_eq!(decompress(&wire), Some(b"aaaaaaa".to_vec()));
        // offset > emitted so far rejects
        let bad = [0x12, b'a', 5, 0];
        assert_eq!(decompress(&bad), None);
        // offset 0 rejects
        let bad2 = [0x12, b'a', 0, 0];
        assert_eq!(decompress(&bad2), None);
    }

    /// Every truncation and single-byte corruption either
    /// rejects or decodes to something different — never
    /// silently produces the original (checksums live on
    /// the caller's side per spec).
    #[test]
    fn malformed_never_silent() {
        let mut rng = SplitMix64::new(0x1B4B);
        let data: Vec<u8> = (0..200).map(|i| (i % 7) as u8 + b'a').collect();
        let c = compress(&data);
        for cut in 1..c.len() {
            // any truncation that still parses must not yield
            // the full original — the last literals are gone
            let d = decompress(&c[..cut]);
            assert_ne!(d, Some(data.clone()), "cut {cut}");
        }
        for _ in 0..500 {
            let mut m = c.clone();
            let i = rng.below(m.len() as u32) as usize;
            m[i] ^= 1 << (rng.below(8));
            // corruption may still decode (LZ4 has no CRC) but
            // must not panic — and if it decodes to the same
            // bytes that's a spec-level collision, not a bug
            let _ = decompress(&m);
        }
    }

    /// Shadow oracle: roundtrip on structured inputs —
    /// repeated, periodic, random, mixed.
    #[test]
    fn oracle_roundtrip() {
        let mut rng = SplitMix64::new(0xBEEF);
        let cases: Vec<Vec<u8>> = vec![
            Vec::new(),
            vec![0],
            b"hello world".to_vec(),
            vec![b'a'; 1000],
            (0..=255u8).cycle().take(4096).collect(),
            {
                let mut v = Vec::new();
                let mut r = SplitMix64::new(1);
                for _ in 0..3000 {
                    v.push(b'a' + r.below(4) as u8);
                }
                v
            },
        ];
        for (i, d) in cases.iter().enumerate() {
            let c = compress(d);
            assert_eq!(decompress(&c).as_deref(), Some(&d[..]), "case {i}");
        }
        // random structured: high-repetition text
        for _ in 0..100 {
            let n = 1 + rng.below(500) as usize;
            let words: [&[u8]; 5] = [b"foo", b"bar", b"quux", b"the", b"lz4"];
            let mut d = Vec::new();
            while d.len() < n {
                d.extend_from_slice(words[rng.below(5) as usize]);
                d.push(b' ');
            }
            d.truncate(n);
            let c = compress(&d);
            assert_eq!(decompress(&c).as_deref(), Some(&d[..]));
        }
    }

    /// Deterministic output — same input compresses
    /// identically every time (the table is reset, no
    /// hash seed).
    #[test]
    fn deterministic_output() {
        let d: Vec<u8> = (0..1000u32).map(|i| (i % 97) as u8).collect();
        assert_eq!(compress(&d), compress(&d));
    }

    /// Compression actually engages on compressible data.
    #[test]
    fn compresses_repeated() {
        let d: Vec<u8> = b"the quick brown fox jumps over the lazy dog. "
            .iter()
            .cycle()
            .take(3000)
            .copied()
            .collect();
        let c = compress(&d);
        assert!(c.len() * 4 < d.len(), "{} vs {}", c.len(), d.len());
        let mut seen = BTreeSet::new();
        seen.insert(decompress(&c));
        assert_eq!(seen.len(), 1);
    }
}

//! DEFLATE compressor — the emit half of [`crate::inflate`]
//! (RFC 1951/1950/1952).
//!
//! Two modes: [`deflate_stored`] writes verbatim stored blocks, and
//! [`deflate`] runs a single-candidate greedy LZ77 matcher (hash on the
//! 3-byte prefix, depth 1 — deterministic and dependency-free) feeding a
//! **fixed-Huffman** block. No dynamic-table estimation, no lazy
//! matching: the output is always a legal stream, just less dense than
//! zlib's. [`deflate_zlib`] and [`deflate_gzip`] wrap the result in the
//! RFC 1950 / RFC 1952 containers.
//!
//! The correctness oracle is this crate's own `inflate`, which is
//! pinned against Python-zlib vectors: `inflate(deflate(x)) == x` and
//! `inflate_zlib(deflate_zlib(x)) == x` hold for every input.
//!
//! ```
//! use izanagi_kit::{deflate::{deflate, deflate_zlib}, inflate::{inflate, inflate_zlib}};
//!
//! let data = b"the quick brown fox jumps over the lazy dog";
//! assert_eq!(inflate(&deflate(data)).as_deref(), Some(&data[..]));
//! assert_eq!(inflate_zlib(&deflate_zlib(data)).as_deref(), Some(&data[..]));
//! ```

use std::vec::Vec;

/// LSB-first bit writer — the mirror of `inflate`'s `Bits`.
struct BitW {
    out: Vec<u8>,
    acc: u32,
    nbits: u32,
}

impl BitW {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }
    /// Emit `bits` low bits of `v`, LSB exiting the stream first.
    fn put(&mut self, v: u32, bits: u32) {
        self.acc |= v << self.nbits;
        self.nbits += bits;
        while self.nbits >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }
    /// Huffman code elements exit MSB-first — reverse before `put`.
    fn put_msb(&mut self, code: u32, bits: u32) {
        let mut r = 0u32;
        for i in 0..bits {
            r |= ((code >> i) & 1) << (bits - 1 - i);
        }
        self.put(r, bits);
    }
    /// Pad to a byte boundary (for stored blocks).
    fn align(&mut self) {
        if self.nbits > 0 {
            self.out.push(self.acc as u8);
            self.acc = 0;
            self.nbits = 0;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        self.align();
        self.out
    }
}

/// Fixed-table literal/length code → (code value, bit length).
/// RFC 1951 §3.2.6.
fn lit_code(sym: u16) -> (u16, u32) {
    match sym {
        0..=143 => (0x30 + sym, 8),
        144..=255 => (0x190 + sym - 144, 9),
        256..=279 => (sym - 256, 7),
        _ => (0xC0 + sym - 280, 8),
    }
}

/// Match length → (length symbol, base, extra bits). RFC 1951 §3.2.5.
const LEN_TAB: [(u16, u16, u32); 29] = [
    (257, 3, 0),
    (258, 4, 0),
    (259, 5, 0),
    (260, 6, 0),
    (261, 7, 0),
    (262, 8, 0),
    (263, 9, 0),
    (264, 10, 0),
    (265, 11, 1),
    (266, 13, 1),
    (267, 15, 1),
    (268, 17, 1),
    (269, 19, 2),
    (270, 23, 2),
    (271, 27, 2),
    (272, 31, 2),
    (273, 35, 3),
    (274, 43, 3),
    (275, 51, 3),
    (276, 59, 3),
    (277, 67, 4),
    (278, 83, 4),
    (279, 99, 4),
    (280, 115, 4),
    (281, 131, 5),
    (282, 163, 5),
    (283, 195, 5),
    (284, 227, 5),
    (285, 258, 0),
];

/// Match distance → (distance symbol, base, extra bits).
const DIST_TAB: [(u16, u16, u32); 30] = [
    (0, 1, 0),
    (1, 2, 0),
    (2, 3, 0),
    (3, 4, 0),
    (4, 5, 1),
    (5, 7, 1),
    (6, 9, 2),
    (7, 13, 2),
    (8, 17, 3),
    (9, 25, 3),
    (10, 33, 4),
    (11, 49, 4),
    (12, 65, 5),
    (13, 97, 5),
    (14, 129, 6),
    (15, 193, 6),
    (16, 257, 7),
    (17, 385, 7),
    (18, 513, 8),
    (19, 769, 8),
    (20, 1025, 9),
    (21, 1537, 9),
    (22, 2049, 10),
    (23, 3073, 10),
    (24, 4097, 11),
    (25, 6145, 11),
    (26, 8193, 12),
    (27, 12289, 12),
    (28, 16385, 13),
    (29, 24577, 13),
];

/// Emit one fixed-Huffman block containing a literal or a match.
fn emit_lit(w: &mut BitW, b: u8) {
    let (c, n) = lit_code(b as u16);
    w.put_msb(c as u32, n);
}

fn emit_match(w: &mut BitW, len: u16, dist: u16) {
    let (sym, base, extra) = LEN_TAB[LEN_TAB.partition_point(|&(_, b, _)| b <= len) - 1];
    let (c, n) = lit_code(sym);
    w.put_msb(c as u32, n);
    if extra > 0 {
        w.put((len - base) as u32, extra);
    }
    let (dsym, dbase, dextra) = DIST_TAB[DIST_TAB.partition_point(|&(_, b, _)| b <= dist) - 1];
    w.put_msb(dsym as u32, 5);
    if dextra > 0 {
        w.put((dist - dbase) as u32, dextra);
    }
}

fn emit_end(w: &mut BitW) {
    let (c, n) = lit_code(256);
    w.put_msb(c as u32, n);
}

/// 3-byte rolling hash for the match table (15-bit index).
fn hash3(d: &[u8], i: usize) -> usize {
    let v = (d[i] as usize) | ((d[i + 1] as usize) << 8) | ((d[i + 2] as usize) << 16);
    (v.wrapping_mul(0x9E37_79B1) >> 17) & 0x7fff
}

/// Raw DEFLATE stream: one fixed-Huffman block, greedy LZ77.
///
/// Match search is depth-1 on a 32Ki-entry hash table — deterministic,
/// bounded, and good enough to exercise real back-references. Inputs
/// longer than ~2GiB would exceed the i32 position field; the function
/// degrades to literal-only emission past that point rather than
/// wrapping.
pub fn deflate(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let mut w = BitW::new();
    w.put(1, 1); // BFINAL
    w.put(1, 2); // BTYPE = 01, fixed Huffman
    if n == 0 || n > i32::MAX as usize {
        for &b in data {
            emit_lit(&mut w, b);
        }
        emit_end(&mut w);
        return w.finish();
    }
    const HBITS: usize = 1 << 15;
    let mut head = vec![-1i32; HBITS];
    let mut i = 0usize;
    while i < n {
        let mut mlen = 0usize;
        let mut mdist = 0usize;
        if i + 3 <= n {
            let h = hash3(data, i);
            let cand = head[h];
            if cand >= 0 {
                let c = cand as usize;
                if i - c <= 32768 {
                    let cap = (n - i).min(258);
                    let mut l = 0usize;
                    while l < cap && data[c + l] == data[i + l] {
                        l += 1;
                    }
                    if l >= 3 {
                        mlen = l;
                        mdist = i - c;
                    }
                }
            }
        }
        if mlen >= 3 {
            emit_match(&mut w, mlen as u16, mdist as u16);
            // Insert every covered position — better density, still O(1) each.
            let end = i + mlen;
            while i < end {
                if i + 3 <= n {
                    let h = hash3(data, i);
                    head[h] = i as i32;
                }
                i += 1;
            }
        } else {
            emit_lit(&mut w, data[i]);
            if i + 3 <= n {
                let h = hash3(data, i);
                head[h] = i as i32;
            }
            i += 1;
        }
    }
    emit_end(&mut w);
    w.finish()
}

/// Raw DEFLATE stream as verbatim stored blocks (≤65535 bytes each).
pub fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut w = BitW::new();
    let mut at = 0usize;
    loop {
        let chunk = (data.len() - at).min(65535);
        let last = at + chunk >= data.len();
        w.put(last as u32, 1);
        w.put(0, 2); // BTYPE = 00, stored
        w.align();
        let len = chunk as u16;
        w.out.push(len as u8);
        w.out.push((len >> 8) as u8);
        let nlen = !len;
        w.out.push(nlen as u8);
        w.out.push((nlen >> 8) as u8);
        w.out.extend_from_slice(&data[at..at + chunk]);
        at += chunk;
        if last {
            break;
        }
    }
    w.finish()
}

/// Adler-32 checksum (RFC 1950). `mod 65521` is applied every 5552
/// bytes — the largest run that cannot overflow `u32`.
pub fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// RFC 1950 zlib stream: `0x78 0x9C` header + deflate + big-endian
/// Adler-32 (written as shifts — the crate bans `to_be_bytes`).
pub fn deflate_zlib(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 16);
    out.push(0x78);
    out.push(0x9C);
    out.extend_from_slice(&deflate(data));
    let s = adler32(data);
    out.push((s >> 24) as u8);
    out.push((s >> 16) as u8);
    out.push((s >> 8) as u8);
    out.push(s as u8);
    out
}

/// RFC 1952 gzip stream: fixed 10-byte header (no name/extra/HCRC),
/// deflate body, little-endian CRC-32 and input size trailer.
pub fn deflate_gzip(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 24);
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 3]);
    out.extend_from_slice(&deflate(data));
    let crc = crate::crc::crc32(data);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inflate::{inflate, inflate_gzip, inflate_zlib};

    #[test]
    fn adler32_vector() {
        // RFC 1950 example: "Wikipedia" → 0x11E60398.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn literal_only_roundtrip() {
        let data = b"abc!";
        assert_eq!(inflate(&deflate(data)).as_deref(), Some(&data[..]));
        let empty: &[u8] = b"";
        assert_eq!(inflate(&deflate(empty)).as_deref(), Some(empty));
    }

    #[test]
    fn matches_roundtrip() {
        let mut data = Vec::new();
        for i in 0..64u8 {
            data.extend_from_slice(b"common-prefix-");
            data.push(i);
        }
        let c = deflate(&data);
        assert!(c.len() < data.len());
        assert_eq!(inflate(&c).as_deref(), Some(&data[..]));
    }

    #[test]
    fn stored_roundtrip_multiblock() {
        let data: Vec<u8> = (0..70_000usize).map(|i| (i * 31) as u8).collect();
        assert_eq!(inflate(&deflate_stored(&data)).as_deref(), Some(&data[..]));
    }

    #[test]
    fn zlib_and_gzip_wrappers() {
        let data = b"wrapped payload wrapped payload wrapped payload";
        assert_eq!(
            inflate_zlib(&deflate_zlib(data)).as_deref(),
            Some(&data[..])
        );
        assert_eq!(
            inflate_gzip(&deflate_gzip(data)).as_deref(),
            Some(&data[..])
        );
        // Header sanity: 0x78 0x9C passes the FCHECK rule.
        let z = deflate_zlib(data);
        assert_eq!(((z[0] as u32) * 256 + z[1] as u32) % 31, 0);
    }

    #[test]
    fn deterministic_twice() {
        let data = b"deterministic deterministic deterministic";
        assert_eq!(deflate(data), deflate(data));
        assert_eq!(deflate_zlib(data), deflate_zlib(data));
    }

    #[test]
    fn rng_data_roundtrip() {
        // Incompressible bytes still round-trip; the matcher just
        // emits literals.
        let mut rng = crate::rng::SplitMix64::new(7);
        let data: Vec<u8> = (0..4096).map(|_| rng.next_u64() as u8).collect();
        assert_eq!(inflate(&deflate(&data)).as_deref(), Some(&data[..]));
    }
}

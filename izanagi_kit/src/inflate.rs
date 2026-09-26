//! DEFLATE decoder — RFC 1951 raw streams plus RFC 1950 zlib and
//! RFC 1952 gzip wrappers. The last big codec gap beside `huffman`/
//! `lzss`.
//!
//! Bit order is the DEFLATE quirk: bits leave each byte LSB-first, but
//! Huffman code *elements* are read MSB-first — this reader yields bits
//! LSB-first and the decoder packs them MSB-into the code.
//!
//! All malformed input degrades to `None` — no panics, no I/O.
//!
//! ```
//! use izanagi_kit::inflate::inflate;
//!
//! // zlib at level 9 on b"ABC", deflate payload only.
//! let deflated = [0x73, 0x74, 0x72, 0x06, 0x00];
//! assert_eq!(inflate(&deflated), Some(b"ABC".to_vec()));
//! ```

use std::vec::Vec;

/// LSB-first bit cursor over a byte slice.
struct Bits<'a> {
    data: &'a [u8],
    /// Absolute bit position.
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Bits { data, pos: 0 }
    }
    /// Reads `n` bits LSB-first; `None` past the end.
    fn take(&mut self, n: usize) -> Option<u32> {
        let mut v = 0u32;
        for i in 0..n {
            let byte = *self.data.get(self.pos / 8)?;
            let bit = (byte >> (self.pos % 8)) & 1;
            v |= (bit as u32) << i;
            self.pos += 1;
        }
        Some(v)
    }
    /// Aligns to the next byte boundary.
    fn align(&mut self) {
        self.pos = (self.pos + 7) & !7;
    }
}

/// Canonical Huffman decoding table: `code[symbol]` values assigned in
/// (length, symbol) order per RFC 1951 §3.2.2.
struct Huff {
    /// Symbols sorted by (length, symbol).
    syms: Vec<u16>,
    /// count[l] = number of symbols with code length l (1..=15).
    count: [u16; 16],
    /// first[l] = first canonical code of length l.
    first: [u16; 16],
    /// base[l] = index into `syms` of the first length-l symbol.
    base: [u16; 16],
}

impl Huff {
    /// Builds a table from code lengths (0 = absent). `None` on
    /// over-subscribed or otherwise invalid length sets.
    fn new(lengths: &[u8]) -> Option<Self> {
        let mut count = [0u16; 16];
        for &l in lengths {
            if l > 15 {
                return None;
            }
            count[l as usize] += 1;
        }
        if count[0] as usize == lengths.len() {
            return None; // empty alphabet
        }
        // Kraft check: Σ 2^{15−l} must not exceed 2^15.
        let mut used = 0u32;
        for (l, &c) in count.iter().enumerate().skip(1) {
            used += (c as u32) << (15 - l);
        }
        if used > (1 << 15) {
            return None;
        }
        // First canonical code per length, and the symbol table.
        // RFC 1951 §3.2.2: bl_count[0] is 0 — absent symbols do not
        // participate in the code-space walk.
        let mut first = [0u16; 16];
        let mut base = [0u16; 16];
        let mut code = 0u32;
        let mut at = 0usize;
        let mut prev = 0u32;
        for (l, &c) in count.iter().enumerate().take(16).skip(1) {
            code = (code + prev) << 1;
            first[l] = code as u16;
            base[l] = at as u16;
            at += c as usize;
            prev = c as u32;
        }
        let mut syms = Vec::with_capacity(at);
        for l in 1..16u8 {
            for (s, &sl) in lengths.iter().enumerate() {
                if sl == l {
                    syms.push(s as u16);
                }
            }
        }
        Some(Huff {
            syms,
            count,
            first,
            base,
        })
    }

    /// Decodes one symbol; `None` on underrun or unassigned code.
    /// Huffman elements are MSB-of-code-first inside the LSB-first
    /// stream, so the accumulator shifts left as bits arrive.
    fn read(&self, b: &mut Bits) -> Option<u16> {
        let mut code = 0u32;
        for l in 1..16usize {
            code = (code << 1) | b.take(1)?;
            let n = self.count[l] as u32;
            let first = self.first[l] as u32;
            if code >= first && code - first < n {
                return self
                    .syms
                    .get(self.base[l] as usize + (code as usize - self.first[l] as usize))
                    .copied();
            }
        }
        None
    }
}

/// Fixed literal/length and distance tables per RFC 1951 §3.2.6
/// (always well-formed — the `None` arm is unreachable).
fn fixed_tables() -> Option<(Huff, Huff)> {
    let mut lit = [0u8; 288];
    for (i, l) in lit.iter_mut().enumerate() {
        *l = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    let dist = [5u8; 30];
    match (Huff::new(&lit), Huff::new(&dist)) {
        (Some(l), Some(d)) => Some((l, d)),
        _ => None,
    }
}

/// Length code 257..285 → (base, extra bits).
const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// Distance code 0..29 → (base, extra bits).
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
/// Order in which the dynamic header's code-length lengths appear.
const CLEN_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Decodes a compressed block's payload with the given tables.
fn decode_block(b: &mut Bits, lit: &Huff, dist: &Huff, out: &mut Vec<u8>) -> Option<()> {
    loop {
        let sym = lit.read(b)?;
        match sym {
            0..=255 => out.push(sym as u8),
            256 => return Some(()),
            257..=285 => {
                let li = (sym - 257) as usize;
                let len = LEN_BASE[li] as usize + b.take(LEN_EXTRA[li] as usize)? as usize;
                let dsym = dist.read(b)? as usize;
                if dsym >= 30 {
                    return None;
                }
                let d = DIST_BASE[dsym] as usize + b.take(DIST_EXTRA[dsym] as usize)? as usize;
                if d > out.len() {
                    return None;
                }
                for _ in 0..len {
                    let v = out[out.len() - d];
                    out.push(v);
                }
            }
            _ => return None,
        }
    }
}

/// Reads the dynamic Huffman header (RFC 1951 §3.2.7) and builds the
/// literal/length + distance tables.
fn dynamic_tables(b: &mut Bits) -> Option<(Huff, Huff)> {
    let hlit = b.take(5)? as usize + 257;
    let hdist = b.take(5)? as usize + 1;
    let hclen = b.take(4)? as usize + 4;
    if hlit > 286 || hdist > 30 {
        return None;
    }
    let mut clen_lens = [0u8; 19];
    for &slot in CLEN_ORDER.iter().take(hclen) {
        clen_lens[slot] = b.take(3)? as u8;
    }
    let clen_huff = Huff::new(&clen_lens)?;
    // Literal+dist lengths form one interleaved stream.
    let total = hlit + hdist;
    let mut lens = Vec::with_capacity(total);
    while lens.len() < total {
        let sym = clen_huff.read(b)?;
        match sym {
            0..=15 => lens.push(sym as u8),
            16 => {
                let prev = *lens.last()?;
                let reps = 3 + b.take(2)? as usize;
                for _ in 0..reps {
                    if lens.len() >= total {
                        return None;
                    }
                    lens.push(prev);
                }
            }
            17 => {
                let reps = 3 + b.take(3)? as usize;
                lens.extend(std::iter::repeat(0u8).take(reps));
            }
            _ => {
                let reps = 11 + b.take(7)? as usize;
                lens.extend(std::iter::repeat(0u8).take(reps));
            }
        }
    }
    if lens.len() > total {
        return None;
    }
    Some((Huff::new(&lens[..hlit])?, Huff::new(&lens[hlit..])?))
}

/// Inflates a raw RFC 1951 DEFLATE stream. `None` on malformed input.
pub fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    Some(inflate_count(data)?.0)
}

/// Like [`inflate`], but also reports how many input bytes the
/// deflate stream consumed — for containers that place a compressed
/// member next to other data (a git packfile's per-object zlib
/// streams, for instance).
pub fn inflate_count(data: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut b = Bits::new(data);
    let mut out = Vec::new();
    loop {
        let final_ = b.take(1)? == 1;
        match b.take(2)? {
            0 => {
                // Stored: align to byte, LEN + one's-complement NLEN.
                b.align();
                let len = b.take(16)? as usize;
                let nlen = b.take(16)? as usize;
                if len ^ nlen != 0xFFFF {
                    return None;
                }
                for _ in 0..len {
                    let byte = b.take(8)? as u8;
                    out.push(byte);
                }
            }
            1 => {
                let (lit, dist) = fixed_tables()?;
                decode_block(&mut b, &lit, &dist, &mut out)?;
            }
            2 => {
                let (lit, dist) = dynamic_tables(&mut b)?;
                decode_block(&mut b, &lit, &dist, &mut out)?;
            }
            _ => return None,
        }
        if final_ {
            return Some((out, b.pos.div_ceil(8)));
        }
    }
}

/// Inflates an RFC 1950 zlib stream (`78 9C`-style header + Adler-32
/// trailer — the trailer is skipped, not validated, to keep the codec
/// dependency-free).
pub fn inflate_zlib(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 6 {
        return None;
    }
    let cmf = data[0];
    let flg = data[1];
    if cmf & 0x0F != 8 || (cmf as u32 * 256 + flg as u32) % 31 != 0 {
        return None;
    }
    if flg & 0x20 != 0 {
        return None; // preset dictionary unsupported
    }
    inflate(&data[2..data.len() - 4])
}

/// Like [`inflate_zlib`], but tolerates trailing bytes and reports
/// the total zlib member length (header + deflate + Adler-32 trailer)
/// so callers can step to the next member — e.g. git packfile
/// objects. The trailer itself is still skipped, not validated.
pub fn inflate_zlib_count(data: &[u8]) -> Option<(Vec<u8>, usize)> {
    if data.len() < 6 {
        return None;
    }
    let cmf = data[0];
    let flg = data[1];
    if cmf & 0x0F != 8 || (cmf as u32 * 256 + flg as u32) % 31 != 0 {
        return None;
    }
    if flg & 0x20 != 0 {
        return None; // preset dictionary unsupported
    }
    let (out, used) = inflate_count(data.get(2..)?)?;
    let total = 2usize.checked_add(used)?.checked_add(4)?;
    if total > data.len() {
        return None;
    }
    Some((out, total))
}

/// Inflates an RFC 1952 gzip stream (header fields skipped, CRC/ISIZE
/// trailer ignored).
pub fn inflate_gzip(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 18 || data[0] != 0x1f || data[1] != 0x8b || data[2] != 8 {
        return None;
    }
    let flg = data[3];
    if flg & 0xE0 != 0 {
        return None; // reserved bits must be zero
    }
    let mut at = 10usize; // magic+CM+FLG+MTIME+XFL+OS
    if flg & 0x04 != 0 {
        // FEXTRA: 16-bit little-endian length then payload.
        let xlen = *data.get(at)? as usize | (*data.get(at + 1)? as usize) << 8;
        at += 2 + xlen;
    }
    for &mask in &[0x08u8, 0x10] {
        if flg & mask != 0 {
            // FNAME / FCOMMENT: NUL-terminated.
            while *data.get(at)? != 0 {
                at += 1;
            }
            at += 1;
        }
    }
    if flg & 0x02 != 0 {
        at += 2; // FHCRC
    }
    if at >= data.len() - 8 {
        return None;
    }
    inflate(&data[at..data.len() - 8])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// zlib.decompress'd vectors produced by CPython 3 (RFC conformant).
    const HELLO_DEFLATE: &[u8] = &[0x73, 0x74, 0x72, 0x06, 0x00];
    const HELLO_ZLIB: &[u8] = &[
        0x78, 0xda, 0x73, 0x74, 0x72, 0x06, 0x00, 0x01, 0x8d, 0x00, 0xc7,
    ];
    const HELLO_GZIP: &[u8] = &[
        0x1f, 0x8b, 0x08, 0x00, 0xc1, 0xf4, 0xb5, 0x6a, 0x02, 0xff, 0x73, 0x74, 0x72, 0x06, 0x00,
        0x48, 0x03, 0x83, 0xa3, 0x03, 0x00, 0x00, 0x00,
    ];
    /// 31 × 'a', fixed-Huffman block (bfinal=1, btype=01).
    const A31_DEFLATE: &[u8] = &[0x4b, 0x4c, 0xc4, 0x0b, 0x00];
    /// 264 bytes of repeated text, still a fixed block.
    const FIXED_264: &[u8] = &[
        0x2b, 0xc9, 0x48, 0x55, 0x28, 0x2c, 0xcd, 0x4c, 0xce, 0x56, 0x48, 0x2a, 0xca, 0x2f, 0xcf,
        0x53, 0x48, 0xcb, 0xaf, 0x50, 0xc8, 0x2a, 0xcd, 0x2d, 0x28, 0x56, 0xc8, 0x2f, 0x4b, 0x2d,
        0x52, 0x28, 0x01, 0x4a, 0xe7, 0x24, 0x56, 0x55, 0x2a, 0xa4, 0xe4, 0xa7, 0x83, 0x39, 0xc3,
        0x55, 0x2d, 0x00,
    ];
    /// Uncompressed stored block (level 0).
    const STORED_DEFLATE: &[u8] = &[
        0x01, 0x0e, 0x00, 0xf1, 0xff, 0x53, 0x54, 0x4f, 0x52, 0x45, 0x44, 0x42, 0x4c, 0x4f, 0x43,
        0x4b, 0x31, 0x32, 0x33,
    ];
    const STORED_ZLIB: &[u8] = &[
        0x78, 0x01, 0x01, 0x0e, 0x00, 0xf1, 0xff, 0x53, 0x54, 0x4f, 0x52, 0x45, 0x44, 0x42, 0x4c,
        0x4f, 0x43, 0x4b, 0x31, 0x32, 0x33, 0x1e, 0xcf, 0x03, 0xd3,
    ];

    #[test]
    fn stored_block() {
        assert_eq!(inflate(STORED_DEFLATE), Some(b"STOREDBLOCK123".to_vec()));
    }

    #[test]
    fn stored_bad_nlen_rejected() {
        let mut bad = STORED_DEFLATE.to_vec();
        bad[4] ^= 0xFF;
        assert_eq!(inflate(&bad), None);
    }

    #[test]
    fn fixed_hello_roundtrip() {
        assert_eq!(inflate(HELLO_DEFLATE), Some(b"ABC".to_vec()));
        assert_eq!(inflate(A31_DEFLATE), Some(vec![b'a'; 31]));
    }

    #[test]
    fn fixed_block_with_backrefs() {
        let want = b"the quick brown fox jumps over the lazy dog "
            .iter()
            .cycle()
            .take(264)
            .copied()
            .collect::<Vec<u8>>();
        assert_eq!(inflate(FIXED_264), Some(want));
    }

    #[test]
    fn zlib_and_gzip_wrappers() {
        assert_eq!(inflate_zlib(HELLO_ZLIB), Some(b"ABC".to_vec()));
        assert_eq!(inflate_gzip(HELLO_GZIP), Some(b"ABC".to_vec()));
        assert_eq!(inflate_zlib(STORED_ZLIB), Some(b"STOREDBLOCK123".to_vec()));
        assert_eq!(inflate_zlib(&[0x78, 0x9c]), None);
    }

    #[test]
    fn count_variants_report_consumed() {
        // raw deflate member (no zlib header): "hello hello hello"
        let raw = crate::deflate::deflate(b"hello hello hello");
        let (out, used) = inflate_count(&raw).unwrap();
        assert_eq!(out, b"hello hello hello");
        assert_eq!(used, raw.len());
        // zlib member + 3 trailing bytes: used must point past the
        // member so a container can chain the next object.
        let mut z = crate::deflate::deflate_zlib(b"hi");
        let zlen = z.len();
        z.extend_from_slice(&[0xDE, 0xAD, 0xBE]);
        let (out, used) = inflate_zlib_count(&z).unwrap();
        assert_eq!(out, b"hi");
        assert_eq!(used, zlen); // header(2)+member+adler(4), not the tail
        assert!(inflate_zlib_count(&z[..4]).is_none()); // truncated
    }

    #[test]
    fn dynamic_block_decodes() {
        // zlib level 9 on 300 skewed bytes emits a btype=10 dynamic
        // block (first byte 0x25 → bfinal=1, btype=10).
        let dyn_body: &[u8] = &[
            0x25, 0x4f, 0x5b, 0x16, 0x44, 0x21, 0x08, 0x5a, 0x2b, 0xf8, 0xec, 0xb5, 0xff, 0xdf,
            0xc1, 0xb9, 0xd9, 0xd1, 0x4a, 0x04, 0x02, 0x1c, 0x46, 0x4c, 0x80, 0x05, 0xf8, 0x36,
            0x5e, 0x34, 0xb4, 0x0a, 0xe6, 0x34, 0x1d, 0x9c, 0x03, 0xc9, 0x80, 0x59, 0x07, 0x2e,
            0x18, 0x42, 0x7b, 0x5a, 0x23, 0xcc, 0x58, 0x9b, 0x8e, 0xf0, 0x57, 0x50, 0x15, 0x0b,
            0x52, 0x4c, 0x0d, 0xda, 0x2a, 0x65, 0xae, 0x33, 0x5c, 0xc3, 0x43, 0xda, 0x71, 0x73,
            0xc7, 0xca, 0x4e, 0x8a, 0xe1, 0xdf, 0xf8, 0x52, 0xfb, 0xf4, 0xd1, 0x8f, 0xe3, 0x43,
            0xd2, 0x38, 0x06, 0xc5, 0xed, 0xd1, 0x4a, 0x13, 0x67, 0xbe, 0xce, 0x8a, 0x71, 0x32,
            0x96, 0x0e, 0xf7, 0x3b, 0xdf, 0x71, 0x15, 0x3d, 0x55, 0x32, 0x84, 0x44, 0x5e, 0x72,
            0x87, 0xde, 0x57, 0xa2, 0xee, 0xff, 0x03, 0xd7, 0x6d, 0xb3, 0x6b, 0xc0, 0x26, 0xb6,
            0x25, 0x23, 0x8b, 0x9b, 0x36, 0x17, 0x4d, 0x30, 0x24, 0x24, 0x05, 0x21, 0x93, 0x16,
            0x8b, 0x1a, 0xd2, 0x16, 0x4b, 0x6f, 0xd8, 0xae, 0xcf, 0xa4, 0xe7, 0xfa, 0x01,
        ];
        let want: &[u8] = &[
            0x61, 0x61, 0x64, 0x61, 0x63, 0x62, 0x61, 0x62, 0x61, 0x62, 0x61, 0x61, 0x62, 0x67,
            0x61, 0x61, 0x64, 0x6a, 0x63, 0x62, 0x6c, 0x61, 0x68, 0x61, 0x61, 0x61, 0x61, 0x67,
            0x61, 0x63, 0x64, 0x62, 0x63, 0x61, 0x61, 0x61, 0x64, 0x62, 0x61, 0x63, 0x62, 0x61,
            0x66, 0x65, 0x61, 0x63, 0x63, 0x68, 0x65, 0x61, 0x6c, 0x61, 0x62, 0x65, 0x61, 0x62,
            0x61, 0x64, 0x66, 0x63, 0x68, 0x61, 0x65, 0x63, 0x63, 0x62, 0x67, 0x6a, 0x62, 0x64,
            0x61, 0x65, 0x64, 0x6d, 0x67, 0x61, 0x62, 0x64, 0x61, 0x62, 0x61, 0x61, 0x61, 0x66,
            0x61, 0x61, 0x62, 0x68, 0x61, 0x62, 0x63, 0x69, 0x67, 0x68, 0x61, 0x62, 0x62, 0x69,
            0x6b, 0x61, 0x61, 0x61, 0x61, 0x62, 0x63, 0x61, 0x61, 0x62, 0x62, 0x63, 0x6b, 0x64,
            0x63, 0x64, 0x64, 0x61, 0x69, 0x66, 0x68, 0x66, 0x62, 0x62, 0x61, 0x64, 0x61, 0x61,
            0x61, 0x61, 0x62, 0x61, 0x61, 0x61, 0x61, 0x62, 0x61, 0x68, 0x64, 0x61, 0x61, 0x62,
            0x62, 0x61, 0x68, 0x6d, 0x62, 0x62, 0x61, 0x61, 0x62, 0x61, 0x67, 0x61, 0x61, 0x6b,
            0x63, 0x61, 0x63, 0x61, 0x63, 0x6c, 0x68, 0x65, 0x61, 0x62, 0x61, 0x66, 0x63, 0x66,
            0x61, 0x61, 0x66, 0x6d, 0x68, 0x66, 0x67, 0x65, 0x61, 0x63, 0x62, 0x61, 0x61, 0x61,
            0x61, 0x64, 0x6b, 0x62, 0x6a, 0x6d, 0x6b, 0x62, 0x61, 0x61, 0x61, 0x61, 0x64, 0x69,
            0x67, 0x62, 0x64, 0x66, 0x61, 0x64, 0x69, 0x66, 0x65, 0x62, 0x61, 0x66, 0x61, 0x66,
            0x6c, 0x62, 0x62, 0x6a, 0x65, 0x61, 0x61, 0x61, 0x69, 0x66, 0x61, 0x67, 0x6c, 0x64,
            0x62, 0x63, 0x61, 0x61, 0x6c, 0x64, 0x63, 0x6a, 0x62, 0x68, 0x67, 0x61, 0x61, 0x61,
            0x61, 0x63, 0x61, 0x62, 0x61, 0x69, 0x62, 0x62, 0x63, 0x69, 0x62, 0x6a, 0x62, 0x63,
            0x63, 0x61, 0x62, 0x61, 0x61, 0x66, 0x61, 0x62, 0x65, 0x63, 0x61, 0x63, 0x63, 0x66,
            0x61, 0x63, 0x61, 0x61, 0x66, 0x62, 0x63, 0x65, 0x69, 0x62, 0x64, 0x62, 0x63, 0x64,
            0x62, 0x63, 0x62, 0x6a, 0x65, 0x68, 0x6a, 0x61, 0x63, 0x6a, 0x67, 0x61, 0x61, 0x62,
            0x61, 0x61, 0x61, 0x64, 0x66, 0x69,
        ];
        assert_eq!(inflate(dyn_body), Some(want.to_vec()));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(inflate(&[]), None);
        assert_eq!(inflate(&[0x07]), None); // btype 11 reserved
        assert_eq!(inflate(&[0x06, 0x00, 0x00]), None); // btype 10 truncated
        assert_eq!(inflate_zlib(&[0x00; 40]), None);
        assert_eq!(inflate_gzip(&[0x00; 40]), None);
    }

    #[test]
    fn deterministic_twice() {
        let a = inflate(FIXED_264);
        let b = inflate(FIXED_264);
        assert_eq!(a, b);
    }
}

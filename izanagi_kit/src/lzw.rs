//! LZW codec — greedy phrase-table compression, the classic
//! Unix-`compress`/GIF scheme. 12-bit codes over
//! [`bits`](crate::bits): the first 256 codes are literals, then the
//! dictionary grows one phrase per emitted code up to 4096 entries and
//! freezes. Inside the `rle → lzss → huffman` ladder this is the
//! variant that adapts *without* a stored model: the wire carries no
//! dictionary at all — the decoder rebuilds it in lockstep, so the
//! mapping is a pure function of the stream.
//!
//! ```
//! use izanagi_kit::lzw;
//! let wire = lzw::encode(b"ababa babab ababa");
//! assert_eq!(lzw::decode(&wire).as_deref(), Some(b"ababa babab ababa".as_ref()));
//! ```

use crate::bits::{BitReader, BitWriter};

/// Code width in bits; the dictionary cap is `1 << CODE_BITS`.
const CODE_BITS: u32 = 12;
const CAP: usize = 1 << CODE_BITS;

/// LZW-compress `data`; always succeeds (worst case ≈ 12/9 expansion).
pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut dict: std::collections::BTreeMap<Vec<u8>, u32> = std::collections::BTreeMap::new();
    for i in 0..256u32 {
        dict.insert(vec![i as u8], i);
    }
    let mut next = 256u32;
    let mut w = BitWriter::new();
    let mut cur: Vec<u8> = Vec::new();
    for &b in data {
        let mut ext = cur.clone();
        ext.push(b);
        if dict.contains_key(&ext) {
            cur = ext;
        } else {
            if let Some(&code) = dict.get(&cur) {
                w.write_bits(code as u64, CODE_BITS);
            }
            if next < CAP as u32 {
                dict.insert(ext, next);
                next += 1;
            }
            cur.clear();
            cur.push(b);
        }
    }
    if let Some(&code) = dict.get(&cur) {
        w.write_bits(code as u64, CODE_BITS);
    }
    w.into_bytes()
}

/// Inverse of [`encode`]; `None` on truncated wires, out-of-range
/// codes, or the KwKwK case pointing past the table.
pub fn decode(wire: &[u8]) -> Option<Vec<u8>> {
    let mut dict: Vec<Vec<u8>> = (0..256u32).map(|i| vec![i as u8]).collect();
    let mut r = BitReader::new(wire);
    let mut out = Vec::new();
    let mut prev: Vec<u8> = Vec::new();
    while r.bits_remaining() >= CODE_BITS as u64 {
        let code = r.read_bits(CODE_BITS).ok()? as usize;
        let entry = if code < dict.len() {
            dict[code].clone()
        } else if code == dict.len() && !prev.is_empty() {
            // KwKwK: the new phrase is prev + prev[0].
            let mut e = prev.clone();
            e.push(prev[0]);
            e
        } else {
            return None;
        };
        out.extend_from_slice(&entry);
        if !prev.is_empty() && dict.len() < CAP {
            let mut e = prev.clone();
            e.push(entry[0]);
            dict.push(e);
        }
        prev = entry;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn round_trip_and_malformed() {
        for data in [
            &b""[..],
            b"a",
            b"abababababababab",
            b"TOBEORNOTTOBEORTOBEORNOT",
            &vec![7u8; 5000][..],
        ] {
            assert_eq!(decode(&encode(data)).as_deref(), Some(data));
        }
        // Random binary + low-entropy random text.
        let mut rng = SplitMix64::new(0xF00D);
        for _ in 0..100 {
            let n = (rng.below(2000) + 1) as usize;
            let data: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
            assert_eq!(decode(&encode(&data)).as_deref(), Some(data.as_slice()));
        }
        let mut rng2 = SplitMix64::new(0xABC0);
        let text: Vec<u8> = (0..10_000).map(|_| b'a' + (rng2.below(6) as u8)).collect();
        let wire = encode(&text);
        assert_eq!(decode(&wire).as_deref(), Some(text.as_slice()));
        assert!(wire.len() < text.len(), "repetitive input must shrink");
        // Truncation: drop the tail byte so the last code is short.
        let cut = &wire[..wire.len() - 1];
        // Decoding a truncated stream is allowed to succeed with less
        // output — but must never panic, and if it decodes it must be
        // a strict prefix of the truth.
        if let Some(d) = decode(cut) {
            assert!(d.len() <= text.len());
            assert_eq!(&text[..d.len()], &d[..]);
        }
        // Bit-flipped header still decodes or fails cleanly.
        let mut bad = wire.clone();
        if let Some(b) = bad.first_mut() {
            *b ^= 0xFF;
        }
        let _ = decode(&bad);
    }

    #[test]
    fn kwkwk_case() {
        // "aaaa..." triggers code == dict.len() on the decoder.
        let data = vec![b'x'; 100];
        assert_eq!(decode(&encode(&data)).as_deref(), Some(data.as_slice()));
        // Encode a stream manually known to hit KwKwK early: "ababab..."
        let data = b"abababababababababababababab";
        assert_eq!(decode(&encode(&data[..])).as_deref(), Some(&data[..]));
    }
}

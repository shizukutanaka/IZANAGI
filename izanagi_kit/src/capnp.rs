//! Cap'n Proto serialization envelope — the stream framing of the
//! Cap'n Proto encoding spec (segment table, little-endian words).
//!
//! A stream begins with `u32 LE segment_count − 1` followed by
//! `segment_count` u32 LE word sizes (each segment's size is in
//! 8-byte words), padded to an even count of u32s (the table itself
//! is a whole number of words). A single-segment stream has a
//! 2-word table; a two-segment stream's table is 3 words + 1 pad.
//!
//! ```
//! use izanagi_kit::capnp::{parse, segment, segment_count};
//! let mut d = vec![0, 0, 0, 0];       // 1 segment
//! d.extend_from_slice(&[2, 0, 0, 0]); // 2 words
//! d.extend_from_slice(&[0; 16]);      // 2 words of data
//! let c = parse(&d).unwrap();
//! assert_eq!(segment_count(&c), 1);
//! assert_eq!(c.data_at, 8);
//! let s = segment(&d, &c, 0).unwrap();
//! assert_eq!(s.1, 2); // 2 words
//! ```

/// A parsed stream header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capnp {
    /// Number of segments (`segment_count_minus_one + 1`).
    pub segments: u32,
    /// Offset of the segment-size table (always 4).
    pub table_at: usize,
    /// Offset of segment 0's data (past the padded table).
    pub data_at: usize,
    /// Total bytes the stream occupies (table + all segments).
    pub total_len: usize,
}

fn u32s(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Parse the segment table. `None` when the table or the declared
/// segments would run past the buffer.
pub fn parse(d: &[u8]) -> Option<Capnp> {
    let n = u32s(d, 0)?.checked_add(1)?; // stored as count−1
    if n == 0 || n > 512 {
        return None; // absurd table; also bounds table bytes
    }
    // table = 1 count word + n size words, padded to even u32 count
    let words = 1 + n as usize;
    let table_len = words.div_ceil(2) * 8;
    if table_len > d.len() {
        return None;
    }
    let mut total = table_len;
    for i in 0..n {
        let w = u32s(d, 4 + i as usize * 4)? as usize;
        total = total.checked_add(w.checked_mul(8)?)?;
    }
    if total > d.len() {
        return None;
    }
    Some(Capnp {
        segments: n,
        table_at: 4,
        data_at: table_len,
        total_len: total,
    })
}

/// Declared segment count.
pub fn segment_count(c: &Capnp) -> usize {
    c.segments as usize
}

/// Segment `i`'s `(offset, word_count)`. `None` when `i` is out of
/// range.
pub fn segment(d: &[u8], c: &Capnp, i: usize) -> Option<(usize, usize)> {
    if i >= c.segments as usize {
        return None;
    }
    let words = u32s(d, c.table_at + i * 4)? as usize;
    let mut at = c.data_at;
    for j in 0..i {
        let w = u32s(d, c.table_at + j * 4)? as usize;
        at += w * 8;
    }
    Some((at, words))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_seg() -> Vec<u8> {
        let mut d = vec![0, 0, 0, 0];
        d.extend_from_slice(&[2, 0, 0, 0]);
        d.extend_from_slice(&[0xAB; 16]);
        d
    }

    fn two_seg() -> Vec<u8> {
        let mut d = vec![1, 0, 0, 0]; // 2 segments
        d.extend_from_slice(&[1, 0, 0, 0]); // seg0: 1 word
        d.extend_from_slice(&[3, 0, 0, 0]); // seg1: 3 words
        d.extend_from_slice(&[0, 0, 0, 0]); // pad to even u32 count
        d.extend_from_slice(&[0x11; 8]); // seg0 data
        d.extend_from_slice(&[0x22; 24]); // seg1 data
        d
    }

    #[test]
    fn one_segment() {
        let d = one_seg();
        let c = parse(&d).unwrap();
        assert_eq!(segment_count(&c), 1);
        assert_eq!(c.data_at, 8);
        assert_eq!(c.total_len, 24);
        assert_eq!(segment(&d, &c, 0), Some((8, 2)));
        assert_eq!(segment(&d, &c, 1), None);
    }

    #[test]
    fn two_segments_padded_table() {
        let d = two_seg();
        let c = parse(&d).unwrap();
        assert_eq!(segment_count(&c), 2);
        assert_eq!(c.data_at, 16); // 4-word table
        assert_eq!(segment(&d, &c, 0), Some((16, 1)));
        assert_eq!(segment(&d, &c, 1), Some((24, 3)));
        assert_eq!(c.total_len, 48);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0, 0, 0]).is_none());
        let mut d = one_seg();
        d.truncate(20); // declared 24
        assert!(parse(&d).is_none());
        let mut d2 = one_seg();
        d2[0] = 0xFF; // ~512 segments
        assert!(parse(&d2).is_none());
    }
}

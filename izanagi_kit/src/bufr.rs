//! WMO BUFR (FM 94) message envelope parsing.
//!
//! Layout: `BUFR` + total_len(u24 BE) + edition byte, then section 1
//! (`len u24 + master table + ...`), optional section 2, section 3,
//! section 4, and a `7777` trailer.
//!
//! ```
//! use izanagi_kit::bufr::parse;
//!
//! // edition 4: BUFR len=20, sec1(len 8: 3B len + body), then 7777
//! let m = [
//!     b'B', b'U', b'F', b'R',
//!     0, 0, 20, 4,       // total len 20, edition 4
//!     0, 0, 8, 0, 0, 0, 0, 0, // sec1: len 8, master table 0
//!     b'7', b'7', b'7', b'7',
//! ];
//! let b = parse(&m).unwrap();
//! assert_eq!(b.edition, 4);
//! ```

/// Parsed BUFR envelope.
#[derive(Debug)]
pub struct Bufr {
    /// Declared total message length (must equal `d.len()`).
    pub total_len: usize,
    /// BUFR edition (3 or 4 seen in the wild).
    pub edition: u8,
    /// Offset of section 1.
    pub sec1_at: usize,
    /// Section 1 length.
    pub sec1_len: usize,
    /// BUFR master table number (section 1 byte 3).
    pub master_table: u8,
    /// Whether an optional section 2 is present (edition 4: bit in sec1 flag byte).
    pub has_sec2: bool,
    /// Offset of section 3 (after sec1 + optional sec2, when skippable).
    pub sec3_at: Option<usize>,
}

fn u24(d: &[u8], o: usize) -> usize {
    ((d[o] as usize) << 16) | ((d[o + 1] as usize) << 8) | d[o + 2] as usize
}

/// Parses a BUFR message: requires `BUFR` magic, declared length matching
/// `d.len()`, section-1 fully present, and a `7777` trailer.
pub fn parse(d: &[u8]) -> Option<Bufr> {
    if d.len() < 12 || &d[..4] != b"BUFR" {
        return None;
    }
    let total = u24(d, 4);
    if total != d.len() {
        return None;
    }
    let edition = d[7];
    if !(3..=4).contains(&edition) {
        return None;
    }
    let sec1_at = 8;
    let sec1_len = u24(d, sec1_at);
    if sec1_len < 3 || sec1_at + sec1_len > d.len() {
        return None;
    }
    let master_table = d[sec1_at + 3];
    // edition 4: sec1 byte 9 (offset sec1_at+9) bit... the "optional section
    // present" flag lives in sec1 at a fixed offset for ed3/ed4 headers;
    // detect conservatively by scanning for a plausible sec3/7777 instead.
    let after1 = sec1_at + sec1_len;
    let mut has_sec2 = false;
    let mut sec3_at = None;
    if after1 + 4 <= d.len() {
        // If bytes at after1 look like a section-2 header (len>=4), treat them
        // as sec2 only when an explicit flag check is impossible; BUFR sec2
        // has no magic so we use the sec1 flag byte when available.
        let flag_at = sec1_at + 9;
        let flag = if flag_at < sec1_at + sec1_len {
            d[flag_at]
        } else {
            0
        };
        has_sec2 = flag & 0x80 != 0;
        if has_sec2 {
            let sec2_len = u24(d, after1);
            if sec2_len < 4 || after1 + sec2_len > d.len() {
                return None;
            }
            sec3_at = Some(after1 + sec2_len);
        } else {
            sec3_at = Some(after1);
        }
    }
    if &d[d.len() - 4..] != b"7777" {
        return None;
    }
    Some(Bufr {
        total_len: total,
        edition,
        sec1_at,
        sec1_len,
        master_table,
        has_sec2,
        sec3_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ed4() {
        // 8B head + 8B section-1 + 4B "7777" = 20 bytes
        let m = [
            b'B', b'U', b'F', b'R', 0, 0, 20, 4, // header, len 20, ed 4
            0, 0, 8, 0, 0, 0, 0, 0, // sec1: len 8, master table 0
            b'7', b'7', b'7', b'7',
        ];
        let b = parse(&m).unwrap();
        assert_eq!(b.edition, 4);
        assert_eq!(b.total_len, 20);
        assert_eq!(b.sec1_at, 8);
        assert_eq!(b.sec1_len, 8);
        assert_eq!(b.master_table, 0);
        assert!(!b.has_sec2);
        assert_eq!(b.sec3_at, Some(16));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[b'B', b'U', b'F', b'X', 0, 0, 8, 4]).is_none());
        // edition out of range
        assert!(parse(&[b'B', b'U', b'F', b'R', 0, 0, 12, 9, 0, 0, 4, 0]).is_none());
        // len mismatch
        assert!(
            parse(&[b'B', b'U', b'F', b'R', 0, 0, 40, 4, 0, 0, 4, 0, b'7', b'7', b'7', b'7'])
                .is_none()
        );
    }
}

//! TD0 (TeleDisk disk image): `TD` (stored) or `td`
//! (LZW compressed) signature + sequence, check-sig,
//! version, data rate, drive type, stepping, DOS flag,
//! sides, header CRC16; optional comment block
//! (`u16le` crc + `u16le` len + date/time + text).
//!
//! ```
//! let mut d = b"TD\x00T".to_vec(); // sig, seq, check
//! d.extend_from_slice(&[0x20, 2, 1, 0, 0, 1]); // ver,rate,drv,step,dos,sides
//! d.extend_from_slice(&[0x00, 0x00]); // header crc
//! d.extend_from_slice(&[0, 0, 5, 0]); // comment crc, len 5
//! d.extend_from_slice(&[99, 12, 31, 23, 59, 59]); // y/m/d h:m:s
//! d.extend_from_slice(b"hello");
//! d.extend_from_slice(&[0; 8]); // payload
//! let p = izanagi_kit::td0::parse(&d).unwrap();
//! assert!(!p.compressed);
//! assert!(p.has_comment);
//! assert_eq!(p.comment_len, 5);
//! assert!(izanagi_kit::td0::detect(&d));
//! ```

/// Census of a TeleDisk image.
#[derive(Debug, Clone, PartialEq)]
pub struct Td0 {
    /// `td` (compressed) vs `TD` (stored).
    pub compressed: bool,
    /// Sequence byte.
    pub sequence: u8,
    /// Check-signature byte (echoes `T`/`t`).
    pub check_sig: u8,
    /// Version byte.
    pub version: u8,
    /// Data rate code (0 = 250k, 1 = 300k, 2 = 500k).
    pub data_rate: u8,
    /// Drive type code.
    pub drive_type: u8,
    /// Stepping byte.
    pub stepping: u8,
    /// DOS-allocation flag byte.
    pub dos_alloc: u8,
    /// Sides (1 or 2).
    pub sides: u8,
    /// Header CRC16 (`u16le`).
    pub header_crc: u16,
    /// Comment block present and complete.
    pub has_comment: bool,
    /// Comment payload length.
    pub comment_len: u32,
    /// Comment year (0–99).
    pub comment_year: u8,
    /// Bytes after the header (+ comment block).
    pub payload_len: u32,
}

fn le16(b: &[u8], i: usize) -> u16 {
    b[i] as u16 | ((b[i + 1] as u16) << 8)
}

/// `true` on `TD`/`td` with a plausible header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 12
        && ((b[0] == b'T' && b[1] == b'D') || (b[0] == b't' && b[1] == b'd'))
        && b[3] == b[0]
        && (b[9] == 1 || b[9] == 2)
}

/// Census; `None` without the TD0 signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Td0> {
    if !detect(b) {
        return None;
    }
    let mut t = Td0 {
        compressed: b[0] == b't',
        sequence: b[2],
        check_sig: b[3],
        version: b[4],
        data_rate: b[5],
        drive_type: b[6],
        stepping: b[7],
        dos_alloc: b[8],
        sides: b[9],
        header_crc: le16(b, 10),
        has_comment: false,
        comment_len: 0,
        comment_year: 0,
        payload_len: 0,
    };
    // Optional comment block: crc(2) len(2) then
    // date(3) + time(3) + len bytes of text.
    let mut tail = 12usize;
    if b.len() >= 22 {
        let clen = le16(b, 14) as usize;
        if 22 + clen <= b.len()
            && b[16] <= 99
            && b[17] >= 1
            && b[17] <= 12
            && b[18] >= 1
            && b[18] <= 31
            && b[19] <= 23
            && b[20] <= 59
            && b[21] <= 59
        {
            t.has_comment = true;
            t.comment_len = clen as u32;
            t.comment_year = b[16];
            tail = 22 + clen;
        }
    }
    t.payload_len = b.len().saturating_sub(tail) as u32;
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"td\x00t".to_vec(); // compressed
        d.extend_from_slice(&[0x15, 0, 4, 0, 0, 2]);
        d.extend_from_slice(&[0xAB, 0xCD]);
        d.extend_from_slice(&[0x11, 0x22, 4, 0]); // crc, len 4
        d.extend_from_slice(&[0, 1, 1, 0, 0, 1]);
        d.extend_from_slice(b"note");
        d.extend_from_slice(&[0; 16]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"TD\x00X"));
        assert!(!detect(b"XX\x00X"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert!(p.compressed);
        assert_eq!(p.version, 0x15);
        assert_eq!(p.sides, 2);
        assert_eq!(p.header_crc, 0xCDAB);
        assert!(p.has_comment);
        assert_eq!(p.comment_len, 4);
        assert_eq!(p.payload_len, 16);
    }

    #[test]
    fn no_comment() {
        let mut d = b"TD\x00T".to_vec();
        d.extend_from_slice(&[0x20, 2, 1, 0, 0, 1]);
        d.extend_from_slice(&[0, 0]);
        d.extend_from_slice(&[7; 5]); // data, no comment block
        let p = parse(&d).unwrap();
        assert!(!p.has_comment);
        assert_eq!(p.payload_len, 5);
    }
}

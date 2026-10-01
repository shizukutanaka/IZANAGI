//! lzip compressed file (`.lz`, LZMA stream): each member is
//! `LZIP` magic + `u8` version + `u8` coded dictionary size, then
//! the LZMA payload, then a 20-byte trailer (`u32le` CRC32 +
//! `u64le` data size + `u64le` member size). Multi-member files
//! are consecutive members.
//!
//! ```
//! let mut d = b"LZIP".to_vec();
//! d.extend_from_slice(&[1, 0x80]); // version 1, dict code 0x80
//! d.extend_from_slice(&[0xAA; 10]); // payload
//! d.extend_from_slice(&[0; 4]); // crc32
//! d.extend_from_slice(&[0; 8]); // data size
//! d.extend_from_slice(&[36, 0, 0, 0, 0, 0, 0, 0]); // member size
//! let p = izanagi_kit::lzip::parse(&d).unwrap();
//! assert_eq!(p.version, 1);
//! assert_eq!(p.dict_size_log2, 4); // top-3-bits exponent
//! assert!(izanagi_kit::lzip::detect(&d));
//! ```

/// Census of an lzip file.
#[derive(Debug, Clone, PartialEq)]
pub struct Lzip {
    /// `u8` version (1).
    pub version: u8,
    /// Coded dictionary size byte (log2 base + fraction bits).
    pub dict_code: u8,
    /// Dictionary base-2 exponent (code >> 5 with fraction).
    pub dict_size_log2: u8,
    /// Members counted (consecutive `LZIP` headers).
    pub members: u32,
    /// `data_size` from the final trailer (total uncompressed bytes).
    pub data_size: u64,
    /// `member_size` from the final trailer.
    pub member_size: u64,
    /// `crc32` from the final trailer.
    pub crc32: u32,
    /// Trailer `member_size` matches the measured member length.
    pub size_consistent: bool,
    /// A member ran past the end of the buffer.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}
fn le64(b: &[u8], i: usize) -> u64 {
    let mut v = 0u64;
    for (s, &c) in b[i..i + 8].iter().enumerate() {
        v |= (c as u64) << (s * 8);
    }
    v
}

/// `true` on `LZIP` + version 1 + a sane coded dictionary size.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 6 && b[..4] == *b"LZIP" && b[4] == 1 && b[5] > 0 && b[5] != 0xFF
}

/// Census; `None` without `LZIP`. The LZMA payload is not walked —
/// the 20-byte trailer at the member end supplies crc/data/member
/// sizes; consecutive members are counted when their magic follows
/// a complete member.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Lzip> {
    if !detect(b) {
        return None;
    }
    let code = b[5];
    // dict size = 2^(code>>5) - (code&31)*2^(code>>5)/16 per lzip spec
    let mut l = Lzip {
        version: b[4],
        dict_code: code,
        dict_size_log2: code >> 5,
        members: 0,
        data_size: 0,
        member_size: 0,
        crc32: 0,
        size_consistent: false,
        truncated: false,
    };
    // First member's trailer sits at the tail of the buffer for the
    // common single-member case; `member_size` covers header+data+trailer.
    if b.len() >= 26 {
        l.members = 1;
        let t = b.len() - 20;
        l.crc32 = le32(b, t);
        l.data_size = le64(b, t + 4);
        l.member_size = le64(b, t + 12);
        l.size_consistent = l.member_size as usize == b.len();
    } else {
        l.truncated = true;
    }
    // Count additional members after a consistent first member.
    if l.size_consistent {
        let mut j = l.member_size as usize;
        while j + 6 <= b.len() && &b[j..j + 4] == b"LZIP" {
            l.members += 1;
            if j + 26 > b.len() {
                l.truncated = true;
                break;
            }
            let ms = le64(b, b.len() - 8) as usize;
            if ms == 0 || j + ms > b.len() {
                j = b.len();
            } else {
                j += ms;
            }
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"LZIP".to_vec();
        d.extend_from_slice(&[1, 0x23]); // dict 1<<4 base + 3/16 fraction
        d.extend_from_slice(&[0xAA; 14]); // payload
        let member_size = 6 + 14 + 20;
        d.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]); // crc
        d.extend_from_slice(&500u64.to_le_bytes()); // data size
        d.extend_from_slice(&(member_size as u64).to_le_bytes());
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"LZIP\x02"));
        assert!(!detect(b"LZIP"));
    }

    #[test]
    fn parses_member() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.members, 1);
        assert_eq!(p.data_size, 500);
        assert_eq!(p.member_size, 40);
        assert_eq!(p.crc32, 0x44332211);
        assert!(p.size_consistent);
        assert!(!p.truncated);
    }

    #[test]
    fn dict_size_decoded() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.dict_size_log2, 1); // 0x23 >> 5
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"not lzip").is_none());
    }
}

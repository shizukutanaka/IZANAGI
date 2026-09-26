//! Intel HEX object format.
//!
//! Each line is `:LLAAAATT[DD..]CC` — byte count, 16-bit address,
//! record type, data, and a checksum byte such that the low byte of
//! the running sum (count..data..checksum) is zero. Types: `00` data,
//! `01` end of file, `02` extended segment address, `03` start
//! segment address, `04` extended linear address, `05` start linear
//! address.
//!
//! ```
//! use izanagi_kit::intelhex::{parse, Kind};
//!
//! // ":020000040001F9" = ext-linear 1; ":0400100001020304E2" = data;
//! // ":00000001FF" = EOF
//! let s = ":020000040001F9\r\n:0400100001020304E2\r\n:00000001FF\r\n";
//! let rs = parse(s).unwrap();
//! assert_eq!(rs[0].kind, Kind::ExtLinAddr);
//! assert_eq!(rs[1].addr, 0x0010);
//! assert_eq!(rs[1].data, vec![1, 2, 3, 4]);
//! assert_eq!(rs[2].kind, Kind::Eof);
//! ```

use std::vec::Vec;

/// Record type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `00` — data bytes.
    Data,
    /// `01` — end of file.
    Eof,
    /// `02` — extended segment address (`<<4` base).
    ExtSegAddr,
    /// `03` — start segment address (CS:IP pair).
    StartSegAddr,
    /// `04` — extended linear address (`<<16` base).
    ExtLinAddr,
    /// `05` — start linear address (EIP).
    StartLinAddr,
    /// Any other record type.
    Other(u8),
}

impl Kind {
    fn of(v: u8) -> Self {
        match v {
            0x00 => Kind::Data,
            0x01 => Kind::Eof,
            0x02 => Kind::ExtSegAddr,
            0x03 => Kind::StartSegAddr,
            0x04 => Kind::ExtLinAddr,
            0x05 => Kind::StartLinAddr,
            v => Kind::Other(v),
        }
    }
}

/// One decoded record.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// Record type.
    pub kind: Kind,
    /// 16-bit load address (data records) or record payload address.
    pub addr: u16,
    /// Decoded data bytes (`data.len()` equals the count byte).
    pub data: Vec<u8>,
}

impl Record {
    /// For `ExtSegAddr` records: the paragraph base (`value << 4`).
    /// For `ExtLinAddr` records: the high address bits (`value << 16`).
    /// Reads the first two data bytes as a big-endian u16; `None` for
    /// other record kinds or short payloads.
    pub fn address_base(&self) -> Option<u32> {
        if self.data.len() < 2 {
            return None;
        }
        let v = u32::from(self.data[0]) << 8 | u32::from(self.data[1]);
        match self.kind {
            Kind::ExtSegAddr => Some(v << 4),
            Kind::ExtLinAddr => Some(v << 16),
            _ => None,
        }
    }
}

fn hexv(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

fn hex2(b: &[u8], i: usize) -> Option<u8> {
    Some(hexv(*b.get(i)?)? << 4 | hexv(*b.get(i + 1)?)?)
}

fn line(l: &[u8]) -> Option<Record> {
    if l.first() != Some(&b':') || l.len() < 11 {
        return None;
    }
    let count = hex2(l, 1)?;
    let addr = u16::from(hex2(l, 3)?) << 8 | u16::from(hex2(l, 5)?);
    let kind = hex2(l, 7)?;
    let n = usize::from(count);
    // 1 ':' + 2 count + 4 addr + 2 kind + 2n data + 2 checksum
    if l.len() != 11 + n * 2 {
        return None;
    }
    let mut data = Vec::with_capacity(n);
    let mut sum =
        u32::from(count) + u32::from(addr >> 8) + u32::from(addr & 0xFF) + u32::from(kind);
    for i in 0..n {
        let b = hex2(l, 9 + i * 2)?;
        sum += u32::from(b);
        data.push(b);
    }
    let chk = hex2(l, 9 + n * 2)?;
    if (sum + u32::from(chk)) & 0xFF != 0 {
        return None;
    }
    Some(Record {
        kind: Kind::of(kind),
        addr,
        data,
    })
}

/// Parse a whole file. Every non-empty line must be a valid record
/// (lines may end in `\r\n` or `\n`); a checksum or shape failure
/// rejects the whole input.
pub fn parse(s: &str) -> Option<Vec<Record>> {
    let mut out = Vec::new();
    for l in s.lines() {
        let l = l.strip_suffix('\r').unwrap_or(l);
        if l.is_empty() {
            continue;
        }
        out.push(line(l.as_bytes())?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn parses_data_eof_and_ext() {
        let s = ":020000040001F9\n:0400100001020304E2\n:00000001FF\n";
        let rs = parse(s).unwrap();
        assert_eq!(rs.len(), 3);
        assert_eq!(rs[0].kind, Kind::ExtLinAddr);
        assert_eq!(rs[0].address_base(), Some(0x1_0000));
        assert_eq!(rs[1].kind, Kind::Data);
        assert_eq!(rs[1].addr, 0x0010);
        assert_eq!(rs[1].data, vec![1, 2, 3, 4]);
        assert_eq!(rs[2].kind, Kind::Eof);
        assert_eq!(rs[2].data.len(), 0);
    }

    #[test]
    fn ext_seg_base() {
        let rs = parse(":020000021234B6\n").unwrap();
        assert_eq!(rs[0].kind, Kind::ExtSegAddr);
        assert_eq!(rs[0].address_base(), Some(0x12340));
    }

    #[test]
    fn kind_variants_and_base_none() {
        assert_eq!(Kind::of(0x03), Kind::StartSegAddr);
        assert_eq!(Kind::of(0x05), Kind::StartLinAddr);
        assert_eq!(Kind::of(0x55), Kind::Other(0x55));
        let rs = parse(":0100000300FC\n").unwrap();
        assert_eq!(rs[0].kind, Kind::StartSegAddr);
        assert_eq!(rs[0].address_base(), None);
        let d = Record {
            kind: Kind::Data,
            addr: 0,
            data: Vec::new(),
        };
        assert_eq!(d.address_base(), None);
    }

    #[test]
    fn rejects_bad_checksum_and_shape() {
        assert_eq!(parse(":0400100001020304E4\n"), None); // off-by-one cksum
        assert_eq!(parse(":04001000010203\n"), None); // short
        assert_eq!(parse("0400100001020304E3\n"), None); // no ':'
        assert_eq!(parse(":0400100001020304E2X\n"), None); // trailing junk
        assert_eq!(parse(":0G0000000102\n"), None); // bad hex
        assert_eq!(parse(""), Some(Vec::new()));
        assert_eq!(parse("\n\n"), Some(Vec::new()));
    }
}

//! Motorola S-record object format.
//!
//! Each line is `STLLAA..DD..CC` — a type digit (`S0`..`S9`), a byte
//! count covering the address, data and checksum bytes, an address
//! (2/3/4 bytes big-endian depending on type), the payload, and a
//! checksum byte equal to the ones-complement of the running sum.
//! `S1`/`S2`/`S3` are data (16/24/32-bit addresses), `S0` is the
//! header record, `S5` the record count, and `S7`/`S8`/`S9` the
//! execution entry point.
//!
//! ```
//! use izanagi_kit::srec::{parse, Kind};
//!
//! // S0 header "HDR", one S1 data record, entry point 0.
//! let s = "S00600004844521B\nS1062000010203D3\nS9030000FC\n";
//! let rs = parse(s).unwrap();
//! assert_eq!(rs[0].kind, Kind::Header);
//! assert_eq!(rs[1].kind, Kind::Data16);
//! assert_eq!(rs[1].addr, 0x2000);
//! assert_eq!(rs[1].data, vec![1, 2, 3]);
//! assert_eq!(rs[2].kind, Kind::Entry16);
//! ```

use std::vec::Vec;

/// Record type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `S0` — header (usually carries an ASCII tag).
    Header,
    /// `S1` — data with a 16-bit address.
    Data16,
    /// `S2` — data with a 24-bit address.
    Data24,
    /// `S3` — data with a 32-bit address.
    Data32,
    /// `S5` — count of preceding `S1`/`S2`/`S3` records.
    Count,
    /// `S7` — entry point, 32-bit address.
    Entry32,
    /// `S8` — entry point, 24-bit address.
    Entry24,
    /// `S9` — entry point, 16-bit address.
    Entry16,
    /// `S4`/`S6` or any other digit.
    Other(u8),
}

impl Kind {
    fn of(v: u8) -> Self {
        match v {
            0 => Kind::Header,
            1 => Kind::Data16,
            2 => Kind::Data24,
            3 => Kind::Data32,
            5 => Kind::Count,
            7 => Kind::Entry32,
            8 => Kind::Entry24,
            9 => Kind::Entry16,
            v => Kind::Other(v),
        }
    }

    /// Address byte length for this record type (0 = invalid shape).
    fn addr_len(self) -> usize {
        match self {
            Kind::Header | Kind::Data16 | Kind::Count | Kind::Entry16 => 2,
            Kind::Data24 | Kind::Entry24 => 3,
            Kind::Data32 | Kind::Entry32 => 4,
            Kind::Other(_) => 0,
        }
    }
}

/// One decoded record.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// Record type.
    pub kind: Kind,
    /// Address field (big-endian, up to 32 bits).
    pub addr: u32,
    /// Decoded data bytes.
    pub data: Vec<u8>,
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
    if l.first() != Some(&b'S') || l.len() < 4 {
        return None;
    }
    let kind = Kind::of(*l.get(1)? - b'0');
    let alen = kind.addr_len();
    if alen == 0 {
        return None;
    }
    if *l.get(1)? > b'9' {
        return None;
    }
    let count = usize::from(hex2(l, 2)?);
    // 2 'Sx' + 2 count + 2*count bytes of addr+data+cksum
    if l.len() != 4 + count * 2 || count < alen + 1 {
        return None;
    }
    let mut sum = count;
    let mut addr = 0u32;
    for i in 0..alen {
        let b = hex2(l, 4 + i * 2)?;
        addr = addr << 8 | u32::from(b);
        sum += usize::from(b);
    }
    let n = count - alen - 1;
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let b = hex2(l, 4 + alen * 2 + i * 2)?;
        sum += usize::from(b);
        data.push(b);
    }
    let chk = hex2(l, 4 + count * 2 - 2)?;
    if (sum + usize::from(chk)) & 0xFF != 0xFF {
        return None;
    }
    Some(Record { kind, addr, data })
}

/// Parse a whole file. Every non-empty line must be a valid record;
/// a checksum or shape failure rejects the whole input.
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
    fn parses_header_data_entry() {
        let s = "S00600004844521B\nS1062000010203D3\nS9030000FC\n";
        let rs = parse(s).unwrap();
        assert_eq!(rs.len(), 3);
        assert_eq!(rs[0].kind, Kind::Header);
        assert_eq!(rs[0].addr, 0);
        assert_eq!(rs[0].data, b"HDR");
        assert_eq!(rs[1].kind, Kind::Data16);
        assert_eq!(rs[1].addr, 0x2000);
        assert_eq!(rs[1].data, vec![1, 2, 3]);
        assert_eq!(rs[2].kind, Kind::Entry16);
        assert_eq!(rs[2].addr, 0);
        assert_eq!(rs[2].data.len(), 0);
    }

    #[test]
    fn wider_addresses() {
        // S2: count 04, addr 0x010203, no data; sum = 4+1+2+3 = 0x0A -> cksum 0xF5
        let rs = parse("S204010203F5\n").unwrap();
        assert_eq!(rs[0].kind, Kind::Data24);
        assert_eq!(rs[0].addr, 0x010203);
        // S3: count 05, addr 0x01020304; sum = 5+1+2+3+4 = 0x0F -> cksum 0xF0
        let rs = parse("S30501020304F0\n").unwrap();
        assert_eq!(rs[0].kind, Kind::Data32);
        assert_eq!(rs[0].addr, 0x01020304);
    }

    #[test]
    fn count_and_entry_variants() {
        // S5: count 03, value 0x0007; sum = 3+0+7 = 0x0A -> 0xF5
        let rs = parse("S5030007F5\n").unwrap();
        assert_eq!(rs[0].kind, Kind::Count);
        assert_eq!(rs[0].addr, 7);
        // S7: count 05, entry 0x00000000; sum = 5 -> 0xFA
        let rs = parse("S70500000000FA\n").unwrap();
        assert_eq!(rs[0].kind, Kind::Entry32);
        // S8: count 04, entry 0x000000; sum = 4 -> 0xFB
        let rs = parse("S804000000FB\n").unwrap();
        assert_eq!(rs[0].kind, Kind::Entry24);
    }

    #[test]
    fn kind_other_rejected_and_of() {
        assert_eq!(Kind::of(4), Kind::Other(4));
        assert_eq!(Kind::of(6), Kind::Other(6));
        assert_eq!(Kind::of(4).addr_len(), 0);
        assert_eq!(parse("S4030000F8\n"), None);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(parse(""), Some(Vec::new()));
        assert_eq!(parse("\n"), Some(Vec::new()));
        assert_eq!(parse("X00600004844521B\n"), None);
        assert_eq!(parse("S1062000010203D4\n"), None); // cksum off
        assert_eq!(parse("S105200001\n"), None); // truncated
        assert_eq!(parse("S1\n"), None);
        assert_eq!(parse("S1062G00010203D3\n"), None); // bad hex
        assert_eq!(parse("SA030000F8\n"), None);
    }
}

//! Endace ERF record stream (Extensible Record Format): each record
//! is a 16-byte header — `ts` u64, `type` u8, `flags` u8, `rlen` u16,
//! `lctr` u16, `wlen` u16 (all big-endian except `ts`, which is a
//! 64-bit fixed-point fraction also read BE). ERF has no magic; a
//! record is accepted when `type <= 0x1F`, `rlen >= 16` fits the
//! buffer, `wlen <= rlen - 16`, and every following record parses too.
//!
//! ```
//! // one Ethernet ERF record (type 2) carrying 4 bytes
//! let mut d = vec![0u8; 16];
//! d[8] = 0x02; // type
//! d[10..12].copy_from_slice(&[0, 20]); // rlen = 20
//! d[14..16].copy_from_slice(&[0, 4]); // wlen = 4
//! d.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
//! let e = izanagi_kit::erf::parse(&d).unwrap();
//! assert_eq!(e.records.len(), 1);
//! assert_eq!(e.records[0].wlen, 4);
//! ```

use std::vec::Vec;

/// One ERF record header.
#[derive(Clone, Debug)]
pub struct ErfRecord {
    /// 64-bit fixed-point timestamp (whole + fraction per ERF spec).
    pub ts: u64,
    /// Record type (0 = legacy, 2 = Ethernet, …).
    pub rec_type: u8,
    /// Flags byte (v=lctr validity, r/r/r, iface 2b, vlen).
    pub flags: u8,
    /// Total record length including this 16-byte header.
    pub rlen: u16,
    /// Loss counter.
    pub lctr: u16,
    /// Wire length of the packet.
    pub wlen: u16,
    /// Offset of the record's payload inside the input.
    pub payload_offset: usize,
}

/// A parsed ERF record stream.
#[derive(Clone, Debug)]
pub struct Erf {
    /// Records in order.
    pub records: Vec<ErfRecord>,
}

fn u16be(d: &[u8], o: usize) -> Option<u16> {
    Some((u16::from(*d.get(o)?) << 8) | u16::from(*d.get(o + 1)?))
}
fn u64be(d: &[u8], o: usize) -> Option<u64> {
    let d = d.get(o..o + 8)?;
    let mut v = 0u64;
    for &b in d {
        v = v << 8 | u64::from(b);
    }
    Some(v)
}

/// Parse an ERF stream; `None` when any record header is implausible
/// (type > 0x1F, rlen < 16 or overflowing, wlen beyond rlen).
pub fn parse(d: &[u8]) -> Option<Erf> {
    if d.is_empty() {
        return None;
    }
    let mut records = Vec::new();
    let mut off = 0usize;
    while off < d.len() {
        if d.len() - off < 16 {
            return None;
        }
        let rec_type = d[off + 8];
        // ERF types 0..=0x1F are defined; pad records are type 13.
        if rec_type > 0x1F {
            return None;
        }
        let rlen = u16be(d, off + 10)? as usize;
        let wlen = u16be(d, off + 14)? as usize;
        if rlen < 16 || rlen > d.len() - off || wlen > rlen - 16 {
            return None;
        }
        records.push(ErfRecord {
            ts: u64be(d, off)?,
            rec_type,
            flags: d[off + 9],
            rlen: rlen as u16,
            lctr: u16be(d, off + 12)?,
            wlen: wlen as u16,
            payload_offset: off + 16,
        });
        off += rlen;
    }
    Some(Erf { records })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(ty: u8, rlen: u16, wlen: u16) -> Vec<u8> {
        let mut v = vec![0u8; 16];
        v[8] = ty;
        v[10..12].copy_from_slice(&[(rlen >> 8) as u8, rlen as u8]);
        v[14..16].copy_from_slice(&[(wlen >> 8) as u8, wlen as u8]);
        v
    }

    #[test]
    fn chain() {
        let mut d = rec(2, 20, 4);
        d.extend_from_slice(&[1, 2, 3, 4]);
        d.extend_from_slice(&rec(13, 16, 0)); // pad record, wlen 0
        let e = parse(&d).unwrap();
        assert_eq!(e.records.len(), 2);
        assert_eq!(e.records[0].rec_type, 2);
        assert_eq!(e.records[1].rec_type, 13);
        assert_eq!(e.records[0].payload_offset, 16);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        // bad type
        let mut d = rec(0x30, 16, 0);
        assert!(parse(&d).is_none());
        // rlen beyond buffer
        d = rec(2, 100, 0);
        assert!(parse(&d).is_none());
        // wlen beyond rlen
        d = rec(2, 20, 32);
        assert!(parse(&d).is_none());
    }
}

//! libpcap capture files (the classic `.pcap`, not pcapng): a
//! 24-byte global header — magic (micro- or nano-second resolution,
//! either byte order), version, `thiszone`/`sigfigs`, `snaplen`,
//! `network` linktype — then per-packet records of
//! `ts_sec, ts_frac, incl_len, orig_len` + data. [`parse`] validates
//! and collects record metadata; [`Pcap::packet`] returns the packet
//! bytes. Timestamps are normalized to nanoseconds in
//! [`Rec::ts_ns`].
//!
//! ```
//! use izanagi_kit::pcap::parse;
//! // LE us-resolution file, linktype 1 (Ethernet), one 4-byte packet
//! let mut d = vec![0xD4, 0xC3, 0xB2, 0xA1, 2, 0, 4, 0];
//! d.extend_from_slice(&[0; 8]);              // thiszone + sigfigs
//! d.extend_from_slice(&[0xFF, 0xFF, 0, 0]);  // snaplen
//! d.extend_from_slice(&[1, 0, 0, 0]);        // Ethernet
//! d.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]); // ts = 1.0s
//! d.extend_from_slice(&[4, 0, 0, 0, 4, 0, 0, 0]); // incl = orig = 4
//! d.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
//! let p = parse(&d).unwrap();
//! assert_eq!(p.records[0].ts_ns, 1_000_000_000);
//! assert_eq!(p.packet(0).unwrap(), &[0xDE, 0xAD, 0xBE, 0xEF]);
//! ```

use std::vec::Vec;

/// One captured record's metadata; packet bytes stay in the input.
#[derive(Clone, Debug, PartialEq)]
pub struct Rec {
    /// Timestamp in nanoseconds (normalised from sec+frac using the
    /// file's resolution).
    pub ts_ns: i64,
    /// Captured byte count (`incl_len`).
    pub incl: u32,
    /// On-wire byte count (`orig_len`).
    pub orig: u32,
    /// Byte offset of the packet data inside the input.
    pub data_at: usize,
}

/// A parsed capture file.
#[derive(Clone, Debug, PartialEq)]
pub struct Pcap {
    /// True when the file is little-endian.
    pub le: bool,
    /// Link-layer type (`network` field: 1 Ethernet, 101 raw IP…).
    pub link: u32,
    /// Snapshot length limit.
    pub snaplen: u32,
    /// Packet records in file order.
    pub records: Vec<Rec>,
    /// The packet-data region: records index into this slice's parent
    /// buffer via `data_at`, so `packet` needs the original bytes.
    data: Vec<u8>,
}

fn u32e(le: bool, d: &[u8], i: usize) -> Option<u32> {
    let mut v = 0u32;
    let bs = [*d.get(i)?, *d.get(i + 1)?, *d.get(i + 2)?, *d.get(i + 3)?];
    for (k, &b) in bs.iter().enumerate() {
        let sh = if le { k * 8 } else { (3 - k) * 8 };
        v |= (b as u32) << sh;
    }
    Some(v)
}

/// Parse a pcap file. `None` on a bad magic, truncated header, or a
/// record whose `incl_len` overruns the file (a truncated final
/// record drops it rather than failing the whole capture — common
/// for files cut mid-write).
pub fn parse(d: &[u8]) -> Option<Pcap> {
    let magic = d.get(..4)?;
    let (le, nano) = match magic {
        [0xD4, 0xC3, 0xB2, 0xA1] => (true, false),
        [0xA1, 0xB2, 0xC3, 0xD4] => (false, false),
        [0x4D, 0x3C, 0xB2, 0xA1] => (true, true),
        [0xA1, 0xB2, 0x3C, 0x4D] => (false, true),
        _ => return None,
    };
    if d.len() < 24 {
        return None;
    }
    // version: two u16s — accept 2.x only
    let ver_major = {
        let a = *d.get(4)? as u16;
        let b = *d.get(5)? as u16;
        if le {
            a | (b << 8)
        } else {
            (a << 8) | b
        }
    };
    if ver_major != 2 {
        return None;
    }
    let snaplen = u32e(le, d, 16)?;
    let link = u32e(le, d, 20)?;
    let mut records = Vec::new();
    let mut i = 24;
    while i + 16 <= d.len() {
        let ts_sec = u32e(le, d, i)? as i64;
        let ts_frac = u32e(le, d, i + 4)? as i64;
        let incl = u32e(le, d, i + 8)?;
        let orig = u32e(le, d, i + 12)?;
        let data_at = i + 16;
        if data_at.checked_add(incl as usize)? > d.len() {
            break; // truncated tail record — drop it, keep the rest
        }
        let frac_ns = if nano { ts_frac } else { ts_frac * 1000 };
        records.push(Rec {
            ts_ns: ts_sec.checked_mul(1_000_000_000)?.checked_add(frac_ns)?,
            incl,
            orig,
            data_at,
        });
        i = data_at + incl as usize;
    }
    Some(Pcap {
        le,
        link,
        snaplen,
        records,
        data: d.to_vec(),
    })
}

impl Pcap {
    /// Packet bytes of record `i`, or `None` when `i` is out of range.
    pub fn packet(&self, i: usize) -> Option<&[u8]> {
        let r = self.records.get(i)?;
        self.data.get(r.data_at..r.data_at + r.incl as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(le: bool, nano: bool, link: u32, recs: &[(u32, u32, &[u8])]) -> Vec<u8> {
        let mut d = match (le, nano) {
            (true, false) => vec![0xD4, 0xC3, 0xB2, 0xA1],
            (false, false) => vec![0xA1, 0xB2, 0xC3, 0xD4],
            (true, true) => vec![0x4D, 0x3C, 0xB2, 0xA1],
            (false, true) => vec![0xA1, 0xB2, 0x3C, 0x4D],
        };
        let w32 = |d: &mut Vec<u8>, v: u32| {
            for k in 0..4 {
                let sh = if le { k * 8 } else { (3 - k) * 8 };
                d.push((v >> sh) as u8);
            }
        };
        // version 2.4 = major u16 then minor u16, in file byte order
        if le {
            d.extend_from_slice(&[2, 0, 4, 0]);
        } else {
            d.extend_from_slice(&[0, 2, 0, 4]);
        }
        w32(&mut d, 0); // thiszone
        w32(&mut d, 0); // sigfigs
        w32(&mut d, 65535); // snaplen
        w32(&mut d, link);
        for &(s, f, pkt) in recs {
            w32(&mut d, s);
            w32(&mut d, f);
            w32(&mut d, pkt.len() as u32);
            w32(&mut d, pkt.len() as u32);
            d.extend_from_slice(pkt);
        }
        d
    }

    #[test]
    fn le_records() {
        let d = file(true, false, 1, &[(1, 500_000, &[1, 2, 3]), (2, 0, &[4])]);
        let p = parse(&d).unwrap();
        assert!(p.le);
        assert_eq!(p.link, 1);
        assert_eq!(p.records.len(), 2);
        assert_eq!(p.records[0].ts_ns, 1_500_000_000);
        assert_eq!(p.packet(0).unwrap(), &[1, 2, 3]);
        assert_eq!(p.packet(1).unwrap(), &[4]);
        assert!(p.packet(2).is_none());
    }

    #[test]
    fn be_and_nano() {
        let d = file(false, true, 101, &[(0, 42, &[9])]);
        let p = parse(&d).unwrap();
        assert!(!p.le);
        assert_eq!(p.link, 101);
        assert_eq!(p.records[0].ts_ns, 42); // nano: frac used directly
    }

    #[test]
    fn truncated_tail_dropped() {
        let mut d = file(true, false, 1, &[(1, 0, &[7])]);
        // append a record header claiming 10 bytes with only 2 present
        d.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 8, 8]);
        let p = parse(&d).unwrap();
        assert_eq!(p.records.len(), 1);
    }

    #[test]
    fn bad_inputs() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0xD4, 0xC3, 0xB2]).is_none());
        assert!(parse(&[0x00, 0x00, 0x00, 0x00]).is_none());
        // version 3 rejected
        let mut d = file(true, false, 1, &[]);
        d[4] = 3;
        d[6] = 0;
        assert!(parse(&d).is_none());
    }

    #[test]
    fn determinism() {
        let d = file(true, false, 1, &[(0, 0, &[1])]);
        assert_eq!(parse(&d), parse(&d));
    }
}

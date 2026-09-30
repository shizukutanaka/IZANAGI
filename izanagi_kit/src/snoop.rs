//! snoop packet capture (RFC 1761): `"snoop\0\0\0"` magic, u32 BE
//! `version` + `datalink`, then records of 24-byte headers
//! (`orig_len, incl_len, rec_len, drops, ts_sec, ts_usec`, all u32
//! BE). `rec_len` must cover the 24-byte header + captured bytes
//! padded to a multiple of 4.
//!
//! ```
//! let mut d = b"snoop\0\0\0".to_vec();
//! d.extend_from_slice(&[0,0,0,2, 0,0,0,4]); // v2, Ethernet
//! d.extend_from_slice(&[0,0,0,8, 0,0,0,8, 0,0,0,32, 0,0,0,0,
//!                       0,0,0,10, 0,0,0,5]); // rec hdr
//! d.extend_from_slice(&[1,2,3,4,5,6,7,8]); // 8B packet
//! let s = izanagi_kit::snoop::parse(&d).unwrap();
//! assert_eq!(s.link_type, 4);
//! assert_eq!(s.packets[0].cap_len, 8);
//! ```

use std::vec::Vec;

/// One captured packet record.
#[derive(Clone, Debug)]
pub struct SnoopPkt {
    /// Original on-wire length.
    pub orig_len: u32,
    /// Bytes actually captured.
    pub cap_len: u32,
    /// Cumulative drops reported at this record.
    pub drops: u32,
    /// Seconds since the epoch.
    pub secs: u32,
    /// Microseconds part.
    pub usecs: u32,
    /// Offset of the packet bytes inside the input.
    pub data_offset: usize,
}

/// A parsed snoop capture file.
#[derive(Clone, Debug)]
pub struct Snoop {
    /// File format version (spec says 2).
    pub version: u32,
    /// Datalink type (4 = Ethernet, 8 = PPP, …).
    pub link_type: u32,
    /// Packet records in order.
    pub packets: Vec<SnoopPkt>,
}

fn u32be(d: &[u8], o: usize) -> Option<u32> {
    let d = d.get(o..o + 4)?;
    Some(
        (u32::from(d[0]) << 24)
            | (u32::from(d[1]) << 16)
            | (u32::from(d[2]) << 8)
            | u32::from(d[3]),
    )
}

/// Parse a snoop capture; `None` on bad magic, wrong version, or a
/// record whose `rec_len` doesn't fit the buffer.
pub fn parse(d: &[u8]) -> Option<Snoop> {
    if d.len() < 16 || &d[..8] != b"snoop\0\0\0" {
        return None;
    }
    let version = u32be(d, 8)?;
    if version != 2 {
        return None;
    }
    let link_type = u32be(d, 12)?;
    let mut packets = Vec::new();
    let mut off = 16usize;
    while off < d.len() {
        let orig_len = u32be(d, off)?;
        let cap_len = u32be(d, off + 4)?;
        let rec_len = u32be(d, off + 8)? as usize;
        let drops = u32be(d, off + 12)?;
        let secs = u32be(d, off + 16)?;
        let usecs = u32be(d, off + 20)?;
        if rec_len < 24 || orig_len < cap_len {
            return None;
        }
        if rec_len > d.len() - off {
            return None;
        }
        // rec_len is 24 + cap_len padded to 4; enforce.
        let want = 24 + (cap_len as usize).div_ceil(4) * 4;
        if rec_len != want {
            return None;
        }
        packets.push(SnoopPkt {
            orig_len,
            cap_len,
            drops,
            secs,
            usecs,
            data_offset: off + 24,
        });
        off += rec_len;
    }
    Some(Snoop {
        version,
        link_type,
        packets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(orig: u32, cap: u32, rec: u32) -> Vec<u8> {
        let mut v = Vec::new();
        for x in [orig, cap, rec, 0u32, 100u32, 7u32] {
            v.extend_from_slice(&[(x >> 24) as u8, (x >> 16) as u8, (x >> 8) as u8, x as u8]);
        }
        v
    }

    #[test]
    fn two_packets() {
        let mut d = b"snoop\0\0\0".to_vec();
        d.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 4]);
        d.extend_from_slice(&hdr(4, 4, 28));
        d.extend_from_slice(&[9, 9, 9, 9]);
        d.extend_from_slice(&hdr(6, 6, 32));
        d.extend_from_slice(&[1, 2, 3, 4, 5, 6, 0, 0]); // 6B + 2 pad
        let s = parse(&d).unwrap();
        assert_eq!(s.packets.len(), 2);
        assert_eq!(s.packets[1].cap_len, 6);
        assert_eq!(s.packets[0].data_offset, 16 + 24);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"snoop\0\0\0\x00\x00\x00\x01\x00\x00\x00\x04").is_none()); // v1
                                                                                  // rec_len not covering cap_len
        let mut d = b"snoop\0\0\0".to_vec();
        d.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 4]);
        d.extend_from_slice(&hdr(8, 8, 24)); // rec_len too small
        assert!(parse(&d).is_none());
    }
}

//! Bluetooth HCI snoop log (BTSnoop): `"btsnoop\0"` magic, u32 BE
//! `version` = 1, u32 `datalink` (1001 = HCI UART H4, 1002 = BCSP,
//! 1003 = BCP, 1004 = H5, 2001 = Linux monitor), then per-packet
//! 24-byte headers: `orig_len`, `cap_len`, `flags`, `drops`,
//! `ts u64` (µs since 0 A.D. — subtract `0x00dcddb30f2f8000` for a
//! Unix µs timestamp). `flags` bit 0: 1 = packet sent by host.
//!
//! ```
//! let mut d = b"btsnoop\0".to_vec();
//! d.extend_from_slice(&[0,0,0,1, 0,0,3,233]); // v1 + type 1001
//! d.extend_from_slice(&[0,0,0,4, 0,0,0,4, 0,0,0,1, 0,0,0,0]);
//! d.extend_from_slice(&[0,0,0,0, 0,0,0,1]);
//! d.extend_from_slice(&[1,2,3,4]);
//! let b = izanagi_kit::btsnoop::parse(&d).unwrap();
//! assert_eq!(b.link_type, 1001);
//! assert!(b.packets[0].host_sent);
//! ```

use std::vec::Vec;

/// Epoch offset between 0 A.D. and 1970 in microseconds.
pub const UNIX_EPOCH_DELTA_US: u64 = 0x00dc_ddb3_0f2f_8000;

/// One HCI packet record.
#[derive(Clone, Debug)]
pub struct BtPkt {
    /// Original packet length.
    pub orig_len: u32,
    /// Captured length.
    pub cap_len: u32,
    /// `true` when the packet was sent by the host (flags bit 0).
    pub host_sent: bool,
    /// `true` when the packet is a data packet (flags bit 1) rather
    /// than command/event.
    pub data_pkt: bool,
    /// Dropped-packet counter.
    pub drops: u32,
    /// Timestamp, µs since 0 A.D.
    pub ts_us: u64,
    /// Offset of the packet bytes inside the input.
    pub data_offset: usize,
}

/// A parsed BTSnoop log.
#[derive(Clone, Debug)]
pub struct Btsnoop {
    /// Format version (1).
    pub version: u32,
    /// HCI transport type (1001/1002/1003/1004/2001).
    pub link_type: u32,
    /// Packet records in order.
    pub packets: Vec<BtPkt>,
}

/// Convert a BTSnoop timestamp (µs since 0 A.D.) to µs since the Unix
/// epoch; `None` if before 1970.
pub fn unix_us(ts: u64) -> Option<u64> {
    ts.checked_sub(UNIX_EPOCH_DELTA_US)
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
fn u64be(d: &[u8], o: usize) -> Option<u64> {
    let d = d.get(o..o + 8)?;
    let mut v = 0u64;
    for &b in d {
        v = v << 8 | u64::from(b);
    }
    Some(v)
}

/// Parse a BTSnoop file; `None` on bad magic/version or a record that
/// doesn't fit.
pub fn parse(d: &[u8]) -> Option<Btsnoop> {
    if d.len() < 16 || &d[..8] != b"btsnoop\0" {
        return None;
    }
    let version = u32be(d, 8)?;
    if version != 1 {
        return None;
    }
    let link_type = u32be(d, 12)?;
    if !matches!(link_type, 1001 | 1002 | 1003 | 1004 | 2001) {
        return None;
    }
    let mut packets = Vec::new();
    let mut off = 16usize;
    while off < d.len() {
        let orig_len = u32be(d, off)?;
        let cap_len = u32be(d, off + 4)?;
        let flags = u32be(d, off + 8)?;
        let drops = u32be(d, off + 12)?;
        let ts_us = u64be(d, off + 16)?;
        if orig_len < cap_len || cap_len as usize > d.len() - off - 24 {
            return None;
        }
        packets.push(BtPkt {
            orig_len,
            cap_len,
            host_sent: flags & 1 != 0,
            data_pkt: flags & 2 != 0,
            drops,
            ts_us,
            data_offset: off + 24,
        });
        off += 24 + cap_len as usize;
    }
    Some(Btsnoop {
        version,
        link_type,
        packets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt(orig: u32, cap: u32, flags: u32) -> Vec<u8> {
        let mut v = Vec::new();
        for x in [orig, cap, flags, 0u32] {
            v.extend_from_slice(&[(x >> 24) as u8, (x >> 16) as u8, (x >> 8) as u8, x as u8]);
        }
        v.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 9]); // ts
        v
    }

    #[test]
    fn basic() {
        let mut d = b"btsnoop\0".to_vec();
        d.extend_from_slice(&[0, 0, 0, 1, 0, 0, 3, 233]); // 1001
        d.extend_from_slice(&pkt(4, 4, 1));
        d.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
        let b = parse(&d).unwrap();
        assert_eq!(b.packets.len(), 1);
        assert_eq!(b.packets[0].ts_us, 9);
        assert!(b.packets[0].host_sent);
        assert!(!b.packets[0].data_pkt);
        assert_eq!(b.packets[0].data_offset, 16 + 24);
        assert_eq!(unix_us(UNIX_EPOCH_DELTA_US + 5), Some(5));
        assert_eq!(unix_us(0), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"btsnoop\0\x00\x00\x00\x02\x00\x00\x03\xe9").is_none());
        let mut d = b"btsnoop\0".to_vec();
        d.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 7]); // bad link type
        assert!(parse(&d).is_none());
        let mut d = b"btsnoop\0".to_vec();
        d.extend_from_slice(&[0, 0, 0, 1, 0, 0, 3, 233]);
        d.extend_from_slice(&pkt(8, 8, 0));
        d.extend_from_slice(&[1, 2]); // cap_len beyond buffer
        assert!(parse(&d).is_none());
    }
}

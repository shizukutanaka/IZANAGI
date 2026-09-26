//! Ethernet II / IEEE 802-3 frame header.
//!
//! A frame starts with the 6-byte destination and source MAC
//! addresses followed by a 2-byte big-endian EtherType. Values at
//! or below 1500 denote an IEEE 802-3 length field instead. IEEE
//! 802-1Q VLAN tags (`0x8100`, `0x88A8` provider bridging, `0x9100`
//! QinQ) each add a TCI word and shift the inner EtherType; up to
//! two stacked tags are recognized.
//!
//! ```
//! use izanagi_kit::ethernet::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 4];
//! d[..6].copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
//! d[6..12].copy_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
//! d[12] = 0x08; d[13] = 0x00; // IPv4
//! let f = parse(&d).unwrap();
//! assert_eq!(f.dst[0], 0xaa);
//! assert_eq!(f.ether_type, 0x0800);
//! assert!(f.vlans.is_empty());
//! assert_eq!(&d[f.payload_at..], &[0, 0, 0, 0]);
//! ```

/// Untagged header size in bytes.
pub const HEADER: usize = 14;
/// EtherType: IPv4.
pub const TYPE_IPV4: u16 = 0x0800;
/// EtherType: ARP.
pub const TYPE_ARP: u16 = 0x0806;
/// EtherType: IPv6.
pub const TYPE_IPV6: u16 = 0x86DD;
/// VLAN tag protocol identifiers (802-1Q, provider, QinQ).
pub const VLAN_TPIDS: [u16; 3] = [0x8100, 0x88A8, 0x9100];

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}

/// One stacked VLAN tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vlan {
    /// Tag protocol identifier actually seen on the wire.
    pub tpid: u16,
    /// Tag control information: PCP(3) DEI(1) VID(12).
    pub tci: u16,
}

impl Vlan {
    /// 12-bit VLAN identifier.
    pub fn vid(&self) -> u16 {
        self.tci & 0x0fff
    }
}

/// A parsed Ethernet frame header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ethernet {
    /// Destination MAC address.
    pub dst: [u8; 6],
    /// Source MAC address.
    pub src: [u8; 6],
    /// Innermost EtherType (or the 802-3 length when <= 1500).
    pub ether_type: u16,
    /// Stacked VLAN tags, outermost first.
    pub vlans: Vec<Vlan>,
    /// True when the type field carries an 802-3 length (<= 1500).
    pub is_length: bool,
    /// Offset of the payload after header and tags.
    pub payload_at: usize,
}

/// Parse a frame header. Returns `None` when the frame is shorter
/// than 14 bytes or a VLAN tag chain is truncated.
pub fn parse(d: &[u8]) -> Option<Ethernet> {
    if d.len() < HEADER {
        return None;
    }
    let mut dst = [0u8; 6];
    let mut src = [0u8; 6];
    dst.copy_from_slice(d.get(..6)?);
    src.copy_from_slice(d.get(6..12)?);
    let mut at = 12;
    let mut vlans = Vec::new();
    let mut ether_type = be16(d, at)?;
    while VLAN_TPIDS.contains(&ether_type) {
        if vlans.len() >= 2 {
            return None;
        }
        vlans.push(Vlan {
            tpid: ether_type,
            tci: be16(d, at + 2)?,
        });
        at += 4;
        ether_type = be16(d, at)?;
    }
    Some(Ethernet {
        dst,
        src,
        ether_type,
        vlans,
        is_length: ether_type <= 1500,
        payload_at: at + 2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn frame(ether_type: u16) -> Vec<u8> {
        let mut d = vec![0u8; 64];
        for i in 0..6 {
            d[i] = 0x10 + i as u8;
            d[6 + i] = 0x20 + i as u8;
        }
        d[12] = (ether_type >> 8) as u8;
        d[13] = ether_type as u8;
        d
    }

    #[test]
    fn plain_ipv4_frame() {
        let d = frame(TYPE_IPV4);
        let f = parse(&d).unwrap();
        assert_eq!(f.dst, [0x10, 0x11, 0x12, 0x13, 0x14, 0x15]);
        assert_eq!(f.src, [0x20, 0x21, 0x22, 0x23, 0x24, 0x25]);
        assert_eq!(f.ether_type, TYPE_IPV4);
        assert!(!f.is_length);
        assert_eq!(f.payload_at, HEADER);
    }

    #[test]
    fn vlan_tag_shifts_inner_type() {
        let mut d = frame(0x8100);
        d[14] = 0xa0;
        d[15] = 0x2b; // PCP 5, VID 43
        d[16] = 0x86;
        d[17] = 0xdd;
        let f = parse(&d).unwrap();
        assert_eq!(f.vlans.len(), 1);
        assert_eq!(f.vlans[0].tpid, 0x8100);
        assert_eq!(f.vlans[0].vid(), 43);
        assert_eq!(f.ether_type, TYPE_IPV6);
        assert_eq!(f.payload_at, 18);
    }

    #[test]
    fn qinq_stacks_two_tags() {
        let mut d = frame(0x88a8);
        d[14] = 0;
        d[15] = 1;
        d[16] = 0x81;
        d[17] = 0x00;
        d[18] = 0;
        d[19] = 7;
        d[20] = 0x08;
        d[21] = 0x00;
        let f = parse(&d).unwrap();
        assert_eq!(f.vlans.len(), 2);
        assert_eq!(f.vlans[1].vid(), 7);
        assert_eq!(f.ether_type, TYPE_IPV4);
    }

    #[test]
    fn length_field_is_not_ethertype() {
        let d = frame(64);
        let f = parse(&d).unwrap();
        assert!(f.is_length);
        assert_eq!(f.ether_type, 64);
    }

    #[test]
    fn truncations_reject() {
        assert!(parse(&[0u8; 13]).is_none());
        let mut d = frame(0x8100);
        d.truncate(15); // tag header cut off
        assert!(parse(&d).is_none());
    }
}

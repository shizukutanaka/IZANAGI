//! ARP packet (RFC 826).
//!
//! Hardware type, protocol type and the hardware/protocol address
//! lengths make the packet size generic: sender hardware address,
//! sender protocol address, target hardware address and target
//! protocol address follow the 8-byte fixed part, each sized by
//! `hlen`/`plen`. Ethernet+IPv4 uses `hlen` 6 / `plen` 4 and
//! operation 1 (request) or 2 (reply).
//!
//! ```
//! use izanagi_kit::arp::{parse, Op, HTYPE_ETHERNET, PTYPE_IPV4};
//!
//! let mut d = vec![0u8; 28];
//! d[1] = 1;          // htype = ethernet
//! d[2] = 0x08;       // ptype = IPv4
//! d[4] = 6; d[5] = 4; // hlen, plen
//! d[7] = 1;          // request
//! d[8..14].copy_from_slice(&[0xaa; 6]);
//! d[14..18].copy_from_slice(&[192, 168, 1, 1]);
//! d[24..28].copy_from_slice(&[192, 168, 1, 2]);
//! let a = parse(&d).unwrap();
//! assert_eq!(a.htype, HTYPE_ETHERNET);
//! assert_eq!(a.ptype, PTYPE_IPV4);
//! assert_eq!(a.op(), Op::Request);
//! assert_eq!(a.sender_ipv4(), Some([192, 168, 1, 1]));
//! assert_eq!(a.target_ipv4(), Some([192, 168, 1, 2]));
//! ```

/// Hardware type: Ethernet (10 Mb).
pub const HTYPE_ETHERNET: u16 = 1;
/// Protocol type: IPv4.
pub const PTYPE_IPV4: u16 = 0x0800;

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}

/// ARP operation codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// 1 — who-has request.
    Request,
    /// 2 — reply.
    Reply,
    /// RARP request/reply and other codes.
    Other(u16),
}

/// A parsed ARP packet. Address slices borrow the packet buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arp<'a> {
    /// Hardware address space (1 = Ethernet).
    pub htype: u16,
    /// Protocol address space (0x0800 = IPv4).
    pub ptype: u16,
    /// Hardware address length.
    pub hlen: u8,
    /// Protocol address length.
    pub plen: u8,
    /// Operation code.
    pub oper: u16,
    /// Sender hardware address (`hlen` bytes).
    pub sha: &'a [u8],
    /// Sender protocol address (`plen` bytes).
    pub spa: &'a [u8],
    /// Target hardware address (`hlen` bytes).
    pub tha: &'a [u8],
    /// Target protocol address (`plen` bytes).
    pub tpa: &'a [u8],
}

impl Arp<'_> {
    /// Classified operation code.
    pub fn op(&self) -> Op {
        match self.oper {
            1 => Op::Request,
            2 => Op::Reply,
            o => Op::Other(o),
        }
    }
    /// Sender protocol address as IPv4 octets (plen 4), else `None`.
    pub fn sender_ipv4(&self) -> Option<[u8; 4]> {
        if self.plen == 4 {
            let a: &[u8; 4] = self.spa.try_into().ok()?;
            Some(*a)
        } else {
            None
        }
    }
    /// Target protocol address as IPv4 octets (plen 4), else `None`.
    pub fn target_ipv4(&self) -> Option<[u8; 4]> {
        if self.plen == 4 {
            let a: &[u8; 4] = self.tpa.try_into().ok()?;
            Some(*a)
        } else {
            None
        }
    }
    /// Sender hardware address as MAC octets (hlen 6), else `None`.
    pub fn sender_mac(&self) -> Option<[u8; 6]> {
        if self.hlen == 6 {
            let a: &[u8; 6] = self.sha.try_into().ok()?;
            Some(*a)
        } else {
            None
        }
    }
    /// Target hardware address as MAC octets (hlen 6), else `None`.
    pub fn target_mac(&self) -> Option<[u8; 6]> {
        if self.hlen == 6 {
            let a: &[u8; 6] = self.tha.try_into().ok()?;
            Some(*a)
        } else {
            None
        }
    }
}

/// Parse a packet. Returns `None` when the buffer is shorter than
/// the address-length-dependent packet size.
pub fn parse(d: &[u8]) -> Option<Arp<'_>> {
    if d.len() < 8 {
        return None;
    }
    let hlen = usize::from(d.get(4).copied()?);
    let plen = usize::from(d.get(5).copied()?);
    let need = 8usize
        .checked_add(hlen.checked_mul(2)?)?
        .checked_add(plen.checked_mul(2)?)?;
    if d.len() < need {
        return None;
    }
    let sha_at = 8;
    let spa_at = sha_at + hlen;
    let tha_at = spa_at + plen;
    let tpa_at = tha_at + hlen;
    Some(Arp {
        htype: be16(d, 0)?,
        ptype: be16(d, 2)?,
        hlen: hlen as u8,
        plen: plen as u8,
        oper: be16(d, 6)?,
        sha: d.get(sha_at..spa_at)?,
        spa: d.get(spa_at..tha_at)?,
        tha: d.get(tha_at..tpa_at)?,
        tpa: d.get(tpa_at..need)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 28];
        d[1] = 1;
        d[2] = 0x08;
        d[3] = 0x00;
        d[4] = 6;
        d[5] = 4;
        d[7] = 2; // reply
        d[8..14].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0x00, 0x01]);
        d[14..18].copy_from_slice(&[10, 1, 2, 3]);
        d[18..24].copy_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        d[24..28].copy_from_slice(&[10, 4, 5, 6]);
        d
    }

    #[test]
    fn reply_fields() {
        let d = fixture();
        let a = parse(&d).unwrap();
        assert_eq!(a.op(), Op::Reply);
        assert_eq!(a.sha, &[0xde, 0xad, 0xbe, 0xef, 0x00, 0x01]);
        assert_eq!(a.sender_mac(), Some([0xde, 0xad, 0xbe, 0xef, 0x00, 0x01]));
        assert_eq!(a.target_mac(), Some([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]));
        assert_eq!(a.sender_ipv4(), Some([10, 1, 2, 3]));
        assert_eq!(a.target_ipv4(), Some([10, 4, 5, 6]));
    }

    #[test]
    fn odd_lengths_and_other_op() {
        let mut d = vec![0u8; 14]; // 8 fixed + 2*2 hw + 2*1 proto
        d[4] = 2;
        d[5] = 1;
        d[7] = 9;
        let a = parse(&d).unwrap();
        assert_eq!(a.op(), Op::Other(9));
        assert_eq!(a.sha.len(), 2);
        assert_eq!(a.tpa.len(), 1);
        assert!(a.sender_ipv4().is_none());
        assert!(a.sender_mac().is_none());
    }

    #[test]
    fn truncated_rejects() {
        let d = fixture();
        assert!(parse(&d[..20]).is_none());
        assert!(parse(&[0u8; 7]).is_none());
        // huge declared hlen must not overflow into a false accept
        let mut big = vec![0u8; 8];
        big[4] = 0xff;
        big[5] = 0xff;
        assert!(parse(&big).is_none());
    }
}

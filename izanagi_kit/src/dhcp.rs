//! Minimal reader for DHCP/BOOTP wire packets (RFC 2131/951): the fixed
//! 236-byte header (`op htype hlen hops` / `xid` / `secs` / `flags` /
//! `ciaddr` `yiaddr` `siaddr` `giaddr` / `chaddr` / `sname` / `file`)
//! followed by the `0x63825363` cookie and an RFC 2132 option TLV chain
//! (`code len value`, `0`=pad, `255`=end).
//!
//! ```
//! use izanagi_kit::dhcp::parse;
//!
//! let mut d = vec![0u8; 240];
//! d[0] = 1; // BOOTREQUEST
//! d[2] = 6; // hlen = 6 (Ethernet)
//! d[4..8].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // xid
//! d[236..240].copy_from_slice(&[0x63, 0x82, 0x53, 0x63]); // cookie
//! d.extend_from_slice(&[53, 1, 1, 255]); // msg-type DISCOVER, end
//! let p = parse(&d).unwrap();
//! assert_eq!(p.op, 1);
//! assert_eq!(p.xid, 0xDEADBEEF);
//! assert_eq!(p.options.first().map(|o| o.code), Some(53));
//! ```

/// One RFC 2132 option.
#[derive(Debug)]
pub struct Option_ {
    /// Option code (0=pad is skipped; 255=end terminates the scan).
    pub code: u8,
    /// Declared value length.
    pub len: u8,
    /// Byte offset of the option value.
    pub value_at: usize,
}

/// A parsed DHCP/BOOTP packet.
#[derive(Debug)]
pub struct Dhcp {
    /// `op` — 1 BOOTREQUEST, 2 BOOTREPLY.
    pub op: u8,
    /// `htype` — 1 = Ethernet.
    pub htype: u8,
    /// `hlen` — hardware address length.
    pub hlen: u8,
    /// `xid` — transaction ID (big-endian).
    pub xid: u32,
    /// `secs` — seconds since boot.
    pub secs: u16,
    /// `flags` — broadcast flag is bit 15.
    pub flags: u16,
    /// `yiaddr` — "your" (client) IPv4 address bytes.
    pub yiaddr: [u8; 4],
    /// `siaddr` — next-server IPv4 address bytes.
    pub siaddr: [u8; 4],
    /// `giaddr` — relay agent IPv4 address bytes.
    pub giaddr: [u8; 4],
    /// `chaddr` — client hardware address, first `hlen` bytes.
    pub chaddr: [u8; 16],
    /// Byte offset where the options section begins (240).
    pub options_at: usize,
    /// Parsed option headers.
    pub options: Vec<Option_>,
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    let b0 = *d.get(at)? as u32;
    let b1 = *d.get(at + 1)? as u32;
    let b2 = *d.get(at + 2)? as u32;
    let b3 = *d.get(at + 3)? as u32;
    Some((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    let hi = *d.get(at)? as u16;
    let lo = *d.get(at + 1)? as u16;
    Some((hi << 8) | lo)
}

/// Parse a DHCP packet. `None` when shorter than 240 bytes or the magic
/// cookie is missing; option scan is tolerant — a truncated option ends
/// the scan without failing the packet.
pub fn parse(d: &[u8]) -> Option<Dhcp> {
    if d.len() < 240 || &d[236..240] != b"\x63\x82\x53\x63" {
        return None;
    }
    let mut chaddr = [0u8; 16];
    chaddr.copy_from_slice(&d[28..44]);
    let mut options = Vec::new();
    let mut at = 240;
    loop {
        let code = *d.get(at)?;
        at += 1;
        if code == 0 {
            continue; // pad
        }
        if code == 255 {
            break; // end
        }
        let len = *d.get(at)? as usize;
        at += 1;
        options.push(Option_ {
            code,
            len: len as u8,
            value_at: at,
        });
        at += len;
        if at > d.len() {
            break; // trailing truncation: keep collected options
        }
    }
    Some(Dhcp {
        op: d[0],
        htype: d[1],
        hlen: d[2],
        xid: be32(d, 4)?,
        secs: be16(d, 8)?,
        flags: be16(d, 10)?,
        yiaddr: [d[16], d[17], d[18], d[19]],
        siaddr: [d[20], d[21], d[22], d[23]],
        giaddr: [d[24], d[25], d[26], d[27]],
        chaddr,
        options_at: 240,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> Vec<u8> {
        let mut d = vec![0u8; 240];
        d[0] = 1;
        d[1] = 1;
        d[2] = 6;
        d[4..8].copy_from_slice(&[0x00, 0x01, 0x02, 0x03]);
        d[16..20].copy_from_slice(&[192, 168, 1, 10]);
        d[28] = 0xAA; // chaddr[0]
        d[236..240].copy_from_slice(&[0x63, 0x82, 0x53, 0x63]);
        d.extend_from_slice(&[0, 53, 1, 1, 50, 4, 192, 168, 1, 99, 255]);
        d
    }

    #[test]
    fn parses() {
        let p = parse(&packet()).unwrap();
        assert_eq!(p.op, 1);
        assert_eq!(p.xid, 0x00010203);
        assert_eq!(p.yiaddr, [192, 168, 1, 10]);
        assert_eq!(p.chaddr[0], 0xAA);
        assert_eq!(p.options.len(), 2);
        assert_eq!(p.options[0].code, 53);
        assert_eq!(p.options[1].code, 50);
        assert_eq!(p.options[1].value_at, 246);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 239]).is_none()); // < fixed+cookie
        let mut bad = packet();
        bad[236] = 0;
        assert!(parse(&bad).is_none()); // bad cookie
    }
}

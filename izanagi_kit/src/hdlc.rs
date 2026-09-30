//! Cisco HDLC / ISO 3309 frame parser.
//!
//! Two shapes are recognised:
//!
//! * **Cisco HDLC** (point-to-point serial): 4-byte header
//!   `address ctrl proto:u16be` where `address` is `0x0F` (unicast) or
//!   `0x8F` (broadcast) and `ctrl` is `0x00`/`0x10`. Common `proto`
//!   values: `0x0800` IPv4, `0x8035` SLARP keepalive, `0x2000` Cisco
//!   discovery. `0x0F 00 81 4C` precedes STP-like BPDUs.
//! * **ISO 3309 flag-framed HDLC**: `0x7E` flags around
//!   `address* ctrl* info*` (bit0 of each address byte terminates the
//!   field); FCS is not verified.
//!
//! ```
//! let f = b"\x0f\x00\x08\x00\x45";
//! let h = izanagi_kit::hdlc::parse(f).unwrap();
//! assert_eq!(h.protocol, 0x0800);
//! assert_eq!(h.address, 0x0F);
//! ```

/// Which framing the input used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdlcKind {
    /// Cisco 4-byte header (`0x0F`/`0x8F` + ctrl + ethertype).
    Cisco,
    /// ISO 3309 `0x7E`-delimited frame (ABM/SABM style control byte).
    Iso,
}

/// Parsed HDLC frame summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Hdlc {
    /// Which framing matched.
    pub kind: HdlcKind,
    /// First address byte (`0x0F`/`0x8F` for Cisco frames).
    pub address: u8,
    /// Control byte (ISO: UI/SABM/etc.; Cisco canaries: 0x00/0x10).
    pub control: u8,
    /// Cisco ethertype field (`0` for ISO frames).
    pub protocol: u16,
    /// Information-field byte length (after the header, before FCS).
    pub info_len: usize,
    /// Header length consumed by the parser.
    pub header_len: usize,
}

/// Parse either HDLC flavour; `None` when neither shape fits.
pub fn parse(d: &[u8]) -> Option<Hdlc> {
    if let Some(h) = parse_cisco(d) {
        return Some(h);
    }
    parse_iso(d)
}

fn parse_cisco(d: &[u8]) -> Option<Hdlc> {
    if d.len() < 4 {
        return None;
    }
    let (a, c) = (d[0], d[1]);
    if (a != 0x0F && a != 0x8F) || (c != 0x00 && c != 0x10) {
        return None;
    }
    let protocol = ((d[2] as u16) << 8) | d[3] as u16;
    if protocol == 0 {
        return None;
    }
    Some(Hdlc {
        kind: HdlcKind::Cisco,
        address: a,
        control: c,
        protocol,
        info_len: d.len() - 4,
        header_len: 4,
    })
}

fn parse_iso(d: &[u8]) -> Option<Hdlc> {
    // `7E <addr...> <ctrl> <info>* 7E?` — every address byte carries an
    // extension bit; the last has bit0 set.
    if d.len() < 4 || d[0] != 0x7E {
        return None;
    }
    let mut i = 1usize;
    let mut addr_last = false;
    let mut first_addr = 0u8;
    while i < d.len() {
        let b = d[i];
        if i == 1 {
            first_addr = b;
        }
        i += 1;
        if b & 1 == 1 {
            addr_last = true;
            break;
        }
        if i - 1 > 6 {
            break; // address fields are ≤6 bytes in practice
        }
    }
    if !addr_last || i >= d.len() {
        return None;
    }
    let control = d[i];
    let info_len = d.len() - i - 1;
    let trailing_flag = d[d.len() - 1] == 0x7E;
    Some(Hdlc {
        kind: HdlcKind::Iso,
        address: first_addr,
        control,
        protocol: 0,
        info_len: if trailing_flag {
            info_len.saturating_sub(1)
        } else {
            info_len
        },
        header_len: i + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cisco() {
        let f = b"\x0f\x00\x08\x00ipv4";
        let h = parse(f).unwrap();
        assert_eq!(h.kind, HdlcKind::Cisco);
        assert_eq!(h.address, 0x0F);
        assert_eq!(h.protocol, 0x0800);
        assert_eq!(h.info_len, 4);
        // SLARP
        assert_eq!(parse(b"\x8f\x00\x80\x35s").unwrap().protocol, 0x8035);
    }

    #[test]
    fn iso() {
        // UI frame, broadcast address `03` (bit0 set = last byte), ctrl `03`
        let f = b"\x7e\x03\x03info\x7e";
        let h = parse(f).unwrap();
        assert_eq!(h.kind, HdlcKind::Iso);
        assert_eq!(h.address, 0x03);
        assert_eq!(h.control, 0x03);
        assert_eq!(h.info_len, 4);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x0f").is_none());
        assert!(parse(b"\x0f\x00\x00\x00").is_none());
        assert!(parse(b"\x7e").is_none());
        assert!(parse(b"\x7e\x00\x00").is_none()); // no address terminator
    }
}

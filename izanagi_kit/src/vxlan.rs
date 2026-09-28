//! VXLAN header parser (RFC 7348).
//!
//! A VXLAN packet on UDP/4789 is an 8-byte header followed by an inner
//! Ethernet frame: `flags:u8 reserved24 VNI:u24be reserved8` where only
//! bit 3 (`0x08`, the "I" flag) is defined — it must be set and the
//! reserved bytes must be zero. After the header, an inner Ethernet
//! frame starts with a destination MAC; multicast/broadcast MACs and
//! `FF:FF:FF:FF:FF:FF` are typical inner destinations.
//!
//! ```
//! let f = b"\x08\x00\x00\x00\x00\x00\x2a\x00\xff\xff\xff\xff\xff\xff";
//! let v = izanagi_kit::vxlan::parse(f).unwrap();
//! assert_eq!(v.vni, 42);
//! assert_eq!(v.header_len, 8);
//! ```

/// Parsed VXLAN header.
#[derive(Debug, Clone, PartialEq)]
pub struct Vxlan {
    /// 24-bit VXLAN Network Identifier.
    pub vni: u32,
    /// Header length in bytes (always 8 for RFC 7348).
    pub header_len: usize,
    /// Byte length of the encapsulated inner frame.
    pub inner_len: usize,
    /// Inner destination MAC when at least 6 bytes follow the header.
    pub inner_dst_mac: Option<[u8; 6]>,
    /// Whether the I flag is the only bit set (strict RFC 7348 form).
    pub strict_flags: bool,
}

/// Parse a VXLAN header; `None` when the I flag is clear, reserved
/// bytes are non-zero, or the buffer is shorter than 8 bytes.
pub fn parse(d: &[u8]) -> Option<Vxlan> {
    if d.len() < 8 {
        return None;
    }
    let flags = d[0];
    if flags & 0x08 == 0 {
        return None; // I flag must be set
    }
    if d[1] != 0 || d[2] != 0 || d[3] != 0 || d[7] != 0 {
        return None; // reserved fields must be zero
    }
    let vni = ((d[4] as u32) << 16) | ((d[5] as u32) << 8) | d[6] as u32;
    let inner = &d[8..];
    let mac = if inner.len() >= 6 {
        let mut m = [0u8; 6];
        m.copy_from_slice(&inner[..6]);
        Some(m)
    } else {
        None
    };
    Some(Vxlan {
        vni,
        header_len: 8,
        inner_len: inner.len(),
        inner_dst_mac: mac,
        strict_flags: flags == 0x08,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut f = b"\x08\x00\x00\x00\x00\x00\x2a\x00".to_vec();
        f.extend_from_slice(&[0xFF; 6]);
        f.extend_from_slice(&[0xAA; 6]);
        let v = parse(&f).unwrap();
        assert_eq!(v.vni, 42);
        assert_eq!(v.inner_len, 12);
        assert_eq!(v.inner_dst_mac, Some([0xFF; 6]));
        assert!(v.strict_flags);
    }

    #[test]
    fn header_only() {
        let v = parse(b"\x08\x00\x00\x00\x01\x02\x03\x00").unwrap();
        assert_eq!(v.vni, 0x010203);
        assert_eq!(v.inner_len, 0);
        assert!(v.inner_dst_mac.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x08\x00\x00\x00").is_none());
        assert!(parse(b"\x00\x00\x00\x00\x00\x00\x2a\x00").is_none()); // I=0
        assert!(parse(b"\x08\x00\x01\x00\x00\x00\x2a\x00").is_none()); // reserved
    }
}

//! IS-IS PDU parser (ISO/IEC 10589).
//!
//! On Ethernet an IS-IS PDU follows an 802.2 LLC header
//! `FE FE 03` (DSAP/SSAP `FE`, UI control) — this parser accepts the
//! PDU either at offset 0 or right after that 3-byte LLC prefix.
//!
//! Common fixed header (8 bytes): `0x83` (intradomain routing
//! discriminator), `length` indicator, `version/protocol extension`
//! (`1`), `id length` (0 = 6-byte system ids), `reserved/type` byte
//! whose low 5 bits are the PDU type, `version` (`1`), `reserved`
//! (`0`), `max area addresses` (usually 0 = 3).
//!
//! PDU types: 15 L1 LAN IIH, 16 L2 LAN IIH, 17 P2P IIH, 18 L1 LSP,
//! 20 L2 LSP, 24 L1 CSNP, 25 L2 CSNP, 26 L1 PSNP, 27 L2 PSNP.
//!
//! ```
//! let f = b"\xfe\xfe\x03\x83\x1b\x01\x00\x10\x01\x00\x00";
//! let i = izanagi_kit::isis::parse(f).unwrap();
//! assert_eq!(i.pdu_type, 16); // L2 LAN IIH
//! assert_eq!(i.name, "L2 LAN IIH");
//! ```

/// Known PDU type names by their 5-bit code.
pub fn type_name(code: u8) -> &'static str {
    match code {
        15 => "L1 LAN IIH",
        16 => "L2 LAN IIH",
        17 => "P2P IIH",
        18 => "L1 LSP",
        20 => "L2 LSP",
        24 => "L1 CSNP",
        25 => "L2 CSNP",
        26 => "L1 PSNP",
        27 => "L2 PSNP",
        _ => "unknown",
    }
}

/// Parsed IS-IS PDU header.
#[derive(Debug, Clone, PartialEq)]
pub struct Isis {
    /// 5-bit PDU type code.
    pub pdu_type: u8,
    /// Static name for the type code (`"unknown"` for unassigned).
    pub name: &'static str,
    /// System-ID length field (0 means the default 6 bytes).
    pub id_length: u8,
    /// Maximum area addresses field.
    pub max_area_addresses: u8,
    /// Whether the input carried the `FE FE 03` LLC prefix.
    pub has_llc: bool,
    /// Byte offset where the common header started.
    pub header_offset: usize,
}

/// Parse an IS-IS PDU; `None` without the `0x83` discriminator or a
/// version byte other than 1.
pub fn parse(d: &[u8]) -> Option<Isis> {
    let mut off = 0usize;
    let mut llc = false;
    if d.len() >= 3 && d[0] == 0xFE && d[1] == 0xFE && d[2] == 0x03 {
        off = 3;
        llc = true;
    }
    if d.len() < off + 8 {
        return None;
    }
    if d[off] != 0x83 {
        return None;
    }
    // d[off+1] = length indicator (informational)
    if d[off + 2] != 1 || d[off + 5] != 1 {
        return None; // version/proto-ext and version must be 1
    }
    let id_length = d[off + 3];
    let pdu_type = d[off + 4] & 0x1F;
    let max_area = d[off + 7];
    if pdu_type == 0 {
        return None;
    }
    Some(Isis {
        pdu_type,
        name: type_name(pdu_type),
        id_length,
        max_area_addresses: max_area,
        has_llc: llc,
        header_offset: off,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_llc() {
        let f = b"\xfe\xfe\x03\x83\x1b\x01\x00\x10\x01\x00\x00";
        let i = parse(f).unwrap();
        assert!(i.has_llc);
        assert_eq!(i.header_offset, 3);
        assert_eq!(i.pdu_type, 16);
        assert_eq!(i.name, "L2 LAN IIH");
        assert_eq!(i.id_length, 0);
        assert_eq!(i.max_area_addresses, 0);
    }

    #[test]
    fn bare_lsp() {
        let f = b"\x83\x1b\x01\x00\x12\x01\x00\x03";
        let i = parse(f).unwrap();
        assert!(!i.has_llc);
        assert_eq!(i.pdu_type, 18);
        assert_eq!(i.name, "L1 LSP");
        assert_eq!(i.max_area_addresses, 3);
    }

    #[test]
    fn names() {
        assert_eq!(type_name(15), "L1 LAN IIH");
        assert_eq!(type_name(17), "P2P IIH");
        assert_eq!(type_name(27), "L2 PSNP");
        assert_eq!(type_name(0), "unknown");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x84\x1b\x01\x00\x10\x01\x00\x00").is_none()); // discr
        assert!(parse(b"\x83\x1b\x02\x00\x10\x01\x00\x00").is_none()); // ver ext
        assert!(parse(b"\x83\x1b\x01\x00\x10\x02\x00\x00").is_none()); // version
        assert!(parse(b"\x83\x1b\x01\x00\x00\x01\x00\x00").is_none()); // type 0
    }
}

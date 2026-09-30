//! ATA over Ethernet (AoE) — 10-byte header, ethertype 0x88a2.
//!
//! ```
//! let d = [0x10, 0x00, 0x00, 0x12, 0x34, 0x00, 0x00, 0x01, 0x00, 0x01];
//! let a = izanagi_kit::aoe::parse(&d).unwrap();
//! assert_eq!(a.version, 1);
//! assert_eq!(a.command, izanagi_kit::aoe::AoeCmd::Issue);
//! assert!(izanagi_kit::aoe::detect(&d));
//! ```

/// AoE command codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AoeCmd {
    /// 0 — ATA command issue.
    Issue,
    /// 1 — query config information.
    QueryConfig,
    /// 2 — MAC mask list.
    MacMask,
    /// 3 — reserve/release.
    ReserveRelease,
    /// Other / reserved.
    Other(u8),
}

/// Parsed AoE header.
#[derive(Debug, Clone)]
pub struct Aoe {
    /// Version nibble (1).
    pub version: u8,
    /// `R` response flag.
    pub response: bool,
    /// `E` error flag.
    pub error: bool,
    /// Command code (byte 5).
    pub command: AoeCmd,
    /// Error field (byte 1 low nibble).
    pub error_code: u8,
    /// Major shelf address.
    pub major: u16,
    /// Minor slot address.
    pub minor: u8,
    /// Tag for request/response matching.
    pub tag: u32,
}

/// Detects an AoE header: version 1 in the high nibble, flags byte shape.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 10 && b[0] >> 4 == 1
}

/// Parses the AoE header; `None` under 10 bytes or non-v1.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Aoe> {
    if !detect(b) {
        return None;
    }
    let cmd = match b[5] {
        0 => AoeCmd::Issue,
        1 => AoeCmd::QueryConfig,
        2 => AoeCmd::MacMask,
        3 => AoeCmd::ReserveRelease,
        o => AoeCmd::Other(o),
    };
    Some(Aoe {
        version: b[0] >> 4,
        response: b[0] & 0x08 != 0,
        error: b[0] & 0x04 != 0,
        command: cmd,
        error_code: b[1] & 0x0f,
        major: ((b[2] as u16) << 8) | (b[3] as u16),
        minor: b[4],
        tag: ((b[6] as u32) << 24) | ((b[7] as u32) << 16) | ((b[8] as u32) << 8) | (b[9] as u32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = [0x10, 0x00, 0x00, 0x12, 0x34, 0x01, 0x00, 0x01, 0x00, 0x01];
        let f = parse(&d).unwrap();
        assert_eq!(f.major, 0x12);
        assert_eq!(f.minor, 0x34);
        assert_eq!(f.tag, 0x010001);
        assert!(!f.response && !f.error);
    }

    #[test]
    fn response_error() {
        let mut d = [0u8; 10];
        d[0] = 0x1c; // v1 + R + E
        d[1] = 0x02;
        d[5] = 0x01;
        let f = parse(&d).unwrap();
        assert!(f.response && f.error);
        assert_eq!(f.command, AoeCmd::QueryConfig);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x20; 10]).is_none());
    }
}

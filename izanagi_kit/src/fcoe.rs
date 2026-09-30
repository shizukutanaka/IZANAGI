//! Fibre Channel over Ethernet (FCoE, RFC 5120) + FIP frames.
//!
//! 14-byte header: `ver(4)|rsvd(4)|sof(8)|etype(16)|rsvd|fc_eof|rsvd|crc?`
//! Ethernet type 0x8906, SOF codes, EOF codes.
//!
//! ```
//! // ver=1, SOF=i2 (0x2d), etype=0x8906, EOF=N3 (0x41) at offset 12
//! let mut d = vec![0u8; 24];
//! d[0] = 0x10; d[1] = 0x2d;
//! d[2] = 0x89; d[3] = 0x06;
//! d[12] = 0x41;
//! let f = izanagi_kit::fcoe::parse(&d).unwrap();
//! assert_eq!(f.version, 1);
//! assert!(izanagi_kit::fcoe::detect(&d));
//! ```
use std::string::String;

/// Parsed FCoE frame header.
#[derive(Debug, Clone)]
pub struct Fcoe {
    /// Version nibble.
    pub version: u8,
    /// Start-of-Frame code.
    pub sof: u8,
    /// SOF name (`SOFi2`/`SOFi3`/`SOFn2`/`SOFn3`/`SOFc1`/`SOFf`).
    pub sof_name: String,
    /// End-of-Frame code (offset 12 when present).
    pub eof: u8,
    /// EOF name (`EOFn`/`EOFt`/`EOFa`/`EOFni`/`EOFdt`/`EOFdta`/`EOFrt`).
    pub eof_name: String,
    /// Whether a 4-byte encapsulated EOF+CRC trailer is present per header length.
    pub min_frame_ok: bool,
}

fn sof_name(c: u8) -> String {
    String::from(match c {
        0x28 => "SOFi1",
        0x2d => "SOFi2",
        0x2e => "SOFi3",
        0x29 => "SOFi4",
        0x35 => "SOFn2",
        0x36 => "SOFn3",
        0x39 => "SOFn4",
        0x58 => "SOFc1",
        0x41 => "SOFf",
        _ => "SOF?",
    })
}

fn eof_name(c: u8) -> String {
    String::from(match c {
        0x41 => "EOFn",
        0x42 => "EOFt",
        0x44 => "EOFa",
        0x49 => "EOFni",
        0x50 => "EOFdt",
        0x54 => "EOFdta",
        0x55 => "EOFrt",
        _ => "EOF?",
    })
}

/// Detects an FCoE frame: version nibble 1, ethertype 0x8906, plausible SOF.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 24
        && b[0] >> 4 == 1
        && b[2] == 0x89
        && b[3] == 0x06
        && matches!(b[1], 0x28 | 0x2d | 0x2e | 0x29 | 0x35 | 0x36 | 0x39 | 0x41)
}

/// Parses the FCoE header; `None` when `detect` fails.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Fcoe> {
    if !detect(b) {
        return None;
    }
    let eof = b.get(12).copied().unwrap_or(0);
    Some(Fcoe {
        version: b[0] >> 4,
        sof: b[1],
        sof_name: sof_name(b[1]),
        eof,
        eof_name: eof_name(eof),
        min_frame_ok: b.len() >= 24,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut d = vec![0u8; 32];
        d[0] = 0x10;
        d[1] = 0x35; // SOFn2
        d[2] = 0x89;
        d[3] = 0x06;
        d[12] = 0x42; // EOFt
        let f = parse(&d).unwrap();
        assert_eq!(f.sof_name, "SOFn2");
        assert_eq!(f.eof_name, "EOFt");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 24]).is_none());
        let mut d = vec![0u8; 24];
        d[0] = 0x10;
        d[2] = 0x89;
        d[3] = 0x05; // FIP type not FC
        assert!(parse(&d).is_none());
    }
}

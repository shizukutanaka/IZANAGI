//! iSCSI (RFC 7143) PDUs — 48-byte Basic Header Segment.
//!
//! ```
//! let mut d = vec![0u8; 48]; // NOP-Out
//! d[16] = 0xaa;
//! let p = izanagi_kit::iscsi::parse(&d).unwrap();
//! assert_eq!(p.opcode, 0);
//! assert!(izanagi_kit::iscsi::detect(&d));
//! ```
use std::string::String;

/// Parsed iSCSI Basic Header Segment.
#[derive(Debug, Clone)]
pub struct Iscsi {
    /// `true` when the immediate-delivery bit (0x40 of byte 0) is set.
    pub immediate: bool,
    /// Opcode (6 bits) with direction context.
    pub opcode: u8,
    /// Initiator-side opcode (`true` when 0x00..=0x1f range conventions apply).
    pub initiator: bool,
    /// Opcode class name.
    pub name: String,
    /// `F` final bit.
    pub fin: bool,
    /// Data-segment length (24-bit BE at 5..8).
    pub data_len: u32,
    /// LUN or target-transfer-tag word (8..16 first 8 bytes).
    pub lun_tag: [u8; 8],
    /// Task tag fields: initiator (16..20) / expected status (20..24) raw.
    pub itt_itt: u32,
    /// Additional header-segment length (byte 4).
    pub ahs_len: u8,
}

fn op_name(initiator: bool, op: u8) -> &'static str {
    if initiator {
        match op {
            0x00 => "NOP-Out",
            0x01 => "SCSI Command",
            0x02 => "SCSI Task Management",
            0x03 => "Login",
            0x04 => "Text",
            0x05 => "Data-Out",
            0x06 => "Logout",
            0x10 => "SNACK",
            0x1c..=0x1e => "vendor",
            _ => "reserved",
        }
    } else {
        match op {
            0x20 => "NOP-In",
            0x21 => "SCSI Response",
            0x22 => "Task Management Response",
            0x23 => "Login Response",
            0x24 => "Text Response",
            0x25 => "Data-In",
            0x26 => "Logout Response",
            0x31 => "R2T",
            0x32 => "Async",
            0x3c..=0x3e => "vendor",
            _ => "reserved",
        }
    }
}

/// Detects a plausible iSCSI BHS: 48 bytes min, valid opcode range, and the
/// declared frame length (AHS + data segment) must fit in the buffer. The
/// length bound is what rejects text: an ASCII `#` (0x23) reads as the
/// Login Response opcode and byte 4 as a plausible AHS size, but bytes 5..8
/// form a multi-megabyte data length that cannot fit.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 48 {
        return false;
    }
    let op = b[0] & 0x3f;
    let ok =
        (op <= 0x06) || op == 0x10 || (0x20..=0x26).contains(&op) || (0x31..=0x32).contains(&op);
    let ahs = (b[4] as usize) * 4;
    let data_len = ((b[5] as usize) << 16) | ((b[6] as usize) << 8) | (b[7] as usize);
    ok && 48 + ahs + data_len <= b.len() && (b[4] == 0 || b.len() >= 48 + 4)
}

/// Parses the 48-byte BHS; `None` when `detect` fails.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Iscsi> {
    if !detect(b) {
        return None;
    }
    let op = b[0] & 0x3f;
    let initiator = op <= 0x1f;
    let mut lun_tag = [0u8; 8];
    lun_tag.copy_from_slice(&b[8..16]);
    Some(Iscsi {
        immediate: b[0] & 0x40 != 0,
        opcode: op,
        initiator,
        name: String::from(op_name(initiator, op)),
        fin: b[1] & 0x80 != 0,
        data_len: ((b[5] as u32) << 16) | ((b[6] as u32) << 8) | (b[7] as u32),
        lun_tag,
        itt_itt: ((b[16] as u32) << 24)
            | ((b[17] as u32) << 16)
            | ((b[18] as u32) << 8)
            | (b[19] as u32),
        ahs_len: b[4],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut d = vec![0u8; 64];
        d[0] = 0x41; // immediate + SCSI Command
        d[1] = 0x87; // F + flags
        d[7] = 0x10; // 16-byte data segment (fits the 64-byte buffer)
        d[8] = 1;
        d[16] = 0xde;
        d[17] = 0xad;
        let f = parse(&d).unwrap();
        assert_eq!(f.opcode, 1);
        assert_eq!(f.name, "SCSI Command");
        assert!(f.immediate && f.fin && f.initiator);
        assert_eq!(f.data_len, 0x10);
        assert_eq!(f.itt_itt, 0xdead0000);
    }

    #[test]
    fn response_side() {
        let mut d = vec![0u8; 48];
        d[0] = 0x25; // Data-In
        let f = parse(&d).unwrap();
        assert!(!f.initiator);
        assert_eq!(f.name, "Data-In");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = vec![0u8; 48];
        d[0] = 0x1f; // reserved opcode
        assert!(parse(&d).is_none());
    }

    #[test]
    fn rejects_text_files() {
        // `#` is ASCII 0x23, which decodes to the target Login Response
        // opcode, so comment-led text files used to pass the opcode check.
        // The declared data length (bytes 5..8) saves us: ASCII text cannot
        // form a data segment that fits.
        let t = b"# a comment-led config file\nkey = value\nanother = line\n";
        assert!(!detect(t));
        assert!(parse(t).is_none());
    }
}

//! MAVLink v1/v2 frame headers.
//!
//! v1: `0xFE len seq sysid compid msgid payload cklo ckhi` (CRC-16/MCRF4XX
//! over `len..payload` plus a per-message CRC extra — not included).
//! v2: `0xFD len incompat compat seq sysid compid msgid[3] ...`.
//!
//! ```
//! use izanagi_kit::mavlink::parse;
//!
//! // heartbeat: len=9 seq=0 sys=1 comp=1 msg=0
//! let f = parse(&[0xFE, 9, 0, 1, 1, 0, 0,0,0,0, 0, 0, 4, 0, 0, 0x9E, 0xC7]).unwrap();
//! assert_eq!(f.sysid, 1);
//! assert_eq!(f.msgid, 0);
//! ```

/// MAVLink protocol version detected from the magic byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Version {
    /// `0xFE` v1.
    V1,
    /// `0xFD` v2.
    V2,
}

/// A parsed MAVLink frame (checksum bytes included, not verified —
/// the CRC extra per dialect is external data).
#[derive(Clone, Debug)]
pub struct Frame {
    /// Wire version.
    pub version: Version,
    /// Payload length.
    pub len: u8,
    /// Sequence.
    pub seq: u8,
    /// System id.
    pub sysid: u8,
    /// Component id.
    pub compid: u8,
    /// Message id (24-bit in v2).
    pub msgid: u32,
    /// v2 `incompat_flags`.
    pub incompat: u8,
    /// Offset where the payload starts.
    pub payload_at: usize,
    /// Offset where the trailing 2-byte checksum sits.
    pub checksum_at: usize,
}

impl Frame {
    /// Payload slice.
    pub fn payload<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(self.payload_at..self.checksum_at)
    }
}

fn u24le(d: &[u8], o: usize) -> u32 {
    (d[o] as u32) | ((d[o + 1] as u32) << 8) | ((d[o + 2] as u32) << 16)
}

/// Parse a MAVLink frame. `None` on unknown magic, truncation, or a
/// frame shorter/longer than declared.
pub fn parse(d: &[u8]) -> Option<Frame> {
    match *d.first()? {
        0xFE => {
            let len = *d.get(1)?;
            let at = 6usize;
            let chk = at + len as usize;
            if d.len() < chk + 2 {
                return None;
            }
            Some(Frame {
                version: Version::V1,
                len,
                seq: *d.get(2)?,
                sysid: *d.get(3)?,
                compid: *d.get(4)?,
                msgid: *d.get(5)? as u32,
                incompat: 0,
                payload_at: at,
                checksum_at: chk,
            })
        }
        0xFD => {
            let len = *d.get(1)?;
            let at = 10usize;
            let chk = at + len as usize;
            if d.len() < chk + 2 {
                return None;
            }
            // signature appended when incompat bit0 is set
            if *d.get(2)? & 1 == 1 && d.len() < chk + 2 + 13 {
                return None;
            }
            Some(Frame {
                version: Version::V2,
                len,
                incompat: *d.get(2)?,
                seq: *d.get(4)?,
                sysid: *d.get(5)?,
                compid: *d.get(6)?,
                msgid: u24le(d, 7),
                payload_at: at,
                checksum_at: chk,
            })
        }
        _ => None,
    }
}

/// CRC-16/MCRF4XX (X.25) over `len`..`payload` (dialect CRC extra must be
/// appended by the caller to compare with the frame checksum).
pub fn crc_mcrf4xx(d: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in d {
        let mut t = b ^ (crc as u8);
        t = t ^ (t << 4);
        crc = (crc >> 8) ^ ((t as u16) << 8) ^ ((t as u16) << 3) ^ ((t as u16) >> 4);
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        // heartbeat frame incl. checksum 0x9EC7 ignored
        let d = [0xFE, 9, 7, 1, 2, 0, 1, 2, 3, 4, 0, 0, 4, 0, 0, 0xC7, 0x9E];
        let f = parse(&d).unwrap();
        assert_eq!(f.version, Version::V1);
        assert_eq!(f.payload(&d), Some(&d[6..15]));
        // v2 minimal
        let v2 = [0xFD, 0, 0, 0, 1, 42, 1, 0x21, 0, 0, 0xAA, 0xBB];
        let f2 = parse(&v2).unwrap();
        assert_eq!(f2.version, Version::V2);
        assert_eq!(f2.msgid, 0x21);
        assert_eq!(f2.sysid, 42);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0xAA]).is_none());
        assert!(parse(&[0xFE, 9, 0, 0]).is_none());
    }

    #[test]
    fn crc_known_vector() {
        // CRC-16/MCRF4XX check value of "123456789" = 0x6F91
        assert_eq!(crc_mcrf4xx(b"123456789"), 0x6F91);
    }
}

//! NBD (Network Block Device) protocol — handshake + transmission-phase headers.
//!
//! Old-style: 64-bit `NBDMAGIC`, 32-bit handshake flags, 124B padding, then
//! request/reply headers (`0x25609513` request / `0x67446698` reply).
//!
//! ```
//! let mut d = b"NBDMAGIC".to_vec();
//! d.extend_from_slice(&[0x00, 0x00, 0x00, 0x02, 0x92, 0x53, 0x00, 0x01]); // magic2 + opts
//! d.resize(136, 0);
//! let n = izanagi_kit::nbd::parse(&d).unwrap();
//! assert!(n.handshake);
//! assert!(izanagi_kit::nbd::detect(&d));
//! ```
use std::string::String;

/// Parsed NBD packet/handshake.
#[derive(Debug, Clone)]
pub struct Nbd {
    /// `true` for the `NBDMAGIC` client greeting.
    pub handshake: bool,
    /// `true` for a transmission request (`0x25609513`).
    pub request: bool,
    /// `true` for a transmission reply (`0x67446698`).
    pub reply: bool,
    /// Command/operation name.
    pub op: String,
    /// Request: `type` field / reply: error field.
    pub cmd_or_error: u16,
    /// Request: `from` offset / reply: handle high word context.
    pub offset_or_handle: u64,
    /// Request length field.
    pub length: u32,
    /// New-style negotiation flags word (handshake only).
    pub flags: u16,
}

fn be32(b: &[u8], o: usize) -> u32 {
    ((b[o] as u32) << 24) | ((b[o + 1] as u32) << 16) | ((b[o + 2] as u32) << 8) | (b[o + 3] as u32)
}

fn be64(b: &[u8], o: usize) -> u64 {
    let mut v = 0u64;
    for i in 0..8 {
        v = (v << 8) | (b[o + i] as u64);
    }
    v
}

/// Detects NBD handshake or transmission magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"NBDMAGIC")
        || (b.len() >= 4 && (be32(b, 0) == 0x2560_9513 || be32(b, 0) == 0x6744_6698))
}

/// Parses an NBD packet; `None` without a known magic.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nbd> {
    if b.starts_with(b"NBDMAGIC") && b.len() >= 18 {
        return Some(Nbd {
            handshake: true,
            request: false,
            reply: false,
            op: String::from("handshake"),
            cmd_or_error: 0,
            offset_or_handle: 0,
            length: 0,
            flags: ((b[16] as u16) << 8) | (b[17] as u16),
        });
    }
    if b.len() < 16 {
        return None;
    }
    let magic = be32(b, 0);
    match magic {
        0x2560_9513 => Some(Nbd {
            handshake: false,
            request: true,
            reply: false,
            op: match be32(b, 4) & 0xffff {
                0 => String::from("READ"),
                1 => String::from("WRITE"),
                2 => String::from("DISC"),
                3 => String::from("FLUSH"),
                4 => String::from("TRIM"),
                _ => String::from("other"),
            },
            cmd_or_error: (be32(b, 4) & 0xffff) as u16,
            offset_or_handle: be64(b, 8),
            length: if b.len() >= 16 { be32(b, 12) } else { 0 },
            flags: 0,
        }),
        0x6744_6698 => Some(Nbd {
            handshake: false,
            request: false,
            reply: true,
            op: String::from("reply"),
            cmd_or_error: (be32(b, 4) & 0xffff) as u16,
            offset_or_handle: be64(b, 8),
            length: 0,
            flags: 0,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake() {
        let mut d = b"NBDMAGIC".to_vec();
        d.extend_from_slice(&[0x49, 0x48, 0x41, 0x56, 0x45, 0x4f, 0x50, 0x54]); // IHAVEOPT
        d.extend_from_slice(&[0x00, 0x03]);
        let f = parse(&d).unwrap();
        assert!(f.handshake);
        assert_eq!(f.flags, 3);
    }

    #[test]
    fn request() {
        let mut d = vec![0u8; 28];
        d[..4].copy_from_slice(&[0x25, 0x60, 0x95, 0x13]);
        d[6] = 0x00;
        d[7] = 0x01; // WRITE
        d[8] = 0x11; // offset low
        d[15] = 0x40;
        let f = parse(&d).unwrap();
        assert!(f.request);
        assert_eq!(f.op, "WRITE");
        assert_eq!(f.offset_or_handle, 0x1100_0000_0000_0040);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 16]).is_none());
    }
}

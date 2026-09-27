//! UBI (Unsorted Block Images) erase-counter header parsing.
//!
//! Each physical erase block begins with a 64-byte big-endian EC header:
//! magic `UBI#`, `version` @4, `ec` (erase count, u64) @8,
//! `vid_hdr_offset` @16, `data_offset` @20, `image_seq` @24,
//! `hdr_crc` @60 (CRC32 of bytes 0..60). A VID header (`UBI!` magic,
//! `version` @4) may follow at `vid_hdr_offset`.
//!
//! ```
//! use izanagi_kit::{crc, ubi};
//! let mut d = vec![0u8; 128];
//! d[0..4].copy_from_slice(b"UBI#");
//! d[4] = 1; // version
//! d[16..20].copy_from_slice(&64u32.to_be_bytes()); // vid offset
//! d[20..24].copy_from_slice(&128u32.to_be_bytes()); // data offset
//! let c = crc::crc32(&d[..60]);
//! d[60..64].copy_from_slice(&c.to_be_bytes());
//! d[64..68].copy_from_slice(b"UBI!");
//! d[68] = 1;
//! let u = ubi::parse(&d).unwrap();
//! assert_eq!(u.data_offset, 128);
//! assert!(ubi::vid_present(&d, &u));
//! ```

/// EC header magic.
pub const EC_MAGIC: u32 = 0x5542_4923; // "UBI#"

/// VID header magic.
pub const VID_MAGIC: u32 = 0x5542_4921; // "UBI!"

/// EC header size.
pub const EC_LEN: usize = 64;

/// A parsed UBI erase-counter header.
#[derive(Clone, Debug, PartialEq)]
pub struct Ubi {
    /// UBI version (must be 1).
    pub version: u8,
    /// `ec` — erase counter.
    pub ec: u64,
    /// Offset of the VID header within this block.
    pub vid_hdr_offset: u32,
    /// Offset of payload data within this block.
    pub data_offset: u32,
    /// `image_seq` — blocks of one image share the value.
    pub image_seq: u32,
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

fn u64be(d: &[u8], at: usize) -> Option<u64> {
    Some((u32be(d, at)? as u64) << 32 | u32be(d, at + 4)? as u64)
}

/// Parses the EC header; magic, version=1 and `hdr_crc` must validate.
pub fn parse(d: &[u8]) -> Option<Ubi> {
    if d.len() < EC_LEN {
        return None;
    }
    if u32be(d, 0)? != EC_MAGIC {
        return None;
    }
    let version = *d.get(4)?;
    if version != 1 {
        return None;
    }
    let crc = u32be(d, 60)?;
    if crate::crc::crc32(d.get(..60)?) != crc {
        return None;
    }
    Some(Ubi {
        version,
        ec: u64be(d, 8)?,
        vid_hdr_offset: u32be(d, 16)?,
        data_offset: u32be(d, 20)?,
        image_seq: u32be(d, 24)?,
    })
}

/// Whether a VID header (`UBI!` + version 1) sits at `vid_hdr_offset`.
pub fn vid_present(d: &[u8], u: &Ubi) -> bool {
    let at = match usize::try_from(u.vid_hdr_offset) {
        Ok(a) => a,
        Err(_) => return false,
    };
    match d.get(at..at + 8) {
        Some(h) => {
            (h[0] as u32) << 24 | (h[1] as u32) << 16 | (h[2] as u32) << 8 | h[3] as u32
                == VID_MAGIC
                && h[4] == 1
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 256];
        d[0..4].copy_from_slice(b"UBI#");
        d[4] = 1;
        d[8..16].copy_from_slice(&7u64.to_be_bytes());
        d[16..20].copy_from_slice(&128u32.to_be_bytes());
        d[20..24].copy_from_slice(&192u32.to_be_bytes());
        d[24..28].copy_from_slice(&0xA5u32.to_be_bytes());
        let c = crate::crc::crc32(&d[..60]);
        d[60..64].copy_from_slice(&c.to_be_bytes());
        d[128..132].copy_from_slice(b"UBI!");
        d[132] = 1;
        d
    }

    #[test]
    fn parses_ec() {
        let d = fixture();
        let u = parse(&d).unwrap();
        assert_eq!(u.ec, 7);
        assert_eq!(u.vid_hdr_offset, 128);
        assert_eq!(u.data_offset, 192);
        assert_eq!(u.image_seq, 0xA5);
        assert!(vid_present(&d, &u));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 32]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[4] = 2; // bad version
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[60] ^= 1; // bad crc
        assert!(parse(&d).is_none());
    }
}

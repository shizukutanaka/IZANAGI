//! Microsoft LIT e-books are ITS (InfoTech Storage) containers: an
//! `ITSF` signature, version u32LE, header length, timestamps and two
//! header-section GUIDs.
//!
//! ```
//! use izanagi_kit::lit::parse;
//!
//! let mut d = vec![0u8; 96];
//! d[0..4].copy_from_slice(b"ITSF");
//! d[4..8].copy_from_slice(&3u32.to_le_bytes());
//! d[8..12].copy_from_slice(&96u32.to_le_bytes()); // header_length
//! let l = parse(&d).unwrap();
//! assert_eq!(l.version, 3);
//! assert_eq!(l.header_len, 96);
//! ```

/// `ITSF` (InfoTech Storage Format) signature.
pub const MAGIC: &[u8; 4] = b"ITSF";

fn le32(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at + 4)?;
    Some(u32::from(s[0]) | u32::from(s[1]) << 8 | u32::from(s[2]) << 16 | u32::from(s[3]) << 24)
}

fn le64(d: &[u8], at: usize) -> Option<u64> {
    let lo = u64::from(le32(d, at)?);
    let hi = u64::from(le32(d, at + 4)?);
    Some(lo | hi << 32)
}

/// Parsed ITSF/LIT header fields.
#[derive(Debug, Clone)]
pub struct Lit {
    /// Format version (3 for LIT).
    pub version: u32,
    /// Total header length in bytes.
    pub header_len: u32,
    /// Section-1 GUID (16 bytes, raw).
    pub guid1: [u8; 16],
    /// Section-2 GUID (16 bytes, raw).
    pub guid2: [u8; 16],
    /// Data section offset computed as `header_len` (the stream table
    /// begins right after the header).
    pub data_at: u32,
    /// Total file size reported by the header when present (ITSF v3
    /// stores it at 0x2C as u64; `0` when the field is absent/short).
    pub declared_len: u64,
}

/// Parse the ITSF header shared by LIT/CHM (callers distinguish by the
/// GUIDs / content streams).
pub fn parse(d: &[u8]) -> Option<Lit> {
    if !d.starts_with(MAGIC) {
        return None;
    }
    let version = le32(d, 4)?;
    let header_len = le32(d, 8)?;
    if usize::try_from(header_len)
        .map(|h| h > d.len())
        .unwrap_or(true)
    {
        return None;
    }
    let mut guid1 = [0u8; 16];
    let mut guid2 = [0u8; 16];
    guid1.copy_from_slice(d.get(24..40)?);
    guid2.copy_from_slice(d.get(40..56)?);
    let declared_len = le64(d, 44).unwrap_or(0);
    Some(Lit {
        version,
        header_len,
        guid1,
        guid2,
        data_at: header_len,
        declared_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 96];
        d[0..4].copy_from_slice(b"ITSF");
        d[4..8].copy_from_slice(&3u32.to_le_bytes());
        d[8..12].copy_from_slice(&96u32.to_le_bytes());
        d[24] = 0x11;
        d[40] = 0x22;
        d[44..52].copy_from_slice(&4096u64.to_le_bytes());
        d
    }

    #[test]
    fn fields() {
        let l = parse(&fixture()).unwrap();
        assert_eq!(l.version, 3);
        assert_eq!(l.header_len, 96);
        assert_eq!(l.guid1[0], 0x11);
        assert_eq!(l.guid2[0], 0x22);
        assert_eq!(l.data_at, 96);
        assert_eq!(l.declared_len, 4096);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"CHM!").is_none());
        let mut d = fixture();
        d[8..12].copy_from_slice(&1000u32.to_le_bytes()); // header past EOF
        assert!(parse(&d).is_none());
        let d2 = fixture();
        assert!(parse(&d2[..40]).is_none()); // truncated guids
    }
}

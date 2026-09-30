//! FlatGeobuf — the flatbuffers-based geospatial feature format.
//!
//! Magic: `66 67 62 03 66 67 62 00` (`"fgb"` + major 3 + `"fgb"` + patch 0),
//! then a u32-LE sized FlatBuffers header table.
//!
//! ```
//! let mut d = vec![0x66, 0x67, 0x62, 0x03, 0x66, 0x67, 0x62, 0x00];
//! d.extend_from_slice(&[8, 0, 0, 0]); // header size
//! d.extend_from_slice(&[0u8; 8]);
//! let f = izanagi_kit::fgb::parse(&d).unwrap();
//! assert_eq!((f.major, f.patch), (3, 0));
//! ```

/// Parsed FlatGeobuf prolog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fgb {
    /// Format major version (3.x today).
    pub major: u8,
    /// Format patch version.
    pub patch: u8,
    /// Declared byte length of the FlatBuffers header table.
    pub header_len: u32,
}

/// The 8-byte magic as written on disk.
pub const MAGIC: [u8; 8] = [0x66, 0x67, 0x62, 0x03, 0x66, 0x67, 0x62, 0x00];

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Parse a FlatGeobuf file head; `None` on bad magic or truncated header.
pub fn parse(d: &[u8]) -> Option<Fgb> {
    if !d.starts_with(&MAGIC) {
        return None;
    }
    let header_len = le32(d, 8)?;
    if header_len == 0 || 12 + header_len as usize > d.len() {
        return None;
    }
    Some(Fgb {
        major: d[3],
        patch: d[7],
        header_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&[0u8; 16]); // table bytes + feature
        let f = parse(&d).unwrap();
        assert_eq!((f.major, f.patch, f.header_len), (3, 0, 4));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"fgb\x04fgb\x00\x01\x00\x00\x00").is_none()); // v4 magic
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&100u32.to_le_bytes());
        assert!(parse(&d).is_none()); // header_len overruns input
    }
}

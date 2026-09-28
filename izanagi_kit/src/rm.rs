//! RealMedia `.rm` / `.rmvb`: a `.RMF` file header followed by tagged
//! `u32be`-sized chunks — `PROP`, `MDPR` (one per stream), `CONT`,
//! `DATA`, `INDX`.
//!
//! ```
//! use izanagi_kit::rm::{detect, parse};
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(b".RMF");
//! d.extend_from_slice(&[0, 0, 0, 18]); // chunk size
//! d.extend_from_slice(&[0, 1]); // version
//! d.extend_from_slice(&[0, 0, 0, 0]); // file version
//! d.extend_from_slice(&[0, 0, 0, 3]); // header count
//! d.extend_from_slice(b"PROP");
//! d.extend_from_slice(&[0, 0, 0, 10]); // size
//! d.extend_from_slice(&[0, 0]); // version
//! d.extend_from_slice(b"MDPR");
//! d.extend_from_slice(&[0, 0, 0, 10]);
//! d.extend_from_slice(&[0, 0]);
//! d.extend_from_slice(b"DATA");
//! d.extend_from_slice(&[0, 0, 0, 10]);
//! d.extend_from_slice(&[0, 0]);
//! assert!(detect(&d));
//! let r = parse(&d).unwrap();
//! assert_eq!(r.streams, 1);
//! assert!(r.has_prop && r.has_data);
//! ```

/// Parsed RealMedia header.
#[derive(Debug, Clone, PartialEq)]
pub struct Rm {
    /// `.RMF` header version (`u16`).
    pub version: u16,
    /// File format version field.
    pub file_version: u32,
    /// Declared number of header chunks in the `.RMF` object.
    pub declared_headers: u32,
    /// Chunks walked after the `.RMF` object.
    pub chunks: u32,
    /// `PROP` chunk seen.
    pub has_prop: bool,
    /// `CONT` (content description) chunk seen.
    pub has_cont: bool,
    /// `DATA` chunk seen.
    pub has_data: bool,
    /// `INDX` chunk seen.
    pub has_index: bool,
    /// `MDPR` media-properties chunks — one per stream.
    pub streams: u32,
    /// Max bit rate carried by `PROP` (its first `u32be`), if present.
    pub max_bit_rate: Option<u32>,
}

fn be32(b: &[u8]) -> u32 {
    u32::from(b[0]) << 24 | u32::from(b[1]) << 16 | u32::from(b[2]) << 8 | u32::from(b[3])
}

/// `true` when the buffer opens with `.RMF`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 18 && b[..4] == *b".RMF"
}

/// Parses the chunk walk; `None` without the `.RMF` object.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Rm> {
    if !detect(b) {
        return None;
    }
    let hdr_size = usize::try_from(be32(&b[4..8])).unwrap_or(0);
    if hdr_size < 18 || b.len() < hdr_size {
        return None;
    }
    let mut r = Rm {
        version: u16::from(b[8]) << 8 | u16::from(b[9]),
        file_version: be32(&b[10..14]),
        declared_headers: be32(&b[14..18]),
        chunks: 0,
        has_prop: false,
        has_cont: false,
        has_data: false,
        has_index: false,
        streams: 0,
        max_bit_rate: None,
    };
    let mut off = hdr_size;
    while off + 6 <= b.len() {
        let id = &b[off..off + 4];
        let size = usize::try_from(be32(&b[off + 4..off + 8])).unwrap_or(0);
        if size < 10 || off + size > b.len() {
            break;
        }
        r.chunks += 1;
        match id {
            b"PROP" => {
                r.has_prop = true;
                if off + 14 <= b.len() {
                    r.max_bit_rate = Some(be32(&b[off + 10..off + 14]));
                }
            }
            b"MDPR" => r.streams += 1,
            b"CONT" => r.has_cont = true,
            b"DATA" => r.has_data = true,
            b"INDX" => r.has_index = true,
            _ => {}
        }
        off += size;
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b".RMF");
        d.extend_from_slice(&[0, 0, 0, 18, 0, 1, 0, 0, 0, 0, 0, 0, 0, 3]);
        d.extend_from_slice(b"PROP");
        d.extend_from_slice(&[0, 0, 0, 14, 0, 0, 0, 0, 9, 0]);
        d.extend_from_slice(b"MDPR");
        d.extend_from_slice(&[0, 0, 0, 10, 0, 0]);
        d.extend_from_slice(b"DATA");
        d.extend_from_slice(&[0, 0, 0, 10, 0, 0]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b".RMP"));
        assert!(!detect(&fixture()[..10]));
    }

    #[test]
    fn parses() {
        let r = parse(&fixture()).unwrap();
        assert_eq!(r.version, 1);
        assert_eq!(r.declared_headers, 3);
        assert_eq!(r.chunks, 3);
        assert!(r.has_prop && r.has_data);
        assert!(!r.has_cont && !r.has_index);
        assert_eq!(r.streams, 1);
        assert_eq!(r.max_bit_rate, Some(2304));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"not real").is_none());
        assert!(parse(b".RMF").is_none());
    }
}

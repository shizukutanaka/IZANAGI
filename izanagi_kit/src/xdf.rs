//! XDF (Extensible Data Format, `*.xdf`) chunk census.
//!
//! `XDF:` magic then length-prefixed chunks: `num_len_bytes`(1) +
//! `length`(1/4/8 LE) + `tag`(2 LE) + payload. Tags: 1 FileHeader,
//! 2 StreamHeader, 3 Samples, 4 ClockOffset, 5 Boundary, 6 StreamFooter.
//!
//! ```
//! let s = b"XDF:\x01#\x01\x00<info><version>1</version></info>\x01\x08\x02\x00<xml/>\x01\x06\x03\x00DATA\x01\x06\x06\x00END!";
//! assert!(izanagi_kit::xdf::detect(s));
//! let x = izanagi_kit::xdf::Xdf::parse(s).unwrap();
//! assert_eq!(x.chunks, 4);
//! assert_eq!(x.file_headers, 1);
//! assert_eq!(x.stream_headers, 1);
//! assert_eq!(x.samples, 1);
//! assert_eq!(x.stream_footers, 1);
//! ```

/// Parsed census of an XDF file.
#[derive(Debug, Clone)]
pub struct Xdf {
    /// Total chunks walked.
    pub chunks: usize,
    /// Tag-1 FileHeader chunks.
    pub file_headers: usize,
    /// Tag-2 StreamHeader chunks.
    pub stream_headers: usize,
    /// Tag-3 Samples chunks.
    pub samples: usize,
    /// Tag-4 ClockOffset chunks.
    pub clock_offsets: usize,
    /// Tag-5 Boundary chunks.
    pub boundaries: usize,
    /// Tag-6 StreamFooter chunks.
    pub stream_footers: usize,
    /// Chunks with an unrecognized tag.
    pub unknown_tags: usize,
    /// `<` XML payloads inside chunks.
    pub xml_blocks: usize,
    /// Bytes consumed after `XDF:`.
    pub payload_bytes: usize,
    /// Trailing bytes that did not form a full chunk.
    pub trailing: usize,
}

/// Reports whether `b` starts with the `XDF:` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"XDF:")
}

impl Xdf {
    /// Parses `b` as an XDF file, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let mut x = Xdf {
            chunks: 0,
            file_headers: 0,
            stream_headers: 0,
            samples: 0,
            clock_offsets: 0,
            boundaries: 0,
            stream_footers: 0,
            unknown_tags: 0,
            xml_blocks: 0,
            payload_bytes: 0,
            trailing: 0,
        };
        let mut pos = 4usize;
        while pos < b.len() {
            let nlb = match b[pos] {
                1 => 1usize,
                4 => 4usize,
                8 => 8usize,
                _ => {
                    x.trailing = b.len() - pos;
                    break;
                }
            };
            pos += 1;
            if pos + nlb + 2 > b.len() {
                x.trailing = b.len() - pos;
                break;
            }
            let mut len = 0u64;
            for i in 0..nlb {
                len |= u64::from(b[pos + i]) << (8 * i);
            }
            pos += nlb;
            if len < 2 {
                x.trailing = b.len() - pos;
                break;
            }
            let len = len as usize;
            if pos + len > b.len() {
                x.trailing = b.len() - pos;
                break;
            }
            let tag = u16::from_le_bytes([b[pos], b[pos + 1]]);
            x.chunks += 1;
            match tag {
                1 => x.file_headers += 1,
                2 => x.stream_headers += 1,
                3 => x.samples += 1,
                4 => x.clock_offsets += 1,
                5 => x.boundaries += 1,
                6 => x.stream_footers += 1,
                _ => x.unknown_tags += 1,
            }
            if b[pos..pos + len].contains(&b'<') {
                x.xml_blocks += 1;
            }
            x.payload_bytes += len;
            pos += len;
        }
        Some(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"XDF:\x01#\x01\x00<info><version>1</version></info>\x01\x08\x02\x00<xml/>\x01\x06\x03\x00DATA\x01\x06\x06\x00END!";

    #[test]
    fn parses_xdf() {
        assert!(detect(S));
        let x = Xdf::parse(S).unwrap();
        assert_eq!(x.chunks, 4);
        assert_eq!(x.file_headers, 1);
        assert_eq!(x.stream_headers, 1);
        assert_eq!(x.samples, 1);
        assert_eq!(x.stream_footers, 1);
    }

    #[test]
    fn rejects_non_xdf() {
        assert!(!detect(b"NOTX"));
        assert!(Xdf::parse(b"").is_none());
    }
}

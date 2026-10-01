//! RecordIO chunk container parser (MXNet/dmlc `kMagic` format).
//!
//! Detects `u32 kMagic(0xced7230a) | u32 length | payload | pad`
//! chunks and counts chunks, payload bytes, padded chunks and corrupt
//! tails.
//!
//! ```
//! let mut b = vec![0x0A, 0x23, 0xD7, 0xCE, 4, 0, 0, 0];
//! b.extend_from_slice(b"data");
//! assert!(izanagi_kit::recordio::detect(&b));
//! let c = izanagi_kit::recordio::Recordio::parse(&b).unwrap();
//! assert_eq!(c.chunks, 1);
//! ```

/// Parsed RecordIO stream summary.
#[derive(Debug, Clone)]
pub struct Recordio {
    /// Complete chunks parsed.
    pub chunks: usize,
    /// Total payload bytes.
    pub payload_bytes: u64,
    /// Chunks carrying non-zero padding.
    pub padded: usize,
    /// Trailing/corrupt bytes after the last complete chunk.
    pub trailing: usize,
    /// Largest chunk payload.
    pub longest: u64,
}

const MAGIC: u32 = 0xced7_230a;

fn le_u32(b: &[u8], off: usize) -> u32 {
    u32::from(b[off + 3]) << 24
        | u32::from(b[off + 2]) << 16
        | u32::from(b[off + 1]) << 8
        | u32::from(b[off])
}

/// Whether the buffer looks like a RecordIO stream.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8 && le_u32(b, 0) == MAGIC
}

impl Recordio {
    /// Parses a RecordIO stream summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            chunks: 0,
            payload_bytes: 0,
            padded: 0,
            trailing: 0,
            longest: 0,
        };
        let mut off = 0usize;
        while off + 8 <= b.len() {
            if le_u32(b, off) != MAGIC {
                c.trailing = b.len() - off;
                break;
            }
            let len = le_u32(b, off + 4) as usize;
            let pad = (4 - len % 4) % 4;
            let end = off + 8 + len + pad;
            if end > b.len() {
                c.trailing = b.len() - off;
                break;
            }
            c.chunks += 1;
            c.payload_bytes += len as u64;
            c.longest = c.longest.max(len as u64);
            if pad > 0 {
                c.padded += 1;
            }
            off = end;
        }
        if off < b.len() && c.trailing == 0 {
            c.trailing = b.len() - off;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&[0x0A, 0x23, 0xD7, 0xCE]);
        let n = payload.len();
        v.extend_from_slice(&[
            (n & 0xff) as u8,
            ((n >> 8) & 0xff) as u8,
            ((n >> 16) & 0xff) as u8,
            ((n >> 24) & 0xff) as u8,
        ]);
        v.extend_from_slice(payload);
        while v.len() % 4 != 0 {
            v.push(0);
        }
        v
    }

    #[test]
    fn detects_and_counts() {
        let mut b = chunk(b"abcd");
        b.extend_from_slice(&chunk(b"xy"));
        b.extend_from_slice(&chunk(b""));
        assert!(detect(&b));
        let c = Recordio::parse(&b).unwrap();
        assert_eq!(c.chunks, 3);
        assert_eq!(c.payload_bytes, 6);
        assert_eq!(c.padded, 1);
        assert_eq!(c.longest, 4);
    }

    #[test]
    fn corrupt_tail() {
        let mut b = chunk(b"aa");
        b.extend_from_slice(&[9, 9]);
        assert!(detect(&b));
        let c = Recordio::parse(&b).unwrap();
        assert_eq!(c.chunks, 1);
        assert_eq!(c.trailing, 2);
    }

    #[test]
    fn rejects() {
        assert!(!detect(b""));
        assert!(!detect(b"plain"));
        assert!(Recordio::parse(&[0; 20]).is_none());
    }
}

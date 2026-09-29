//! TFRecord binary container parser (length-prefixed CRC records).
//!
//! Detects `u64 len | u32 len-crc | payload | u32 data-crc` record
//! streams and counts records, total payload bytes, longest record and
//! empty records.
//!
//! ```
//! let mut b = vec![8, 0, 0, 0, 0, 0, 0, 0, 0xAA, 0, 0, 0];
//! b.extend_from_slice(b"payload1");
//! b.extend_from_slice(&[0xBB, 0, 0, 0]);
//! b.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
//! assert!(izanagi_kit::tfrecord::detect(&b));
//! let c = izanagi_kit::tfrecord::Tfrecord::parse(&b).unwrap();
//! assert_eq!(c.records, 2);
//! assert_eq!(c.payload_bytes, 8);
//! ```

/// Parsed TFRecord stream summary.
#[derive(Debug, Clone)]
pub struct Tfrecord {
    /// Complete records parsed.
    pub records: usize,
    /// Total payload bytes.
    pub payload_bytes: u64,
    /// Longest single record payload.
    pub longest: u64,
    /// Zero-length records.
    pub empty: usize,
    /// Trailing bytes after the last complete record.
    pub trailing: usize,
}

fn le_u64(b: &[u8], off: usize) -> u64 {
    let mut v = 0u64;
    for i in (0..8).rev() {
        v = (v << 8) | u64::from(b[off + i]);
    }
    v
}

/// Whether the buffer looks like a TFRecord stream.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 16 {
        return false;
    }
    let len = le_u64(b, 0);
    len as usize <= b.len().saturating_sub(16) && (len > 0 || b.len() == 16)
}

impl Tfrecord {
    /// Parses a TFRecord stream summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            records: 0,
            payload_bytes: 0,
            longest: 0,
            empty: 0,
            trailing: 0,
        };
        let mut off = 0usize;
        while off + 12 <= b.len() {
            let len = le_u64(b, off) as usize;
            let next = off + 12 + len;
            if next + 4 > b.len() {
                c.trailing = b.len() - off;
                break;
            }
            c.records += 1;
            c.payload_bytes += len as u64;
            c.longest = c.longest.max(len as u64);
            if len == 0 {
                c.empty += 1;
            }
            off = next + 4;
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

    fn rec(payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        for i in 0..8 {
            v.push(((payload.len() >> (i * 8)) & 0xff) as u8);
        }
        v.extend_from_slice(&[0, 0, 0, 0]);
        v.extend_from_slice(payload);
        v.extend_from_slice(&[0, 0, 0, 0]);
        v
    }

    #[test]
    fn detects_and_counts() {
        let mut b = rec(b"alpha");
        b.extend_from_slice(&rec(b"beta-gamma"));
        b.extend_from_slice(&rec(b""));
        assert!(detect(&b));
        let c = Tfrecord::parse(&b).unwrap();
        assert_eq!(c.records, 3);
        assert_eq!(c.payload_bytes, 15);
        assert_eq!(c.longest, 10);
        assert_eq!(c.empty, 1);
        assert_eq!(c.trailing, 0);
    }

    #[test]
    fn trailing_garbage() {
        let mut b = rec(b"xx");
        b.extend_from_slice(&[1, 2, 3]);
        assert!(detect(&b));
        let c = Tfrecord::parse(&b).unwrap();
        assert_eq!(c.records, 1);
        assert_eq!(c.trailing, 3);
    }

    #[test]
    fn rejects_short() {
        assert!(!detect(b"short"));
        assert!(!detect(&[0u8; 4]));
        assert!(Tfrecord::parse(b"").is_none());
    }
}

//! Java Flight Recorder (`.jfr`) chunk-file parser.
//!
//! Verifies the `FLR\0` magic plus the 68-byte big-endian chunk header
//! (`major`/`minor` version, `size`, `constantPoolOffset`,
//! `metadataOffset`, `startNanos`, `durationNanos`, `startTicks`,
//! `ticksPerSecond`, `features`) and reports the fields as integers.
//!
//! ```
//! let mut b = Vec::new();
//! b.extend_from_slice(b"FLR\0");
//! b.extend_from_slice(&2u16.to_be_bytes());
//! b.extend_from_slice(&1u16.to_be_bytes());
//! b.extend_from_slice(&4096u64.to_be_bytes());
//! b.extend_from_slice(&512u64.to_be_bytes());
//! b.extend_from_slice(&2048u64.to_be_bytes());
//! b.extend_from_slice(&1000u64.to_be_bytes());
//! b.extend_from_slice(&300u64.to_be_bytes());
//! b.extend_from_slice(&7u64.to_be_bytes());
//! b.extend_from_slice(&2800000000u64.to_be_bytes());
//! b.extend_from_slice(&3u32.to_be_bytes());
//! assert!(izanagi_kit::jfr::detect(&b));
//! let c = izanagi_kit::jfr::Jfr::parse(&b).unwrap();
//! assert_eq!(c.major, 2);
//! assert_eq!(c.chunk_size, 4096);
//! ```

/// Parsed JFR chunk header summary.
#[derive(Debug, Clone)]
pub struct Jfr {
    /// Format major version (currently `2`).
    pub major: u16,
    /// Format minor version.
    pub minor: u16,
    /// Total chunk size in bytes.
    pub chunk_size: u64,
    /// Constant-pool section offset.
    pub constant_pool_offset: u64,
    /// Metadata section offset.
    pub metadata_offset: u64,
    /// Chunk start timestamp, nanoseconds.
    pub start_nanos: u64,
    /// Chunk duration, nanoseconds.
    pub duration_nanos: u64,
    /// Chunk start ticks.
    pub start_ticks: u64,
    /// Ticks per second.
    pub ticks_per_second: u64,
    /// Feature flags bitset (`compressedInts` …).
    pub features: u32,
}

// JFR fields are big-endian on disk; assemble them by shift so no
// native or big-endian conversion enters the library.
fn u64be(b: &[u8], off: usize) -> Option<u64> {
    let s = b.get(off..off + 8)?;
    let mut v = 0u64;
    for &x in s {
        v = (v << 8) | u64::from(x);
    }
    Some(v)
}

fn u16be(b: &[u8], off: usize) -> u16 {
    (u16::from(b[off]) << 8) | u16::from(b[off + 1])
}

fn u32be(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off + 4)?;
    let mut v = 0u32;
    for &x in s {
        v = (v << 8) | u32::from(x);
    }
    Some(v)
}

/// Whether the buffer starts a JFR chunk file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 68 || !b.starts_with(b"FLR\0") {
        return false;
    }
    let major = u16be(b, 4);
    (1..=3).contains(&major)
}

impl Jfr {
    /// Parses a JFR chunk header summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        Some(Self {
            major: u16be(b, 4),
            minor: u16be(b, 6),
            chunk_size: u64be(b, 8)?,
            constant_pool_offset: u64be(b, 16)?,
            metadata_offset: u64be(b, 24)?,
            start_nanos: u64be(b, 32)?,
            duration_nanos: u64be(b, 40)?,
            start_ticks: u64be(b, 48)?,
            ticks_per_second: u64be(b, 56)?,
            features: u32be(b, 64)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(major: u16) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"FLR\0");
        b.extend_from_slice(&major.to_be_bytes());
        b.extend_from_slice(&1u16.to_be_bytes());
        for v in [4096u64, 512, 2048, 1000, 300, 7, 2_800_000_000] {
            b.extend_from_slice(&v.to_be_bytes());
        }
        b.extend_from_slice(&3u32.to_be_bytes());
        b
    }

    #[test]
    fn detects_and_fields() {
        let b = hdr(2);
        assert!(detect(&b));
        let c = Jfr::parse(&b).unwrap();
        assert_eq!(c.major, 2);
        assert_eq!(c.minor, 1);
        assert_eq!(c.chunk_size, 4096);
        assert_eq!(c.constant_pool_offset, 512);
        assert_eq!(c.metadata_offset, 2048);
        assert_eq!(c.duration_nanos, 300);
        assert_eq!(c.ticks_per_second, 2_800_000_000);
        assert_eq!(c.features, 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"FLRX"));
        assert!(!detect(b"gmon"));
        assert!(!detect(&hdr(9)));
        assert!(Jfr::parse(b"FLR\0").is_none());
    }
}

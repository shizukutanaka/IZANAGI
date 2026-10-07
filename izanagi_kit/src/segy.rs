//! SEG-Y rev 0/1 file header parsing (3200-byte EBCDIC text header +
//! 400-byte binary header, big-endian).
//!
//! ```
//! use izanagi_kit::segy::{parse, SampleFormat};
//!
//! let mut h = vec![0u8; 3600];
//! h[3216..3218].copy_from_slice(&[0, 100]); // hdt = 100 us? (u16 at 3216)
//! h[3220..3222].copy_from_slice(&[0, 50]);  // hns = 50
//! h[3224..3226].copy_from_slice(&[0, 5]);   // format = 5 (IEEE f32)
//! let s = parse(&h).unwrap();
//! assert_eq!(s.sample_format, SampleFormat::IeeeF32);
//! assert_eq!(s.traces_at, 3600);
//! ```

/// SEG-Y sample format code (binary header offset 3225).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 1 — IBM floating-point 4-byte.
    IbmF32,
    /// 2 — 4-byte two's-complement integer.
    Int32,
    /// 3 — 2-byte two's-complement integer.
    Int16,
    /// 4 — 4-byte fixed-point with gain (obsolete).
    FixedGain,
    /// 5 — IEEE floating-point 4-byte.
    IeeeF32,
    /// 8 — 1-byte two's-complement integer.
    Int8,
    /// Unrecognised code.
    Other(u16),
}

impl SampleFormat {
    /// Bytes per sample for the known formats, else `None`.
    pub fn bytes(&self) -> Option<usize> {
        match self {
            SampleFormat::IbmF32
            | SampleFormat::Int32
            | SampleFormat::FixedGain
            | SampleFormat::IeeeF32 => Some(4),
            SampleFormat::Int16 => Some(2),
            SampleFormat::Int8 => Some(1),
            SampleFormat::Other(_) => None,
        }
    }
}

/// Parsed SEG-Y binary header fields.
#[derive(Debug)]
pub struct Segy {
    /// Job identification number.
    pub job_id: i32,
    /// Line number.
    pub line: i32,
    /// Reel number.
    pub reel: i32,
    /// Traces per ensemble.
    pub traces_per_ensemble: u16,
    /// Auxiliary traces per ensemble.
    pub aux_traces: u16,
    /// Sample interval, microseconds.
    pub dt_us: u16,
    /// Samples per trace.
    pub nsamp: u16,
    /// Sample format code.
    pub sample_format: SampleFormat,
    /// Offset where trace data begins (always 3600).
    pub traces_at: usize,
}

/// Parses a SEG-Y file: needs ≥3600 bytes; reads the 400-byte binary
/// header (all big-endian).
pub fn parse(d: &[u8]) -> Option<Segy> {
    if d.len() < 3600 {
        return None;
    }
    let b = &d[3200..];
    let i32be = |o: usize| {
        ((b[o] as i32) << 24)
            | ((b[o + 1] as i32) << 16)
            | ((b[o + 2] as i32) << 8)
            | b[o + 3] as i32
    };
    let u16be = |o: usize| ((b[o] as u16) << 8) | b[o + 1] as u16;
    let fmt_raw = u16be(24);
    Some(Segy {
        job_id: i32be(0),
        line: i32be(4),
        reel: i32be(8),
        traces_per_ensemble: u16be(12),
        aux_traces: u16be(14),
        dt_us: u16be(16),
        nsamp: u16be(20),
        sample_format: match fmt_raw {
            1 => SampleFormat::IbmF32,
            2 => SampleFormat::Int32,
            3 => SampleFormat::Int16,
            4 => SampleFormat::FixedGain,
            5 => SampleFormat::IeeeF32,
            8 => SampleFormat::Int8,
            o => SampleFormat::Other(o),
        },
        traces_at: 3600,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut h = vec![0u8; 3600];
        h[3200..3204].copy_from_slice(&[0x00, 0x00, 0x30, 0x39]); // job 12345
        h[3204..3208].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x9C]); // line -100
        h[3212..3214].copy_from_slice(&[0x00, 0x28]); // 40 ens
        h[3216..3218].copy_from_slice(&[0x0F, 0xA0]); // dt 4000 us
        h[3220..3222].copy_from_slice(&[0x07, 0xD0]); // nsamp 2000
        h[3224..3226].copy_from_slice(&[0x00, 0x01]); // IBM float
        let s = parse(&h).unwrap();
        assert_eq!(s.job_id, 12345);
        assert_eq!(s.line, -100);
        assert_eq!(s.traces_per_ensemble, 40);
        assert_eq!(s.dt_us, 4000);
        assert_eq!(s.nsamp, 2000);
        assert_eq!(s.sample_format, SampleFormat::IbmF32);
        assert_eq!(s.sample_format.bytes(), Some(4));
    }

    #[test]
    fn rejects() {
        assert!(parse(&vec![0u8; 3599]).is_none());
        let mut h = vec![0u8; 3600];
        h[3224..3226].copy_from_slice(&[0, 99]);
        assert_eq!(parse(&h).unwrap().sample_format, SampleFormat::Other(99));
    }
}

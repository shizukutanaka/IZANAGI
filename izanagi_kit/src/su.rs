//! Seismic Unix (SU) trace header sniffing — SEGY-style 240-byte trace
//! headers, little-endian, with no file-level header.
//!
//! Key fields: `tracl`(i32 @0), `ns`(u16 @114), `dt`(u16 @116, µs).
//!
//! ```
//! use izanagi_kit::su::parse;
//!
//! let mut t = vec![0u8; 240 + 8];
//! t[0..4].copy_from_slice(&[1, 0, 0, 0]);       // tracl 1
//! t[114..116].copy_from_slice(&[2, 0]);         // ns 2
//! t[116..118].copy_from_slice(&[0xE8, 0x03]);   // dt 1000 us
//! let s = parse(&t).unwrap();
//! assert_eq!(s.tracl, 1);
//! assert_eq!(s.ns, 2);
//! assert_eq!(s.trace_size, 248);
//! ```

/// Parsed SU trace header (first trace of a file or a single trace).
pub struct Su {
    /// Trace sequence number in line (`tracl`).
    pub tracl: i32,
    /// Trace sequence number within reel (`tracr`).
    pub tracr: i32,
    /// Trace number within ensemble (`cdp`-relative `tracf`).
    pub fldr: i32,
    /// Number of samples in this trace (`ns`).
    pub ns: u16,
    /// Sample interval in microseconds (`dt`).
    pub dt_us: u16,
    /// Bytes per trace including its header (240 + ns × 4 floats).
    pub trace_size: usize,
    /// Number of whole traces that fit evenly in `d` (0 when the tail
    /// is ragged).
    pub trace_count: usize,
}

/// Parses an SU trace buffer. `ns` must be > 0 and each trace occupies
/// `240 + ns*4` bytes; the trailing sample data must fit.
pub fn parse(d: &[u8]) -> Option<Su> {
    if d.len() < 240 {
        return None;
    }
    let i32le = |o: usize| {
        (d[o] as i32)
            | ((d[o + 1] as i32) << 8)
            | ((d[o + 2] as i32) << 16)
            | ((d[o + 3] as i32) << 24)
    };
    let u16le = |o: usize| (d[o] as u16) | ((d[o + 1] as u16) << 8);
    let ns = u16le(114);
    if ns == 0 {
        return None;
    }
    let trace_size = 240usize + ns as usize * 4;
    if d.len() < trace_size {
        return None;
    }
    Some(Su {
        tracl: i32le(0),
        tracr: i32le(4),
        fldr: i32le(8),
        ns,
        dt_us: u16le(116),
        trace_size,
        trace_count: if d.len() % trace_size == 0 {
            d.len() / trace_size
        } else {
            0
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut t = vec![0u8; 240 + 16];
        t[0..4].copy_from_slice(&[5, 0, 0, 0]);
        t[114..116].copy_from_slice(&[4, 0]);
        t[116..118].copy_from_slice(&[0xD0, 0x07]); // 2000
        let s = parse(&t).unwrap();
        assert_eq!(s.tracl, 5);
        assert_eq!(s.ns, 4);
        assert_eq!(s.dt_us, 2000);
        assert_eq!(s.trace_size, 256);
        assert_eq!(s.trace_count, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0; 239]).is_none());
        let mut t = vec![0u8; 244];
        t[114..116].copy_from_slice(&[10, 0]); // ns=10 needs 280 bytes
        assert!(parse(&t).is_none());
        let mut t = vec![0u8; 240];
        t[114..116].copy_from_slice(&[0, 0]); // ns=0
        assert!(parse(&t).is_none());
    }
}

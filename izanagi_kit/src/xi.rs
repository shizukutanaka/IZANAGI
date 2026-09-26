//! FastTracker II instrument (`.xi`) — `Extended Instrument: ` file
//! header, then a fixed 230-byte instrument header: 96-byte
//! note→sample map, volume/pan envelopes, vibrato, fadeout, and
//! `num_samples` at file offset 0x128.
//!
//! ```
//! use izanagi_kit::xi::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 2 + 40];
//! d[..21].copy_from_slice(b"Extended Instrument: ");
//! d[0x15..0x15 + 4].copy_from_slice(b"Lead");
//! d[0x2B] = 0x1A;
//! d[0x2C..0x2C + 6].copy_from_slice(b"FT 2.9");
//! d[0x40] = 2; d[0x41] = 1; // version 1.02 (LE)
//! d[0x42] = 7;              // note 1 -> sample 7
//! d[0x128] = 1;             // num_samples
//! let x = parse(&d).unwrap();
//! assert_eq!(x.name, "Lead");
//! assert_eq!(x.num_samples, 1);
//! assert_eq!(x.sample_for_note(0), Some(7));
//! ```

/// `"Extended Instrument: "` (21 bytes).
pub const MAGIC: &[u8; 21] = b"Extended Instrument: ";
/// Fixed part of the file: header + 230-byte instrument block + 2-byte
/// sample count.
pub const HEADER: usize = 0x12A;
/// Offset of the 96-entry note→sample map.
pub const NOTE_MAP: usize = 0x42;
/// Offset of the `num_samples` u16LE.
pub const NUM_SAMPLES: usize = 0x128;
/// Bytes per sample header following `num_samples`.
pub const SAMPLE_HEADER: usize = 40;

/// Parsed `.xi` instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xi<'a> {
    /// Instrument name (NUL/space padded, 22 bytes max).
    pub name: &'a str,
    /// Tracker name (20 bytes max).
    pub tracker: &'a str,
    /// Format version u16LE (`0x0102` = v1.02).
    pub version: u16,
    /// Number of sample headers following the fixed block.
    pub num_samples: u16,
    /// Offset of the first sample header.
    pub sample_headers_at: usize,
    d: &'a [u8],
}

fn name_at(d: &[u8], at: usize, n: usize) -> &str {
    let s = d.get(at..at + n).unwrap_or(&[]);
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    std::str::from_utf8(&s[..end]).unwrap_or("").trim_end()
}

/// Parse; requires the magic and enough room for the fixed block.
pub fn parse(d: &[u8]) -> Option<Xi<'_>> {
    if d.get(..21)? != MAGIC || *d.get(0x2B)? != 0x1A || d.len() < HEADER {
        return None;
    }
    let version = u16::from(d[0x40]) | (u16::from(d[0x41]) << 8);
    let num_samples = u16::from(d[NUM_SAMPLES]) | (u16::from(d[NUM_SAMPLES + 1]) << 8);
    Some(Xi {
        name: name_at(d, 0x15, 22),
        tracker: name_at(d, 0x2C, 20),
        version,
        num_samples,
        sample_headers_at: HEADER,
        d,
    })
}

impl<'a> Xi<'a> {
    /// Sample index for note `n` (0..95); `None` outside the map.
    pub fn sample_for_note(&self, n: usize) -> Option<u8> {
        if n >= 96 {
            return None;
        }
        self.d.get(NOTE_MAP + n).copied()
    }

    /// Sample header `i` (40 bytes) when present.
    pub fn sample_header(&self, i: usize) -> Option<&'a [u8]> {
        if i >= self.num_samples as usize {
            return None;
        }
        let at = self.sample_headers_at.checked_add(i * SAMPLE_HEADER)?;
        self.d.get(at..at + SAMPLE_HEADER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; HEADER + 2 * SAMPLE_HEADER + 16];
        d[..21].copy_from_slice(MAGIC);
        d[0x15..0x15 + 6].copy_from_slice(b"Bass 1");
        d[0x2B] = 0x1A;
        d[0x2C..0x2C + 6].copy_from_slice(b"MilkyT");
        d[0x40] = 2;
        d[0x41] = 1;
        d[0x42] = 3; // note 0 -> sample 3
        d[0x42 + 95] = 1;
        d[NUM_SAMPLES] = 2;
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let x = parse(&d).unwrap();
        assert_eq!(x.name, "Bass 1");
        assert_eq!(x.tracker, "MilkyT");
        assert_eq!(x.version, 0x0102);
        assert_eq!(x.num_samples, 2);
        assert_eq!(x.sample_for_note(0), Some(3));
        assert_eq!(x.sample_for_note(95), Some(1));
        assert_eq!(x.sample_for_note(96), None);
        assert_eq!(x.sample_headers_at, HEADER);
        assert!(x.sample_header(1).is_some());
        assert!(x.sample_header(2).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"Extended Instrument: \x00").is_none()); // too short
        let mut d = fixture();
        d[0x2B] = 0; // bad marker
        assert!(parse(&d).is_none());
    }
}

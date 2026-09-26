//! SBI — Sound Blaster Instrument file (OPL2/OPL3 FM patch):
//! `SBI\x1A` magic, 32-byte NUL-padded name, then 16 register bytes
//! covering the modulator/carrier operator fields and feedback.
//!
//! ```
//! use izanagi_kit::sbi::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER];
//! d[..4].copy_from_slice(b"SBI\x1A");
//! d[4..10].copy_from_slice(b"Bell  ");
//! d[36] = 0x21; // modulator AM/VIB/EG/KSR/MULTIPLE
//! let s = parse(&d).unwrap();
//! assert_eq!(s.name, "Bell");
//! assert_eq!(s.register(0), Some(0x21));
//! ```

/// Magic including the DOS EOF marker.
pub const MAGIC: &[u8; 4] = b"SBI\x1A";
/// Total fixed header size.
pub const HEADER: usize = 52;
/// Offset of the 16 OPL register bytes.
pub const REGS_AT: usize = 36;
/// Number of register bytes stored.
pub const REGS: usize = 16;

/// Parsed `.sbi` instrument.
#[derive(Debug, Clone, Copy)]
pub struct Sbi<'a> {
    /// Instrument name (NUL/space padded).
    pub name: &'a str,
    d: &'a [u8],
}

/// Parse; requires the magic and the full 52-byte header.
pub fn parse(d: &[u8]) -> Option<Sbi<'_>> {
    if d.get(..4)? != MAGIC || d.len() < HEADER {
        return None;
    }
    let s = &d[4..36];
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    Some(Sbi {
        name: std::str::from_utf8(&s[..end]).unwrap_or("").trim_end(),
        d,
    })
}

impl<'a> Sbi<'a> {
    /// OPL register byte `i` (0..15): modulator fields at 0..4,
    /// carrier at 5..9, feedback/connection at 10, waveform selects
    /// at 12 and 13.
    pub fn register(&self, i: usize) -> Option<u8> {
        if i >= REGS {
            return None;
        }
        self.d.get(REGS_AT + i).copied()
    }

    /// The `(modulator, carrier)` byte pair for operator field `i`
    /// (0..4): registers `i` and `i + 5`.
    pub fn carrier_modulator(&self, i: usize) -> Option<(u8, u8)> {
        if i > 4 {
            return None;
        }
        Some((self.register(i)?, self.register(i + 5)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[..4].copy_from_slice(MAGIC);
        d[4..9].copy_from_slice(b"Snare");
        for i in 0..REGS {
            d[REGS_AT + i] = i as u8;
        }
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let s = parse(&d).unwrap();
        assert_eq!(s.name, "Snare");
        assert_eq!(s.register(0), Some(0));
        assert_eq!(s.register(15), Some(15));
        assert_eq!(s.register(16), None);
        assert_eq!(s.carrier_modulator(0), Some((0, 5)));
        assert_eq!(s.carrier_modulator(4), Some((4, 9)));
        assert_eq!(s.carrier_modulator(5), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"SBI\x1A").is_none()); // no name/regs
        let mut d = fixture();
        d[3] = 0;
        assert!(parse(&d).is_none());
    }
}

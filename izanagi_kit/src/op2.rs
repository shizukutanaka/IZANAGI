//! GENMIDI.OP2 — Doom/DMX OPL instrument bank: `#O3_II#` magic,
//! 175 fixed 36-byte instrument records, then 175 NUL-padded
//! 32-byte names.
//!
//! Each record packs the OPL2 register set: flags, fine tuning,
//! fixed note, and two operator blocks (modulator then carrier:
//! tremolo/vibrato/sustain/KSR/multiple, KSL/output, attack/decay,
//! sustain/release, waveform select).
//!
//! ```
//! use izanagi_kit::op2::{parse, INSTRUMENTS};
//!
//! let mut d = vec![0u8; 8 + INSTRUMENTS * 36 + INSTRUMENTS * 32];
//! d[..7].copy_from_slice(b"#O3_II#");
//! let names_at = 8 + INSTRUMENTS * 36;
//! d[names_at..names_at + 5].copy_from_slice(b"Snare");
//! let b = parse(&d).unwrap();
//! assert_eq!(b.name(0), Some("Snare"));
//! assert!(b.instrument(174).is_some());
//! ```

/// Magic (7 bytes; records begin at offset 8).
pub const MAGIC: &[u8; 7] = b"#O3_II#";
/// Instrument count.
pub const INSTRUMENTS: usize = 175;
/// Bytes per instrument record.
pub const RECORD: usize = 36;
/// Bytes per name.
pub const NAME_LEN: usize = 32;

/// Parsed `.op2` bank.
#[derive(Debug, Clone, Copy)]
pub struct Op2<'a> {
    d: &'a [u8],
}

/// Parse; requires magic + all 175 records and names.
pub fn parse(d: &[u8]) -> Option<Op2<'_>> {
    if d.get(..7)? != MAGIC {
        return None;
    }
    let need = 8usize
        .checked_add(INSTRUMENTS * RECORD)?
        .checked_add(INSTRUMENTS * NAME_LEN)?;
    if d.len() < need {
        return None;
    }
    Some(Op2 { d })
}

impl<'a> Op2<'a> {
    /// Instrument record `i` (36 bytes).
    pub fn instrument(&self, i: usize) -> Option<&'a [u8]> {
        if i >= INSTRUMENTS {
            return None;
        }
        let at = 8 + i * RECORD;
        self.d.get(at..at + RECORD)
    }

    /// Instrument name `i`.
    pub fn name(&self, i: usize) -> Option<&'a str> {
        if i >= INSTRUMENTS {
            return None;
        }
        let at = 8 + INSTRUMENTS * RECORD + i * NAME_LEN;
        let s = self.d.get(at..at + NAME_LEN)?;
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        std::str::from_utf8(&s[..end]).ok().map(str::trim_end)
    }

    /// The record's two-byte flags word (double-voice, fixed note, …).
    pub fn flags(&self, i: usize) -> Option<u16> {
        let r = self.instrument(i)?;
        Some(u16::from(r[0]) | (u16::from(r[1]) << 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 8 + INSTRUMENTS * RECORD + INSTRUMENTS * NAME_LEN];
        d[..7].copy_from_slice(MAGIC);
        d[8] = 3; // instrument 0 flags lo
        let n = 8 + INSTRUMENTS * RECORD;
        d[n..n + 4].copy_from_slice(b"Kick");
        d[n + 32..n + 32 + 6].copy_from_slice(b"Hi-Hat");
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let b = parse(&d).unwrap();
        assert_eq!(b.flags(0), Some(3));
        assert_eq!(b.name(0), Some("Kick"));
        assert_eq!(b.name(1), Some("Hi-Hat"));
        assert_eq!(b.name(175), None);
        assert_eq!(b.instrument(174).unwrap().len(), RECORD);
        assert!(b.instrument(175).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(MAGIC).is_none());
        assert!(parse(&fixture()[..8 + 10]).is_none());
    }
}

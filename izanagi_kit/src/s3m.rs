//! Scream Tracker 3 module (`.s3m`).
//!
//! `name[28] | 0x1A | type(0x10) | u16 reserved | ordnum | insnum |
//! patnum | flags | cwtv | ffi(1) | "SCRM" | gv | is | it | mv | uc |
//! dp | reserved[10] | channels[32]` then the order table
//! (`ordnum` bytes, 0xFE = skip, 0xFF = end) followed by `insnum`
//! and `patnum` u16 parapointers — paragraph addresses that must be
//! shifted left 4 to get file offsets.
//!
//! ```
//! use izanagi_kit::s3m::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER];
//! d[0..4].copy_from_slice(b"SONG");
//! d[28] = 0x1A;
//! d[29] = 0x10;
//! d[32..34].copy_from_slice(&1u16.to_le_bytes()); // ordnum
//! d[42..44].copy_from_slice(&1u16.to_le_bytes()); // ffi
//! d[44..48].copy_from_slice(b"SCRM");
//! d.resize(HEADER + 1, 0);
//! let m = parse(&d).unwrap();
//! assert_eq!(m.title, "SONG");
//! assert_eq!(m.orders(&d), [0]);
//! ```

use std::string::String;
use std::vec::Vec;

/// Fixed header size (through the channel table).
pub const HEADER: usize = 96;
/// Order byte marking the end of the list.
pub const ORDER_END: u8 = 0xFF;
/// Order byte marking a skipped slot.
pub const ORDER_SKIP: u8 = 0xFE;
/// Channels table size.
pub const CHANNEL_TABLE: usize = 32;

/// A parsed S3M module header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3m {
    /// Song title (28 bytes, NUL-trimmed).
    pub title: String,
    /// Order count.
    pub ordnum: u16,
    /// Instrument / pattern counts.
    pub instruments: u16,
    /// Patterns.
    pub patterns: u16,
    /// Header flags.
    pub flags: u16,
    /// Tracker version.
    pub cwtv: u16,
    /// Global volume / initial speed / initial tempo / master volume.
    pub gv: u8,
    /// Initial speed.
    pub is: u8,
    /// Initial tempo.
    pub it: u8,
    /// Master volume.
    pub mv: u8,
    /// Channel settings table (`< 16` enabled, `0xFF` disabled).
    pub channels: [u8; CHANNEL_TABLE],
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

/// Where the order table starts (always [`HEADER`]).
fn orders_at() -> usize {
    HEADER
}

/// Parse the S3M header, or `None` on bad magic/type/`ffi`.
pub fn parse(d: &[u8]) -> Option<S3m> {
    if *d.get(28)? != 0x1A || *d.get(29)? != 0x10 || d.get(44..48)? != b"SCRM" {
        return None;
    }
    if u16le(d, 42)? != 1 {
        return None; // ffi must be 1 (signed samples historically)
    }
    let mut name = [0u8; 28];
    name.copy_from_slice(d.get(0..28)?);
    let end = name.iter().position(|&b| b == 0).unwrap_or(28);
    let mut channels = [0u8; CHANNEL_TABLE];
    channels.copy_from_slice(d.get(64..96)?);
    Some(S3m {
        title: String::from_utf8_lossy(&name[..end]).trim_end().to_string(),
        ordnum: u16le(d, 32)?,
        instruments: u16le(d, 34)?,
        patterns: u16le(d, 36)?,
        flags: u16le(d, 38)?,
        cwtv: u16le(d, 40)?,
        gv: *d.get(48)?,
        is: *d.get(49)?,
        it: *d.get(50)?,
        mv: *d.get(51)?,
        channels,
    })
}

impl S3m {
    /// Raw order bytes.
    pub fn orders<'a>(&self, d: &'a [u8]) -> &'a [u8] {
        d.get(orders_at()..orders_at() + usize::from(self.ordnum))
            .unwrap_or(&[])
    }

    /// Playable order list: stops at [`ORDER_END`], skips
    /// [`ORDER_SKIP`] slots.
    pub fn sequence(&self, d: &[u8]) -> Vec<u8> {
        self.orders(d)
            .iter()
            .take_while(|&&b| b != ORDER_END)
            .copied()
            .filter(|&b| b != ORDER_SKIP)
            .collect()
    }

    /// Channel `i` setting: `0..8` left, `8..16` right, `0xFF` off.
    pub fn chan(&self, i: usize) -> Option<u8> {
        self.channels.get(i).copied()
    }

    /// Whether channel `i` is enabled in the settings table.
    pub fn chan_on(&self, i: usize) -> bool {
        matches!(self.channels.get(i), Some(&c) if c < 16)
    }

    /// Instrument `i` parapointer → byte offset.
    pub fn instrument_ptr(&self, d: &[u8], i: usize) -> Option<u32> {
        let at = orders_at() + usize::from(self.ordnum) + i * 2;
        if i >= usize::from(self.instruments) {
            return None;
        }
        Some(u32::from(u16le(d, at)?) << 4)
    }

    /// Pattern `i` parapointer → byte offset.
    pub fn pattern_ptr(&self, d: &[u8], i: usize) -> Option<u32> {
        let at = orders_at() + usize::from(self.ordnum) + usize::from(self.instruments) * 2 + i * 2;
        if i >= usize::from(self.patterns) {
            return None;
        }
        Some(u32::from(u16le(d, at)?) << 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn module(ord: u16, ins: u16, pat: u16) -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[0..7].copy_from_slice(b"MODNAME");
        d[28] = 0x1A;
        d[29] = 0x10;
        d[32..34].copy_from_slice(&ord.to_le_bytes());
        d[34..36].copy_from_slice(&ins.to_le_bytes());
        d[36..38].copy_from_slice(&pat.to_le_bytes());
        d[38..40].copy_from_slice(&0x2000u16.to_le_bytes());
        d[40..42].copy_from_slice(&0x1320u16.to_le_bytes());
        d[42..44].copy_from_slice(&1u16.to_le_bytes());
        d[44..48].copy_from_slice(b"SCRM");
        d[48] = 64;
        d[49] = 6;
        d[50] = 125;
        d[51] = 176;
        d[64] = 0; // ch0 left
        d[65] = 9; // ch1 right
        d[66] = 0xFF; // disabled
        d.resize(
            HEADER + usize::from(ord) + 2 * (usize::from(ins) + usize::from(pat)),
            0,
        );
        d
    }

    #[test]
    fn header_fields() {
        let d = module(2, 1, 1);
        let m = parse(&d).unwrap();
        assert_eq!(m.title, "MODNAME");
        assert_eq!(m.ordnum, 2);
        assert_eq!(m.instruments, 1);
        assert_eq!(m.patterns, 1);
        assert_eq!(m.flags, 0x2000);
        assert_eq!(m.cwtv, 0x1320);
        assert_eq!(m.gv, 64);
        assert_eq!(m.is, 6);
        assert_eq!(m.it, 125);
        assert_eq!(m.mv, 176);
        assert_eq!(m.chan(0), Some(0));
        assert_eq!(m.chan(1), Some(9));
        assert!(m.chan_on(0));
        assert!(!m.chan_on(2));
        assert_eq!(m.chan(32), None);
    }

    #[test]
    fn orders_and_paraptrs() {
        let mut d = module(4, 1, 1);
        d[HEADER..HEADER + 4].copy_from_slice(&[0, 0xFE, 1, 0xFF]);
        // ins ptr @ 96+4 = 100: para 0x100 → byte 0x1000
        d[100..102].copy_from_slice(&0x100u16.to_le_bytes());
        // pat ptr @ 102: para 0x200 → 0x2000
        d[102..104].copy_from_slice(&0x200u16.to_le_bytes());
        let m = parse(&d).unwrap();
        assert_eq!(m.orders(&d), &[0, 0xFE, 1, 0xFF]);
        assert_eq!(m.sequence(&d), vec![0, 1]);
        assert_eq!(m.instrument_ptr(&d, 0), Some(0x1000));
        assert_eq!(m.pattern_ptr(&d, 0), Some(0x2000));
        assert_eq!(m.instrument_ptr(&d, 1), None);
        assert_eq!(m.pattern_ptr(&d, 1), None);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        for (i, v) in [(28, 0u8), (29, 0u8)] {
            let mut d = module(1, 0, 0);
            d[i] = v;
            assert_eq!(parse(&d), None);
        }
        let mut d = module(1, 0, 0);
        d[44] = b'X';
        assert_eq!(parse(&d), None);
        let mut d = module(1, 0, 0);
        d[42] = 2; // ffi != 1
        assert_eq!(parse(&d), None);
    }
}

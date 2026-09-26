//! Impulse Tracker module (`.it`).
//!
//! `IMPM | name[26] | highlight u16 | ordnum | ins | smp | pat |
//! cwtv | cmwt | flags | special | gv | mv | is | it | sep | pwd |
//! msglen u16 | msgoff u32 | reserved u32 | channel_pan[64] |
//! channel_vol[64] | orders[ordnum] | paraptrs…`. Parapointers are
//! absolute file offsets (not paragraphs — unlike S3M).
//!
//! ```
//! use izanagi_kit::it::parse;
//!
//! let mut d = vec![0u8; 196];
//! d[0..4].copy_from_slice(b"IMPM");
//! d[4..10].copy_from_slice(b"SONG\0\0");
//! d[32..34].copy_from_slice(&1u16.to_le_bytes());  // ordnum
//! d[40..42].copy_from_slice(&0x0214u16.to_le_bytes()); // cwtv
//! d[48] = 64;                                      // gv
//! d[192] = 0;                                      // order 0
//! let m = parse(&d).unwrap();
//! assert_eq!(m.title, "SONG");
//! assert_eq!(m.orders, [0]);
//! assert_eq!(m.pan(0), Some(0));
//! ```

use std::string::String;
use std::vec::Vec;

/// Fixed part of the header, before the order table.
pub const HEADER: usize = 192;
/// Channel count covered by pan/volume tables.
pub const CHANNELS: usize = 64;

/// A parsed IT module header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct It {
    /// Song name (26 bytes, NUL-trimmed).
    pub title: String,
    /// Order-table entry count.
    pub ordnum: u16,
    /// Instrument / sample / pattern counts.
    pub instruments: u16,
    /// Samples.
    pub samples: u16,
    /// Patterns.
    pub patterns: u16,
    /// "Created with tracker" version.
    pub cwtv: u16,
    /// "Compatible with" version.
    pub cmwt: u16,
    /// Header flags.
    pub flags: u16,
    /// Special flags (bit 0 = has message).
    pub special: u16,
    /// Global volume.
    pub gv: u8,
    /// Mix volume.
    pub mv: u8,
    /// Initial speed / tempo.
    pub initial_speed: u8,
    /// Initial tempo.
    pub initial_tempo: u8,
    /// Order table.
    pub orders: Vec<u8>,
    /// Instrument parapointers (file offsets).
    pub instrument_ptrs: Vec<u32>,
    /// Sample parapointers.
    pub sample_ptrs: Vec<u32>,
    /// Pattern parapointers.
    pub pattern_ptrs: Vec<u32>,
    /// Message text offset / length.
    pub message_at: u32,
    /// Message length.
    pub message_len: u16,
    /// Per-channel default pan (bit 7 = muted, 100 = surround).
    pub channel_pan: [u8; CHANNELS],
    /// Per-channel default volume.
    pub channel_vol: [u8; CHANNELS],
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

fn ptr_table(d: &[u8], at: usize, n: usize) -> Option<Vec<u32>> {
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        v.push(u32le(d, at + i * 4)?);
    }
    Some(v)
}

/// Parse the IT header, or `None` on bad magic or truncated tables.
pub fn parse(d: &[u8]) -> Option<It> {
    if d.get(..4)? != b"IMPM" {
        return None;
    }
    let mut name = [0u8; 26];
    name.copy_from_slice(d.get(4..30)?);
    let mut pan = [0u8; CHANNELS];
    pan.copy_from_slice(d.get(64..128)?);
    let mut vol = [0u8; CHANNELS];
    vol.copy_from_slice(d.get(128..192)?);
    let end = name.iter().position(|&b| b == 0).unwrap_or(26);
    let title = String::from_utf8_lossy(&name[..end]).trim_end().to_string();
    let ordnum = u16le(d, 32)?;
    let ins = u16le(d, 34)?;
    let smp = u16le(d, 36)?;
    let pat = u16le(d, 38)?;
    let mut at = HEADER;
    let orders = d.get(at..at + usize::from(ordnum))?.to_vec();
    at += usize::from(ordnum);
    let instrument_ptrs = ptr_table(d, at, usize::from(ins))?;
    at += usize::from(ins) * 4;
    let sample_ptrs = ptr_table(d, at, usize::from(smp))?;
    at += usize::from(smp) * 4;
    let pattern_ptrs = ptr_table(d, at, usize::from(pat))?;
    Some(It {
        title,
        ordnum,
        instruments: ins,
        samples: smp,
        patterns: pat,
        cwtv: u16le(d, 40)?,
        cmwt: u16le(d, 42)?,
        flags: u16le(d, 44)?,
        special: u16le(d, 46)?,
        gv: *d.get(48)?,
        mv: *d.get(49)?,
        initial_speed: *d.get(50)?,
        initial_tempo: *d.get(51)?,
        orders,
        instrument_ptrs,
        sample_ptrs,
        pattern_ptrs,
        message_at: u32le(d, 56)?,
        message_len: u16le(d, 54)?,
        channel_pan: pan,
        channel_vol: vol,
    })
}

impl It {
    /// Default pan for channel `i` (bit 7 set = muted).
    pub fn pan(&self, i: usize) -> Option<u8> {
        self.channel_pan.get(i).copied()
    }

    /// Default volume for channel `i`.
    pub fn volume(&self, i: usize) -> Option<u8> {
        self.channel_vol.get(i).copied()
    }

    /// Message bytes when the module carries one.
    pub fn message<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        let at = usize::try_from(self.message_at).ok()?;
        d.get(at..at + usize::from(self.message_len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn module(ordnum: u16, ins: u16, smp: u16, pat: u16) -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[0..4].copy_from_slice(b"IMPM");
        d[4..11].copy_from_slice(b"MODNAME");
        d[32..34].copy_from_slice(&ordnum.to_le_bytes());
        d[34..36].copy_from_slice(&ins.to_le_bytes());
        d[36..38].copy_from_slice(&smp.to_le_bytes());
        d[38..40].copy_from_slice(&pat.to_le_bytes());
        d[40..42].copy_from_slice(&0x0217u16.to_le_bytes());
        d[42..44].copy_from_slice(&0x0214u16.to_le_bytes());
        d[44..46].copy_from_slice(&9u16.to_le_bytes());
        d[48] = 128;
        d[49] = 48;
        d[50] = 6;
        d[51] = 125;
        d[64] = 32; // ch0 pan center
        d[128] = 64; // ch0 vol
        d.resize(
            HEADER
                + usize::from(ordnum)
                + 4 * (usize::from(ins) + usize::from(smp) + usize::from(pat)),
            0,
        );
        for (i, b) in d[HEADER..HEADER + usize::from(ordnum)]
            .iter_mut()
            .enumerate()
        {
            *b = i as u8;
        }
        d
    }

    #[test]
    fn header_fields() {
        let d = module(3, 1, 2, 4);
        let m = parse(&d).unwrap();
        assert_eq!(m.title, "MODNAME");
        assert_eq!(m.ordnum, 3);
        assert_eq!(m.instruments, 1);
        assert_eq!(m.samples, 2);
        assert_eq!(m.patterns, 4);
        assert_eq!(m.cwtv, 0x0217);
        assert_eq!(m.cmwt, 0x0214);
        assert_eq!(m.flags, 9);
        assert_eq!(m.gv, 128);
        assert_eq!(m.initial_speed, 6);
        assert_eq!(m.initial_tempo, 125);
        assert_eq!(m.orders, [0, 1, 2]);
        assert_eq!(m.instrument_ptrs.len(), 1);
        assert_eq!(m.sample_ptrs.len(), 2);
        assert_eq!(m.pattern_ptrs.len(), 4);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        let mut d = module(1, 0, 0, 0);
        d[0] = b'X';
        assert_eq!(parse(&d), None);
        let d = module(10, 0, 0, 0);
        let d = &d[..HEADER + 2]; // orders truncated
        assert_eq!(parse(d), None);
    }

    #[test]
    fn message_offsets() {
        let mut d = module(1, 0, 0, 0);
        d[54..56].copy_from_slice(&5u16.to_le_bytes());
        let msg_at = d.len();
        d[56..60].copy_from_slice(&(msg_at as u32).to_le_bytes());
        d.extend_from_slice(b"hello");
        let m = parse(&d).unwrap();
        assert_eq!(m.message_len, 5);
        assert_eq!(m.message_at as usize, msg_at);
        assert_eq!(m.message(&d), Some(b"hello".as_slice()));
        assert_eq!(m.pan(0), Some(32));
        assert_eq!(m.volume(0), Some(64));
        assert_eq!(m.pan(64), None);
    }
}

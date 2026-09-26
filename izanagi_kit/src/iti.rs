//! Impulse Instrument (`.iti`, IMPI) — 32-byte control header,
//! 26-byte name, MIDI pitch map and three envelopes, then `nos`
//! sample headers.
//!
//! Header layout (ITTECH.TXT): `IMPI` + dos filename (12) + flags
//! byte + nna/dct/dca + fadeout + pps/ppc + gbv + dfp + rv/rp +
//! tracker version + sample count + name.
//!
//! ```
//! use izanagi_kit::iti::parse;
//!
//! let mut d = vec![0u8; 0x20 + 26 + 2 + 120 * 2 + 82 * 3];
//! d[..4].copy_from_slice(b"IMPI");
//! d[4..4 + 8].copy_from_slice(b"LEAD.ITI");
//! d[17] = 1;          // nna = cut
//! d[22] = 0x80;       // pps
//! d[23] = 60;         // ppc
//! d[24] = 128;        // gbv
//! d[28] = 0x17; d[29] = 0x02; // trkvers 2.17
//! d[30] = 2;          // two samples
//! d[0x20..0x24].copy_from_slice(b"Lead");
//! let i = parse(&d).unwrap();
//! assert_eq!(i.name, "Lead");
//! assert_eq!(i.num_samples, 2);
//! assert_eq!(i.pitch_pan_center, 60);
//! ```

/// File magic.
pub const MAGIC: &[u8; 4] = b"IMPI";
/// Instrument name offset.
pub const NAME_AT: usize = 0x20;
/// Bytes the fixed header occupies before the MIDI map
/// (`0x20 + 26 + 2`  ifc/ifr/mch/mpr + mbank u16).
pub const FIXED: usize = 0x20 + 26 + 2 + 4;

/// NNA (New Note Action) value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nna {
    /// 0 — cut the old note.
    Cut,
    /// 1 — continue the old note.
    Continue,
    /// 2 — old note enters its release.
    Off,
    /// 3 — old note fades out.
    Fade,
    /// Anything else.
    Other(u8),
}

/// Parsed `.iti` instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iti<'a> {
    /// DOS filename field (12 bytes, NUL-padded).
    pub filename: &'a str,
    /// New Note Action.
    pub nna: Nna,
    /// Duplicate Check Type byte.
    pub dct: u8,
    /// Duplicate Check Action byte.
    pub dca: u8,
    /// Fadeout (0..8192 step 32).
    pub fadeout: u16,
    /// Pitch-Pan Separation.
    pub pitch_pan_separation: u8,
    /// Pitch-Pan Center note.
    pub pitch_pan_center: u8,
    /// Global volume (0..128).
    pub global_volume: u8,
    /// Default pan (bit7 set = no default).
    pub default_pan: u8,
    /// Random volume variation percent.
    pub random_volume: u8,
    /// Random pan variation percent.
    pub random_pan: u8,
    /// Tracker version that wrote the file.
    pub tracker_version: u16,
    /// Sample header count.
    pub num_samples: u8,
    /// Instrument name (26 bytes, NUL-padded).
    pub name: &'a str,
}

fn text(d: &[u8], at: usize, n: usize) -> &str {
    let s = d.get(at..at + n).unwrap_or(&[]);
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    std::str::from_utf8(&s[..end]).unwrap_or("").trim_end()
}

fn le16(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from(*d.get(i)?) | (u16::from(*d.get(i + 1)?) << 8))
}

/// Parse an `.iti` header.
pub fn parse(d: &[u8]) -> Option<Iti<'_>> {
    if d.get(..4)? != MAGIC || d.len() < FIXED {
        return None;
    }
    let nna = match d[17] {
        0 => Nna::Cut,
        1 => Nna::Continue,
        2 => Nna::Off,
        3 => Nna::Fade,
        v => Nna::Other(v),
    };
    Some(Iti {
        filename: text(d, 4, 12),
        nna,
        dct: *d.get(18)?,
        dca: *d.get(19)?,
        fadeout: le16(d, 20)?,
        pitch_pan_separation: *d.get(22)?,
        pitch_pan_center: *d.get(23)?,
        global_volume: *d.get(24)?,
        default_pan: *d.get(25)?,
        random_volume: *d.get(26)?,
        random_pan: *d.get(27)?,
        tracker_version: le16(d, 28)?,
        num_samples: *d.get(30)?,
        name: text(d, NAME_AT, 26),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; FIXED + 16];
        d[..4].copy_from_slice(MAGIC);
        d[4..4 + 8].copy_from_slice(b"LEAD.ITI");
        d[17] = 3;
        d[18] = 1;
        d[19] = 2;
        d[20] = 0x40;
        d[21] = 0x00;
        d[22] = 0x80;
        d[23] = 60;
        d[24] = 128;
        d[25] = 32;
        d[26] = 10;
        d[27] = 20;
        d[28] = 0x14;
        d[29] = 0x02;
        d[30] = 5;
        d[NAME_AT..NAME_AT + 4].copy_from_slice(b"Lead");
        d
    }

    #[test]
    fn fields() {
        let d = fixture();
        let i = parse(&d).unwrap();
        assert_eq!(i.filename, "LEAD.ITI");
        assert_eq!(i.nna, Nna::Fade);
        assert_eq!((i.dct, i.dca), (1, 2));
        assert_eq!(i.fadeout, 0x40);
        assert_eq!(i.pitch_pan_separation, 0x80);
        assert_eq!(i.pitch_pan_center, 60);
        assert_eq!(i.global_volume, 128);
        assert_eq!(i.default_pan, 32);
        assert_eq!((i.random_volume, i.random_pan), (10, 20));
        assert_eq!(i.tracker_version, 0x0214);
        assert_eq!(i.num_samples, 5);
        assert_eq!(i.name, "Lead");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"IMPI").is_none()); // short
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
    }
}

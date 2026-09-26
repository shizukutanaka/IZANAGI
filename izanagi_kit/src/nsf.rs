//! NSF — the NES Sound Format, a 128-byte header plus raw NES machine
//! code that a player bankswitches into place and calls at `init` /
//! `play`. Layout: `NESM\x1A` magic, version, song count, first song,
//! three LE u16 addresses, three 32-byte NUL-padded text fields
//! (title/author/copyright), NTSC and PAL play speeds in
//! microseconds, eight bankswitch registers, region byte, and the
//! extra-sound-chip bitmask.
//!
//! [`parse`] is total and rejects short files, a zero song count, an
//! out-of-range starting song, and addresses below `0x8000` (the spec
//! requires load/init/play to land in NES address space).
//!
//! ```
//! use izanagi_kit::nsf::{parse, Region, Chip};
//!
//! let mut d = b"NESM\x1A".to_vec();
//! d.push(1); // version
//! d.extend_from_slice(&[3, 2]); // songs, first song
//! for a in [0x8000u16, 0x8010, 0x8020] {
//!     d.extend_from_slice(&a.to_le_bytes());
//! }
//! d.extend_from_slice(b"Song name\0"); d.resize(46, 0);
//! d.extend_from_slice(b"Composer\0"); d.resize(78, 0);
//! d.extend_from_slice(b"2025\0"); d.resize(110, 0);
//! d.extend_from_slice(&16666u16.to_le_bytes()); // NTSC speed
//! d.extend_from_slice(&[0; 8]); // no bankswitching
//! d.extend_from_slice(&19997u16.to_le_bytes()); // PAL speed
//! d.push(0); // NTSC
//! d.push(0x01); // VRC6
//! d.resize(128, 0);
//! d.extend_from_slice(&[0xAA; 16]);
//!
//! let n = parse(&d).unwrap();
//! assert_eq!(n.songs, 3);
//! assert_eq!(n.region(), Region::Ntsc);
//! assert!(n.has_chip(Chip::Vrc6));
//! assert!(n.fits(d.len()));
//! ```

use std::string::String;

/// File size of the fixed header.
pub const HEADER: usize = 128;
/// Minimum legal address for load/init/play (NES address space).
pub const MIN_ADDR: u16 = 0x8000;

/// Broadcast region byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    /// NTSC timing only.
    Ntsc,
    /// PAL timing only.
    Pal,
    /// Both speeds valid.
    Dual,
    /// Anything else.
    Unknown(u8),
}

/// Extra sound chips from the bitmask at header byte 125.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    /// Konami VRC6.
    Vrc6,
    /// Konami VRC7.
    Vrc7,
    /// Famicom Disk System sound.
    Fds,
    /// Nintendo MMC5 audio.
    Mmc5,
    /// Namco 163.
    N163,
    /// Sunsoft 5B.
    Sunsoft5b,
}

/// A parsed NSF header.
#[derive(Clone, Debug, PartialEq)]
pub struct Nsf {
    /// Format version (normally 1).
    pub version: u8,
    /// Number of songs (1..=255).
    pub songs: u8,
    /// 1-based index of the first song to play.
    pub start_song: u8,
    /// Where the data is loaded in NES address space.
    pub load: u16,
    /// Init routine address (A = song-1, X = region).
    pub init: u16,
    /// Play routine address (called once per frame).
    pub play: u16,
    /// Song title.
    pub title: String,
    /// Author.
    pub author: String,
    /// Copyright string.
    pub copyright: String,
    /// NTSC play period in microseconds.
    pub ntsc_us: u16,
    /// PAL play period in microseconds.
    pub pal_us: u16,
    /// Region byte.
    pub region: u8,
    /// Extra-chip bitmask (bit 0 = VRC6 … bit 5 = 5B).
    pub extra_chips: u8,
    /// Bankswitch init values written to \$5FF8..\$5FFF.
    pub banks: [u8; 8],
}

fn text(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).into_owned()
}

fn u16le(d: &[u8], at: usize) -> u16 {
    u16::from(d[at]) | u16::from(d[at + 1]) << 8
}

/// Parse the header. `None` when the magic or structural fields are
/// wrong; the trailing program data itself is not validated.
pub fn parse(d: &[u8]) -> Option<Nsf> {
    if d.len() < HEADER || &d[..5] != b"NESM\x1A" {
        return None;
    }
    let version = d[5];
    let songs = d[6];
    let start_song = d[7];
    let load = u16le(d, 8);
    let init = u16le(d, 10);
    let play = u16le(d, 12);
    let title = text(&d[14..46]);
    let author = text(&d[46..78]);
    let copyright = text(&d[78..110]);
    let ntsc_us = u16le(d, 110);
    let mut banks = [0u8; 8];
    banks.copy_from_slice(&d[112..120]);
    let pal_us = u16le(d, 120);
    let region = d[122];
    let extra_chips = d[123];
    if version == 0
        || songs == 0
        || start_song == 0
        || start_song > songs
        || load < MIN_ADDR
        || init < MIN_ADDR
        || play < MIN_ADDR
    {
        return None;
    }
    Some(Nsf {
        version,
        songs,
        start_song,
        load,
        init,
        play,
        title,
        author,
        copyright,
        ntsc_us,
        pal_us,
        region,
        extra_chips,
        banks,
    })
}

impl Nsf {
    /// Region byte interpreted.
    pub fn region(&self) -> Region {
        match self.region {
            0 => Region::Ntsc,
            1 => Region::Pal,
            2 => Region::Dual,
            v => Region::Unknown(v),
        }
    }

    /// Whether an extra sound chip bit is set.
    pub fn has_chip(&self, c: Chip) -> bool {
        let bit = match c {
            Chip::Vrc6 => 0,
            Chip::Vrc7 => 1,
            Chip::Fds => 2,
            Chip::Mmc5 => 3,
            Chip::N163 => 4,
            Chip::Sunsoft5b => 5,
        };
        self.extra_chips & (1 << bit) != 0
    }

    /// Whether the file bankswitches (any nonzero init register).
    pub fn is_banked(&self) -> bool {
        self.banks.iter().any(|&b| b != 0)
    }

    /// First byte of program data.
    pub fn data_at(&self) -> usize {
        HEADER
    }

    /// End address in NES space for a data blob of `file_len` bytes.
    /// `None` when it overflows the 64K map.
    pub fn end(&self, file_len: usize) -> Option<u16> {
        let data = file_len.checked_sub(HEADER)? as u32;
        let end = u32::from(self.load) + data;
        if end > 0x1_0000 {
            return None;
        }
        Some(end as u16)
    }

    /// Whether the data fits the address space without banking.
    pub fn fits(&self, file_len: usize) -> bool {
        self.end(file_len).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn header() -> Vec<u8> {
        let mut d = b"NESM\x1A".to_vec();
        d.extend_from_slice(&[1, 3, 2]);
        for a in [0x8000u16, 0x8010, 0x8020] {
            d.extend_from_slice(&a.to_le_bytes());
        }
        d.extend_from_slice(b"Song name\0");
        d.resize(46, 0);
        d.extend_from_slice(b"Composer\0");
        d.resize(78, 0);
        d.extend_from_slice(b"2025\0");
        d.resize(110, 0);
        d.extend_from_slice(&16666u16.to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        d.extend_from_slice(&19997u16.to_le_bytes());
        d.extend_from_slice(&[0, 0x01]);
        d.resize(128, 0);
        d
    }

    #[test]
    fn fields_decode() {
        let mut d = header();
        d.extend_from_slice(&[0xAA; 16]);
        let n = parse(&d).unwrap();
        assert_eq!((n.version, n.songs, n.start_song), (1, 3, 2));
        assert_eq!((n.load, n.init, n.play), (0x8000, 0x8010, 0x8020));
        assert_eq!(
            (n.title.as_str(), n.author.as_str()),
            ("Song name", "Composer")
        );
        assert_eq!((n.ntsc_us, n.pal_us), (16666, 19997));
        assert_eq!(n.region(), Region::Ntsc);
        assert!(n.has_chip(Chip::Vrc6) && !n.has_chip(Chip::Vrc7));
        assert!(!n.is_banked());
        assert_eq!(n.end(d.len()), Some(0x8010));
        assert!(n.fits(d.len()));
    }

    #[test]
    fn rejects() {
        let d = header();
        assert!(parse(&d[..64]).is_none());
        let mut bad = d.clone();
        bad[0] = b'X';
        assert!(parse(&bad).is_none());
        let mut bad = d.clone();
        bad[6] = 0;
        assert!(parse(&bad).is_none());
        let mut bad = d.clone();
        bad[7] = 9; // start > songs
        assert!(parse(&bad).is_none());
        let mut bad = d;
        bad[8] = 0x00;
        bad[9] = 0x7F; // load < 0x8000
        assert!(parse(&bad).is_none());
    }

    #[test]
    fn dual_region_and_banked() {
        let mut d = header();
        d[112] = 1; // bank reg
        d[122] = 2;
        d[123] = 0x20; // 5B
        let n = parse(&d).unwrap();
        assert_eq!(n.region(), Region::Dual);
        assert!(n.is_banked());
        assert!(n.has_chip(Chip::Sunsoft5b));
        // too much data to fit
        assert!(!n.fits(128 + 0x8100));
    }
}

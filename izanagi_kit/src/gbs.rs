//! GBS — the GameBoy Sound System dump format. A 112-byte header
//! (`GBS` + version + counts + four LE addresses + timer regs + three
//! 32-byte text fields) followed by GameBoy machine code loaded at
//! `load` and driven through `init`/`play`; the hardware timer fields
//! `tma`/`tac` select a periodic interrupt instead of vblank when bit
//! 7 of `tac` is set.
//!
//! [`parse`] validates the magic, a nonzero song count, and a
//! starting song inside it.
//!
//! ```
//! use izanagi_kit::gbs::parse;
//!
//! let mut d = b"GBS\x01".to_vec();           // magic + version
//! d.extend_from_slice(&[5, 1]);              // songs, first
//! for a in [0x4000u16, 0x4000, 0x4010, 0xFFFE] {
//!     d.extend_from_slice(&a.to_le_bytes()); // load, init, play, sp
//! }
//! d.extend_from_slice(&[0x80, 0x00]);        // tma, tac (off)
//! d.extend_from_slice(b"GB demo\0"); d.resize(48, 0);
//! d.extend_from_slice(b"Dev\0"); d.resize(80, 0);
//! d.extend_from_slice(b"2025\0"); d.resize(112, 0);
//! d.extend_from_slice(&[0xC9; 4]);
//!
//! let g = parse(&d).unwrap();
//! assert_eq!((g.load, g.sp), (0x4000, 0xFFFE));
//! assert!(!g.uses_timer());
//! assert_eq!(g.data_at(), 112);
//! ```

use std::string::String;

/// Header length; code follows.
pub const HEADER: usize = 112;

/// A parsed GBS header.
#[derive(Clone, Debug, PartialEq)]
pub struct Gbs {
    /// Format version (normally 1).
    pub version: u8,
    /// Total songs (1..=255).
    pub songs: u8,
    /// 1-based first song.
    pub first_song: u8,
    /// Load address in GB space.
    pub load: u16,
    /// Init routine (A = song-1).
    pub init: u16,
    /// Play routine (vblank or timer).
    pub play: u16,
    /// Initial stack pointer.
    pub sp: u16,
    /// Timer modulo register value.
    pub tma: u8,
    /// Timer control value (bit 7 enables timer-driven play).
    pub tac: u8,
    /// Title.
    pub title: String,
    /// Author.
    pub author: String,
    /// Copyright.
    pub copyright: String,
}

fn text(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).into_owned()
}

fn u16le(d: &[u8], at: usize) -> u16 {
    u16::from(d[at]) | u16::from(d[at + 1]) << 8
}

/// Parse the 112-byte header.
pub fn parse(d: &[u8]) -> Option<Gbs> {
    if d.len() < HEADER || &d[..3] != b"GBS" {
        return None;
    }
    let version = d[3];
    let songs = d[4];
    let first_song = d[5];
    let g = Gbs {
        version,
        songs,
        first_song,
        load: u16le(d, 6),
        init: u16le(d, 8),
        play: u16le(d, 10),
        sp: u16le(d, 12),
        tma: d[14],
        tac: d[15],
        title: text(&d[16..48]),
        author: text(&d[48..80]),
        copyright: text(&d[80..112]),
    };
    if g.version == 0 || g.songs == 0 || g.first_song == 0 || g.first_song > g.songs {
        return None;
    }
    Some(g)
}

impl Gbs {
    /// Timer-driven playback requested (`tac` bit 7).
    pub fn uses_timer(&self) -> bool {
        self.tac & 0x80 != 0
    }

    /// GameBoy timer frequency selector (`tac` low 2 bits):
    /// 0 = 4096 Hz, 1 = 262144, 2 = 65536, 3 = 16384.
    pub fn timer_hz(&self) -> u32 {
        match self.tac & 3 {
            0 => 4096,
            1 => 262_144,
            2 => 65_536,
            _ => 16_384,
        }
    }

    /// Byte offset of the machine code.
    pub fn data_at(&self) -> usize {
        HEADER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn header() -> Vec<u8> {
        let mut d = b"GBS\x01".to_vec();
        d.extend_from_slice(&[5, 1]);
        for a in [0x4000u16, 0x4000, 0x4010, 0xFFFE] {
            d.extend_from_slice(&a.to_le_bytes());
        }
        d.extend_from_slice(&[0x80, 0x00]);
        d.extend_from_slice(b"GB demo\0");
        d.resize(48, 0);
        d.extend_from_slice(b"Dev\0");
        d.resize(80, 0);
        d.extend_from_slice(b"2025\0");
        d.resize(112, 0);
        d
    }

    #[test]
    fn fields_decode() {
        let mut d = header();
        d.extend_from_slice(&[0xC9; 4]);
        let g = parse(&d).unwrap();
        assert_eq!((g.version, g.songs, g.first_song), (1, 5, 1));
        assert_eq!(
            (g.load, g.init, g.play, g.sp),
            (0x4000, 0x4000, 0x4010, 0xFFFE)
        );
        assert_eq!((g.tma, g.tac), (0x80, 0x00));
        assert_eq!(g.title.as_str(), "GB demo");
        assert!(!g.uses_timer());
        assert_eq!(g.data_at(), 112);
    }

    #[test]
    fn timer_modes() {
        let mut d = header();
        d[15] = 0x80 | 0x01; // enable + 262144 Hz
        let g = parse(&d).unwrap();
        assert!(g.uses_timer());
        assert_eq!(g.timer_hz(), 262_144);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"GB").is_none());
        let mut d = header();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = header();
        d[4] = 0;
        assert!(parse(&d).is_none());
        let mut d = header();
        d[5] = 6; // first > songs
        assert!(parse(&d).is_none());
        let mut d = header();
        d[3] = 0; // version 0
        assert!(parse(&d).is_none());
    }
}

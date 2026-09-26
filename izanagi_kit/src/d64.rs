//! Commodore 1541 disk image (`.d64`).
//!
//! A standard 35-track image is `683` sectors of `256` bytes
//! (`174848` bytes); variants append a 683-byte error map or extend to
//! 40 tracks. The BAM lives on track 18 sector 0 and the directory
//! chains from track 18 sector 1, eight 32-byte entries per sector.
//!
//! ```
//! use izanagi_kit::d64::{parse, dir, SECTOR};
//!
//! let mut d = vec![0u8; 683 * SECTOR];
//! // BAM: link -> 18/1, DOS version 'A', disk name, DOS type "2A"
//! let f = parse(&d).unwrap();
//! let bam_at = f.sector_at(18, 0).unwrap();
//! d[bam_at] = 18; d[bam_at + 1] = 1;      // dir chain starts 18/1
//! d[bam_at + 2] = b'A';                   // DOS version
//! d[bam_at + 0x90..bam_at + 0x99].copy_from_slice(b"TEST DISK");
//! // one directory sector at 18/1 with a PRG entry
//! let dir0 = f.sector_at(18, 1).unwrap();
//! d[dir0] = 0; d[dir0 + 1] = 0xFF;        // last sector
//! d[dir0 + 2] = 0x82;                     // filetype: PRG (closed)
//! d[dir0 + 3] = 19; d[dir0 + 4] = 0;      // start track/sector
//! d[dir0 + 5..dir0 + 21].copy_from_slice(&[0xA0; 16]);
//! d[dir0 + 5..dir0 + 10].copy_from_slice(b"HELLO");
//! d[dir0 + 30..dir0 + 32].copy_from_slice(&3u16.to_le_bytes());
//! let names: Vec<_> = dir(&d).map(|e| (e.name, e.sectors)).collect();
//! assert_eq!(names, [(String::from("HELLO"), 3)]);
//! ```

use std::string::String;

/// Bytes per sector.
pub const SECTOR: usize = 256;
/// Standard track count.
pub const TRACKS: usize = 35;
/// Extended image track count.
pub const TRACKS_40: usize = 40;
/// Directory track.
pub const DIR_TRACK: usize = 18;

/// Sectors on each 1541 track (1-based index passed in).
pub fn sectors_of(track: usize) -> usize {
    match track {
        1..=17 => 21,
        18..=24 => 19,
        25..=30 => 18,
        31..=35 => 17,
        36..=40 => 17,
        _ => 0,
    }
}

fn total_sectors(tracks: usize) -> usize {
    (1..=tracks).map(sectors_of).sum()
}

/// A parsed `.d64` image geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct D64 {
    /// Track count (35 or 40).
    pub tracks: usize,
    /// Whether the trailing 683-byte error map is present.
    pub has_error_map: bool,
}

impl D64 {
    /// File offset of `(track, sector)` or `None` when out of range.
    pub fn sector_at(&self, track: usize, sector: usize) -> Option<usize> {
        if track == 0 || track > self.tracks || sector >= sectors_of(track) {
            return None;
        }
        let before: usize = (1..track).map(sectors_of).sum();
        Some((before + sector) * SECTOR)
    }
}

/// Parse an image's geometry. `None` when the size matches no known
/// `.d64` variant.
pub fn parse(d: &[u8]) -> Option<D64> {
    let s35 = total_sectors(TRACKS) * SECTOR; // 174848
    let s40 = total_sectors(TRACKS_40) * SECTOR; // 196608
    match d.len() {
        n if n == s35 => Some(D64 {
            tracks: TRACKS,
            has_error_map: false,
        }),
        n if n == s35 + 683 => Some(D64 {
            tracks: TRACKS,
            has_error_map: true,
        }),
        n if n == s40 => Some(D64 {
            tracks: TRACKS_40,
            has_error_map: false,
        }),
        n if n == s40 + 683 => Some(D64 {
            tracks: TRACKS_40,
            has_error_map: true,
        }),
        _ => None,
    }
}

fn petscii(d: &[u8]) -> String {
    d.iter()
        .map(|&b| match b {
            0xA0 | 0x00 => ' ',
            // unshifted PETSCII 0x41-0x5A renders as ASCII uppercase
            0x41..=0x5A => b as char,
            // shifted letters 0xC1-0xDA map to ASCII lowercase
            0xC1..=0xDA => (b - 0x60) as char,
            0x20..=0x3F | 0x5B..=0x5F => b as char,
            _ => '?',
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Disk name from BAM (16 PETSCII bytes at +0x90, 0xA0-padded).
pub fn disk_name(f: &D64, d: &[u8]) -> Option<String> {
    let at = f.sector_at(DIR_TRACK, 0)? + 0x90;
    Some(petscii(d.get(at..at + 16)?))
}

/// DOS type bytes from BAM (`0xA5..0xA7`, usually `"2A"`).
pub fn dos_type(f: &D64, d: &[u8]) -> Option<String> {
    let at = f.sector_at(DIR_TRACK, 0)? + 0xA5;
    Some(String::from_utf8_lossy(d.get(at..at + 2)?).into_owned())
}

/// Free-sector count on `track` from the BAM entry.
pub fn free_sectors(f: &D64, d: &[u8], track: usize) -> Option<u8> {
    let at = f.sector_at(DIR_TRACK, 0)? + 4 + (track.checked_sub(1)?) * 4;
    d.get(at).copied()
}

/// One directory entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Raw file-type byte.
    pub file_type: u8,
    /// File kind (`Del`, `Seq`, `Prg`, `Usr`, `Rel`, `Other`).
    pub kind: Kind,
    /// True when the closed-bit (0x80) is set.
    pub closed: bool,
    /// Starting track.
    pub track: u8,
    /// Starting sector.
    pub sector: u8,
    /// PETSCII filename (trimmed).
    pub name: String,
    /// Size in sectors.
    pub sectors: u16,
}

/// File kind decoded from the low file-type bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Deleted slot.
    Del,
    /// Sequential file.
    Seq,
    /// Program file.
    Prg,
    /// User file.
    Usr,
    /// Relative file.
    Rel,
    /// Unknown type bits.
    Other(u8),
}

fn kind(b: u8) -> Kind {
    match b & 0x07 {
        0 => Kind::Del,
        1 => Kind::Seq,
        2 => Kind::Prg,
        3 => Kind::Usr,
        4 => Kind::Rel,
        v => Kind::Other(v),
    }
}

/// Iterator over directory entries, following the link chain.
/// Non-deleted entries are yielded; malformed links end the walk.
pub fn dir(d: &[u8]) -> Dir<'_> {
    let f = match parse(d) {
        Some(f) => f,
        None => D64 {
            tracks: 0,
            has_error_map: false,
        },
    };
    Dir {
        f,
        d,
        track: DIR_TRACK as u8,
        sector: 1,
        entry: 0,
        seen: 0,
    }
}

/// See [`dir`].
#[derive(Clone)]
pub struct Dir<'a> {
    f: D64,
    d: &'a [u8],
    track: u8,
    sector: u8,
    entry: usize,
    seen: usize,
}

impl<'a> Iterator for Dir<'a> {
    type Item = Entry;
    fn next(&mut self) -> Option<Entry> {
        loop {
            if self.track == 0 || self.seen > self.f.tracks * 24 {
                return None;
            }
            let base = self
                .f
                .sector_at(usize::from(self.track), usize::from(self.sector))?;
            if self.entry == 8 {
                // chain link = bytes 0/1 of the sector
                self.track = *self.d.get(base)?;
                self.sector = *self.d.get(base + 1)?;
                self.entry = 0;
                self.seen += 1;
                continue;
            }
            let e = base + self.entry * 32;
            let ft = *self.d.get(e + 2)?;
            self.entry += 1;
            if kind(ft) == Kind::Del {
                continue;
            }
            return Some(Entry {
                file_type: ft,
                kind: kind(ft),
                closed: ft & 0x80 != 0,
                track: *self.d.get(e + 3)?,
                sector: *self.d.get(e + 4)?,
                name: petscii(self.d.get(e + 5..e + 21)?),
                sectors: u16::from(*self.d.get(e + 30)?) | u16::from(*self.d.get(e + 31)?) << 8,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn sectors_per_track_table() {
        assert_eq!(sectors_of(0), 0);
        assert_eq!(sectors_of(1), 21);
        assert_eq!(sectors_of(17), 21);
        assert_eq!(sectors_of(18), 19);
        assert_eq!(sectors_of(24), 19);
        assert_eq!(sectors_of(25), 18);
        assert_eq!(sectors_of(30), 18);
        assert_eq!(sectors_of(31), 17);
        assert_eq!(sectors_of(40), 17);
        assert_eq!(sectors_of(41), 0);
        assert_eq!(total_sectors(35), 683);
        assert_eq!(total_sectors(40), 768);
    }

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; total_sectors(TRACKS) * SECTOR];
        let f = parse(&d).unwrap();
        let bam = f.sector_at(18, 0).unwrap();
        d[bam] = 18;
        d[bam + 1] = 1;
        d[bam + 2] = b'A';
        d[bam + 0x90..bam + 0xA0].copy_from_slice(&[0xA0; 16]);
        d[bam + 0x90..bam + 0x99].copy_from_slice(b"TEST DISK");
        d[bam + 0xA5..bam + 0xA7].copy_from_slice(b"2A");
        let dir0 = f.sector_at(18, 1).unwrap();
        d[dir0] = 0;
        d[dir0 + 1] = 0xFF;
        // entry 0: closed PRG "HELLO"
        d[dir0 + 2] = 0x82;
        d[dir0 + 3] = 19;
        d[dir0 + 4] = 0;
        d[dir0 + 5..dir0 + 21].copy_from_slice(&[0xA0; 16]);
        d[dir0 + 5..dir0 + 10].copy_from_slice(b"HELLO");
        d[dir0 + 30..dir0 + 32].copy_from_slice(&3u16.to_le_bytes());
        // entry 1: deleted slot (skipped)
        d[dir0 + 32 + 2] = 0x00;
        // entry 2: SEQ "DATA"
        d[dir0 + 64 + 2] = 0x81;
        d[dir0 + 64 + 3] = 20;
        d[dir0 + 64 + 4] = 1;
        d[dir0 + 64 + 5..dir0 + 64 + 21].copy_from_slice(&[0xA0; 16]);
        d[dir0 + 64 + 5..dir0 + 64 + 9].copy_from_slice(b"DATA");
        d[dir0 + 64 + 30..dir0 + 64 + 32].copy_from_slice(&1u16.to_le_bytes());
        d
    }

    #[test]
    fn geometry() {
        let d = image();
        let f = parse(&d).unwrap();
        assert_eq!(f.tracks, 35);
        assert!(!f.has_error_map);
        assert_eq!(f.sector_at(1, 0), Some(0));
        assert_eq!(f.sector_at(18, 0), Some(17 * 21 * SECTOR));
        assert_eq!(f.sector_at(18, 18), Some((17 * 21 + 18) * SECTOR));
        assert_eq!(f.sector_at(18, 19), None);
        assert_eq!(f.sector_at(0, 0), None);
        assert_eq!(f.sector_at(36, 0), None);
    }

    #[test]
    fn bam_fields() {
        let d = image();
        let f = parse(&d).unwrap();
        assert_eq!(disk_name(&f, &d).as_deref(), Some("TEST DISK"));
        assert_eq!(dos_type(&f, &d).as_deref(), Some("2A"));
        assert_eq!(free_sectors(&f, &d, 1), Some(0));
    }

    #[test]
    fn dir_entries() {
        let d = image();
        let got: Vec<Entry> = dir(&d).collect();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "HELLO");
        assert_eq!(got[0].kind, Kind::Prg);
        assert!(got[0].closed);
        assert_eq!(got[0].sectors, 3);
        assert_eq!((got[0].track, got[0].sector), (19, 0));
        assert_eq!(got[1].name, "DATA");
        assert_eq!(got[1].kind, Kind::Seq);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 1000]).is_none());
        assert!(parse(&vec![0u8; total_sectors(TRACKS) * SECTOR + 1]).is_none());
        // error-map variant parses
        let d = vec![0u8; total_sectors(TRACKS) * SECTOR + 683];
        assert!(parse(&d).unwrap().has_error_map);
        // 40-track variant
        let d = vec![0u8; total_sectors(TRACKS_40) * SECTOR];
        assert_eq!(parse(&d).unwrap().tracks, 40);
    }
}

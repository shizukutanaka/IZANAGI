//! PSID/RSID — Commodore 64 SID music files. The header is big-endian:
//! `PSID` or `RSID`, version u16 (1–4), data offset u16, load/init/play
//! addresses u16, song count and start, a u32 `speed` bitfield (bit *i*
//! set ⇒ song *i+1* is driven by the CIA timer rather than the raster),
//! three 32-byte text fields, and — from version 2 — flags
//! (SID model per instance), start page, page length and second/third
//! SID addresses.
//!
//! A `load` of 0 means the real load address is the first two bytes of
//! the program data, little-endian. `RSID` is the strict variant: it
//! requires version ≥ 2 and load/init/play all zero.
//!
//! ```
//! use izanagi_kit::psid::{parse, Kind, SidModel};
//!
//! let mut d = b"PSID".to_vec();
//! for w in [2u16, 0x7C, 0x1000, 0x1003, 0x1006, 1, 1] {
//!     d.extend_from_slice(&w.to_be_bytes()); // BE header
//! }
//! d.extend_from_slice(&0u32.to_be_bytes());  // speed bits
//! d.extend_from_slice(b"Tune\0"); d.resize(54, 0);
//! d.extend_from_slice(b"Musician\0"); d.resize(86, 0);
//! d.extend_from_slice(b"2025\0"); d.resize(118, 0);
//! d.extend_from_slice(&0x0020u16.to_be_bytes()); // flags: 8580
//! d.extend_from_slice(&[0, 0, 0, 0]);            // v2 extras
//! let p = parse(&d).unwrap();
//! assert_eq!(p.kind(), Kind::Psid);
//! assert_eq!(p.sid_model(0), SidModel::Mos8580);
//! assert_eq!(p.load, Some(0x1000));
//! ```

use std::string::String;

/// Header size of a version-1 file.
pub const HEADER_V1: u16 = 0x76;
/// Header size from version 2 onward.
pub const HEADER_V2: u16 = 0x7C;

/// Which magic the file carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `PSID` — playable with raster or CIA timing.
    Psid,
    /// `RSID` — real C64 environment required.
    Rsid,
}

/// SID chip model encoded in the flags field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidModel {
    /// Unspecified.
    Unknown,
    /// MOS 6581.
    Mos6581,
    /// MOS 8580.
    Mos8580,
    /// Either is acceptable.
    Both,
}

/// A parsed PSID/RSID header.
#[derive(Clone, Debug, PartialEq)]
pub struct Psid {
    /// `PSID` or `RSID`.
    pub kind: Kind,
    /// Format version (1..=4).
    pub version: u16,
    /// Byte offset of the C64 program data.
    pub data_at: u16,
    /// Load address; `None` means it is stored at the data start.
    pub load: Option<u16>,
    /// Init routine address.
    pub init: u16,
    /// Play routine address (0 ⇒ RSID-style, called by hardware IRQ).
    pub play: u16,
    /// Song count.
    pub songs: u16,
    /// 1-based starting song.
    pub start_song: u16,
    /// CIA-timer bit per song (bit 0 = song 1).
    pub speed: u32,
    /// Title.
    pub title: String,
    /// Author.
    pub author: String,
    /// Copyright.
    pub copyright: String,
    /// Flags word (v2+; 0 for v1).
    pub flags: u16,
    /// Start page (v3+; 0 = load anywhere).
    pub start_page: u8,
    /// Page length (v3+).
    pub page_length: u8,
    /// Second SID address byte (v3+; 0 = none).
    pub sid2: u8,
    /// Third SID address byte (v4; 0 = none).
    pub sid3: u8,
}

fn text(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).into_owned()
}

fn u16be(d: &[u8], at: usize) -> u16 {
    u16::from(d[at]) << 8 | u16::from(d[at + 1])
}

/// Parse a PSID or RSID file header.
pub fn parse(d: &[u8]) -> Option<Psid> {
    if d.len() < usize::from(HEADER_V1) {
        return None;
    }
    let kind = if &d[..4] == b"PSID" {
        Kind::Psid
    } else if &d[..4] == b"RSID" {
        Kind::Rsid
    } else {
        return None;
    };
    let version = u16be(d, 4);
    let data_at = u16be(d, 6);
    let load = u16be(d, 8);
    let init = u16be(d, 10);
    let play = u16be(d, 12);
    let songs = u16be(d, 14);
    let start_song = u16be(d, 16);
    let speed = u32::from(u16be(d, 18)) << 16 | u32::from(u16be(d, 20));
    let title = text(&d[22..54]);
    let author = text(&d[54..86]);
    let copyright = text(&d[86..118]);
    if version >= 2 && d.len() < usize::from(HEADER_V2) {
        return None;
    }
    let (flags, start_page, page_length, sid2, sid3) = if version >= 2 {
        (u16be(d, 118), d[120], d[121], d[122], d[123])
    } else {
        (0, 0, 0, 0, 0)
    };
    // Structural validation.
    if !(1..=4).contains(&version)
        || songs == 0
        || start_song == 0
        || start_song > songs
        || usize::from(data_at) > d.len()
        || (version == 1 && data_at != HEADER_V1)
        || (version >= 2 && data_at != HEADER_V2)
    {
        return None;
    }
    if kind == Kind::Rsid && (version < 2 || load != 0 || init != 0 || play != 0) {
        return None;
    }
    Some(Psid {
        kind,
        version,
        data_at,
        load: if load == 0 { None } else { Some(load) },
        init,
        play,
        songs,
        start_song,
        speed,
        title,
        author,
        copyright,
        flags,
        start_page,
        page_length,
        sid2,
        sid3,
    })
}

impl Psid {
    /// Which magic the file carried.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Whether song `i` (1-based) is CIA-timed.
    pub fn uses_cia(&self, song: u16) -> bool {
        if song == 0 || song > self.songs {
            return false;
        }
        self.speed & (1 << (song - 1)) != 0
    }

    /// SID model for instance `i` (0, 1, 2): flags bits 4-5, 6-7, 8-9.
    pub fn sid_model(&self, instance: u8) -> SidModel {
        let bits = (self.flags >> (4 + instance * 2)) & 3;
        match bits {
            1 => SidModel::Mos6581,
            2 => SidModel::Mos8580,
            3 => SidModel::Both,
            _ => SidModel::Unknown,
        }
    }

    /// Real load address — either the header field or the first two
    /// bytes of the program data (little-endian).
    pub fn file_load(&self, d: &[u8]) -> Option<u16> {
        if let Some(l) = self.load {
            return Some(l);
        }
        let at = usize::from(self.data_at);
        if d.len() < at + 2 {
            return None;
        }
        Some(u16::from(d[at]) | u16::from(d[at + 1]) << 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn v2() -> Vec<u8> {
        let mut d = b"PSID".to_vec();
        for w in [2u16, 0x7C, 0x1000, 0x1003, 0x1006, 2, 1] {
            d.extend_from_slice(&w.to_be_bytes());
        }
        d.extend_from_slice(&0x0000_0002u32.to_be_bytes()); // song 2 → CIA
        d.extend_from_slice(b"Tune\0");
        d.resize(54, 0);
        d.extend_from_slice(b"Musician\0");
        d.resize(86, 0);
        d.extend_from_slice(b"2025\0");
        d.resize(118, 0);
        d.extend_from_slice(&0x0020u16.to_be_bytes());
        d.extend_from_slice(&[0x20, 0x40, 0x50, 0x70]);
        d.extend_from_slice(&[0xAA; 8]);
        d
    }

    #[test]
    fn v2_fields() {
        let d = v2();
        let p = parse(&d).unwrap();
        assert_eq!(p.kind(), Kind::Psid);
        assert_eq!((p.version, p.data_at), (2, 0x7C));
        assert_eq!(p.load, Some(0x1000));
        assert_eq!((p.songs, p.start_song), (2, 1));
        assert!(!p.uses_cia(1) && p.uses_cia(2) && !p.uses_cia(3));
        assert_eq!(p.title.as_str(), "Tune");
        assert_eq!(p.sid_model(0), SidModel::Mos8580);
        assert_eq!(p.sid_model(1), SidModel::Unknown);
        assert_eq!(
            (p.start_page, p.page_length, p.sid2, p.sid3),
            (0x20, 0x40, 0x50, 0x70)
        );
        assert_eq!(p.file_load(&d), Some(0x1000));
    }

    #[test]
    fn v1_and_embedded_load() {
        let mut d = b"PSID".to_vec();
        for w in [1u16, 0x76, 0, 0x1000, 0x1003, 1, 1] {
            d.extend_from_slice(&w.to_be_bytes());
        }
        d.extend_from_slice(&0u32.to_be_bytes());
        d.resize(118, 0);
        d.resize(0x76, 0); // v1 header is shorter than the texts span
        let n = parse(&d).unwrap();
        assert_eq!((n.version, n.load), (1, None));
        // embedded LE load address at data start
        let mut d2 = d.clone();
        d2.extend_from_slice(&0x4321u16.to_le_bytes());
        assert_eq!(n.file_load(&d2), Some(0x4321));
    }

    #[test]
    fn rsid_rules() {
        let mut d = b"RSID".to_vec();
        for w in [2u16, 0x7C, 0, 0, 0, 1, 1] {
            d.extend_from_slice(&w.to_be_bytes());
        }
        d.extend_from_slice(&0u32.to_be_bytes());
        d.resize(118, 0);
        d.extend_from_slice(&[0u8; 6]);
        assert_eq!(parse(&d).unwrap().kind(), Kind::Rsid);
        let mut bad = d.clone();
        bad[12] = 1; // RSID must have play == 0
        assert!(parse(&bad).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = v2();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = v2();
        d[5] = 9; // version 9 out of range
        assert!(parse(&d).is_none());
        let mut d = v2();
        d[7] = 0x76; // v2 header size must be 0x7C
        assert!(parse(&d).is_none());
        let mut d = v2();
        d[15] = 0; // songs = 0
        d[17] = 0;
        assert!(parse(&d).is_none());
    }
}

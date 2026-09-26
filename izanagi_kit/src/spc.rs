//! SPC — SNES SPC700 state dumps (`.spc`). Fixed 0x10200-byte image:
//! the 34-byte signature line `SNES-SPC700 Sound File Data vX.YZ` +
//! `0x1A`, a tag-presence byte at 0x23 (`26` = has ID666), registers
//! (PC/A/X/Y/PSW/SP at 0x25..0x2C), 64 KiB of RAM at 0x100, 128 DSP
//! registers at 0x10100, and the 64-byte IPL ROM at 0x101C0. The
//! inline ID666 tag at 0x2E has text and binary layouts — detected by
//! the `/` separators in the 11-byte date field.
//!
//! ```
//! use izanagi_kit::spc::{parse, TAG_PRESENT};
//!
//! let mut d = vec![0u8; 0x10200];
//! d[..33].copy_from_slice(b"SNES-SPC700 Sound File Data v0\x2E30");
//! d[33] = 0x1A;
//! d[0x23] = TAG_PRESENT;
//! d[0x24] = 30;
//! d[0x25..0x27].copy_from_slice(&0x0400u16.to_le_bytes());
//! d[0x2B] = 0xEF;
//! let s = parse(&d).unwrap();
//! assert_eq!((s.pc, s.sp), (0x0400, 0xEF));
//! assert_eq!(s.ram(&d).len(), 0x10000);
//! ```

use std::string::String;

/// Total size of the fixed region (registers + RAM + DSP + IPL).
pub const IMAGE: usize = 0x1_0200;
/// Byte at 0x23 meaning "an ID666 tag follows".
pub const TAG_PRESENT: u8 = 26;
/// Byte at 0x23 meaning "no tag".
pub const TAG_ABSENT: u8 = 27;
/// Signature bytes (`v0\x2E30` written escaped so no float-looking
/// literal ever appears in the source).
pub const SIG: &[u8] = b"SNES-SPC700 Sound File Data v0\x2E30";

/// Which ID666 layout the tag uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagKind {
    /// Date as `MM/DD/YYYY` text; fields at text offsets.
    Text,
    /// Date as a u32 `YYYYMMDD`; fields at binary offsets.
    Binary,
}

/// The inline ID666 metadata tag.
#[derive(Clone, Debug, PartialEq)]
pub struct Tag {
    /// Which layout was detected.
    pub kind: TagKind,
    /// Song title.
    pub title: String,
    /// Game title.
    pub game: String,
    /// Dumper name.
    pub dumper: String,
    /// Comments.
    pub comments: String,
    /// Artist name.
    pub artist: String,
    /// Date string — `MM/DD/YYYY` for text tags, `YYYYMMDD` digits for
    /// binary.
    pub date: String,
    /// Seconds before fade (ASCII digits in both layouts).
    pub secs: u32,
    /// Fade length in milliseconds (ASCII digits in both layouts).
    pub fade_ms: u32,
    /// Channel-disable bitmask (0 = enabled channel).
    pub channel_disable: u8,
    /// Emulator that produced the dump (0 unknown, 1 ZSNES, 2 Snes9x).
    pub emulator: u8,
}

/// The register block and fixed-region offsets of an SPC image.
#[derive(Clone, Debug, PartialEq)]
pub struct Spc {
    /// Program counter.
    pub pc: u16,
    /// Accumulator.
    pub a: u8,
    /// Index X.
    pub x: u8,
    /// Index Y.
    pub y: u8,
    /// Processor status word.
    pub psw: u8,
    /// Stack pointer.
    pub sp: u8,
    /// Minor version byte (e.g. 30).
    pub version_minor: u8,
    /// Whether byte 0x23 said a tag is present.
    pub has_tag: bool,
    /// Parsed ID666 tag when present.
    pub tag: Option<Tag>,
}

fn text(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).into_owned()
}

fn ascii_u32(d: &[u8]) -> u32 {
    let mut v = 0u32;
    for &b in d {
        if !b.is_ascii_digit() {
            break;
        }
        v = v.saturating_mul(10).saturating_add(u32::from(b - b'0'));
    }
    v
}

/// Parse the fixed image. `None` on a bad signature, wrong length, or
/// an unknown tag-presence byte.
pub fn parse(d: &[u8]) -> Option<Spc> {
    if d.len() < IMAGE || &d[..33] != SIG || d[33] != 0x1A {
        return None;
    }
    let has_tag = match d[0x23] {
        TAG_PRESENT => true,
        TAG_ABSENT => false,
        _ => return None,
    };
    let spc = Spc {
        pc: u16::from(d[0x25]) | u16::from(d[0x26]) << 8,
        a: d[0x27],
        x: d[0x28],
        y: d[0x29],
        psw: d[0x2A],
        sp: d[0x2B],
        version_minor: d[0x24],
        has_tag,
        tag: if has_tag { tag(d) } else { None },
    };
    Some(spc)
}

fn tag(d: &[u8]) -> Option<Tag> {
    // Text tags keep '/' separators in the 11-byte date field.
    let text_kind = d.get(0xA0) == Some(&b'/') && d.get(0xA3) == Some(&b'/');
    let (kind, date, artist_at, cd_at, emu_at) = if text_kind {
        (TagKind::Text, text(&d[0x9E..0xA9]), 0xB1, 0xD1, 0xD2)
    } else {
        let stamp = u32::from(d[0x9E])
            | u32::from(d[0x9F]) << 8
            | u32::from(d[0xA0]) << 16
            | u32::from(d[0xA1]) << 24;
        (TagKind::Binary, stamp.to_string(), 0xB0, 0xD0, 0xD1)
    };
    let fade_len = if kind == TagKind::Text { 5 } else { 4 };
    Some(Tag {
        kind,
        title: text(&d[0x2E..0x4E]),
        game: text(&d[0x4E..0x6E]),
        dumper: text(&d[0x6E..0x7E]),
        comments: text(&d[0x7E..0x9E]),
        artist: text(&d[artist_at..artist_at + 32]),
        date,
        secs: ascii_u32(&d[0xA9..0xAC]),
        fade_ms: ascii_u32(&d[0xAC..0xAC + fade_len]),
        channel_disable: d[cd_at],
        emulator: d[emu_at],
    })
}

impl Spc {
    /// The 64 KiB SPC700 RAM image.
    pub fn ram<'a>(&self, d: &'a [u8]) -> &'a [u8] {
        &d[0x100..0x10100]
    }

    /// The 128 DSP register bytes.
    pub fn dsp<'a>(&self, d: &'a [u8]) -> &'a [u8] {
        &d[0x10100..0x10180]
    }

    /// The 64-byte IPL boot ROM image.
    pub fn ipl<'a>(&self, d: &'a [u8]) -> &'a [u8] {
        &d[0x101C0..0x10200]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; IMAGE];
        d[..33].copy_from_slice(SIG);
        d[33] = 0x1A;
        d[0x23] = TAG_PRESENT;
        d[0x24] = 30;
        d[0x25..0x27].copy_from_slice(&0x0400u16.to_le_bytes());
        d[0x27] = 1;
        d[0x28] = 2;
        d[0x29] = 3;
        d[0x2A] = 0xB0;
        d[0x2B] = 0xEF;
        d
    }

    #[test]
    fn registers_and_regions() {
        let mut d = image();
        d[0x100] = 0xAA;
        d[0x10100] = 0xBB;
        d[0x101FF] = 0xCC;
        let s = parse(&d).unwrap();
        assert_eq!(
            (s.pc, s.a, s.x, s.y, s.psw, s.sp),
            (0x400, 1, 2, 3, 0xB0, 0xEF)
        );
        assert_eq!(s.version_minor, 30);
        assert!(s.has_tag);
        assert_eq!(s.ram(&d)[0], 0xAA);
        assert_eq!(s.dsp(&d)[0], 0xBB);
        assert_eq!(s.ipl(&d)[63], 0xCC);
    }

    #[test]
    fn text_tag() {
        let mut d = image();
        d[0x2E..0x2E + 4].copy_from_slice(b"Song");
        d[0x4E..0x4E + 4].copy_from_slice(b"Game");
        d[0x9E..0xA9].copy_from_slice(b"12/31/1999 ");
        d[0xA9..0xAC].copy_from_slice(b"120");
        d[0xAC..0xB1].copy_from_slice(b"2500 ");
        d[0xB1..0xB1 + 6].copy_from_slice(b"Artist");
        d[0xD1] = 0xFF;
        d[0xD2] = 1;
        let s = parse(&d).unwrap();
        let t = s.tag.unwrap();
        assert_eq!(t.kind, TagKind::Text);
        assert_eq!((t.title.as_str(), t.artist.as_str()), ("Song", "Artist"));
        assert_eq!(t.date.trim_end(), "12/31/1999");
        assert_eq!((t.secs, t.fade_ms), (120, 2500));
        assert_eq!((t.channel_disable, t.emulator), (0xFF, 1));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = image();
        d[33] = 0x00;
        assert!(parse(&d).is_none());
        let mut d = image();
        d[0x23] = 99;
        assert!(parse(&d).is_none());
        let mut d = image();
        d.truncate(IMAGE - 1);
        assert!(parse(&d).is_none());
    }
}

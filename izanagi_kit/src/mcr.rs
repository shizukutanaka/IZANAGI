//! PlayStation 1 memory card image (`.mcr`) parsing.
//!
//! 128 KiB card = 16 blocks × 8 KiB. Block 0's frame 0 is the `MC`
//! header; block-0 frames 1..=15 are 128-byte directory entries —
//! one per usable block. Entry layout: `state u8`, `link u8`,
//! `size u32LE`, `next_block u16LE` (`0xFFFF` ends the chain),
//! `title[20]` and a trailing XOR checksum over bytes 0..127.
//!
//! ```
//! use izanagi_kit::mcr;
//! let mut d = vec![0u8; 131072];
//! d[0] = b'M'; d[1] = b'C';
//! // directory entry for block 1 at offset 128
//! d[128] = 0x51; // allocated, first/last block
//! d[132..136].copy_from_slice(&0x2000u32.to_le_bytes());
//! d[136..138].copy_from_slice(&0xFFFFu16.to_le_bytes());
//! let mut x = 0u8;
//! for b in &d[128..255] { x ^= *b; }
//! d[255] = x;
//! let m = mcr::parse(&d).unwrap();
//! assert_eq!(m.files.len(), 1);
//! ```

use std::string::String;
use std::vec::Vec;

/// Raw `.mcr` size.
pub const CARD_LEN: usize = 131072;
/// Blocks on the card (15 usable; block 0 is the header).
pub const USABLE_BLOCKS: u32 = 15;

/// Directory entry states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// 0x51 — first (and possibly only) block of a save.
    First,
    /// 0x52 — middle block of a multi-block save.
    Middle,
    /// 0x53 — last block.
    Last,
    /// 0xA0 — free, first block of a deleted slot.
    FreeFirst,
    /// 0xA1 — free middle.
    FreeMiddle,
    /// 0xA2 — free last.
    FreeLast,
    /// Anything else — unknown/corrupt.
    Unknown(u8),
}

fn state(b: u8) -> State {
    match b {
        0x51 => State::First,
        0x52 => State::Middle,
        0x53 => State::Last,
        0xA0 => State::FreeFirst,
        0xA1 => State::FreeMiddle,
        0xA2 => State::FreeLast,
        o => State::Unknown(o),
    }
}

/// One file chain found in the directory.
#[derive(Clone, Debug, PartialEq)]
pub struct Save {
    /// Directory slot of the first block (1..15).
    pub slot: u8,
    /// Declared `size` in bytes.
    pub size: u32,
    /// Shift-JIS-ish `title` bytes, trimmed at NUL.
    pub title: String,
    /// Block chain walked via `next_block`.
    pub chain: Vec<u16>,
}

/// A parsed card.
#[derive(Clone, Debug, PartialEq)]
pub struct Mcr {
    /// Directory entry states for blocks 1..=15.
    pub states: Vec<State>,
    /// File chains rooted at `0x51` entries.
    pub files: Vec<Save>,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) | (s[1] as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a raw `.mcr` image: exact 128 KiB, `MC` magic, XOR-verified
/// directory frames; chains of `0x51` entries become `Save`s.
pub fn parse(d: &[u8]) -> Option<Mcr> {
    if d.len() != CARD_LEN || d.get(..2)? != b"MC" {
        return None;
    }
    let mut states = Vec::with_capacity(15);
    let mut raw_entries: Vec<(u32, u16, String)> = Vec::with_capacity(15);
    for i in 0..15usize {
        let at = 128 + i * 128;
        let entry = d.get(at..at + 128)?;
        let mut x = 0u8;
        for &b in &entry[..127] {
            x ^= b;
        }
        if x != entry[127] {
            return None;
        }
        let st = state(entry[0]);
        let size = u32le(d, at + 4)?;
        let next = u16le(d, at + 8)?;
        let title_bytes = &entry[10..10 + 20];
        let end = title_bytes.iter().position(|&b| b == 0).unwrap_or(20);
        let title = String::from_utf8_lossy(&title_bytes[..end]).into_owned();
        states.push(st);
        raw_entries.push((size, next, title));
    }
    let mut files = Vec::new();
    for (i, st) in states.iter().enumerate() {
        if *st != State::First {
            continue;
        }
        let (size, mut next, title) = raw_entries[i].clone();
        let mut chain = vec![(i + 1) as u16];
        let mut guard = 0;
        while next != 0xFFFF && guard < 15 {
            let n = next as usize;
            if !(1..=15).contains(&n) {
                break;
            }
            chain.push(next);
            next = raw_entries[n - 1].1;
            guard += 1;
        }
        files.push(Save {
            slot: (i + 1) as u8,
            size,
            title,
            chain,
        });
    }
    Some(Mcr { states, files })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; CARD_LEN];
        d[0] = b'M';
        d[1] = b'C';
        for i in 0..15usize {
            let at = 128 + i * 128;
            d[at] = 0xA0; // free
            let mut x = 0u8;
            for b in &d[at..at + 127] {
                x ^= *b;
            }
            d[at + 127] = x;
        }
        d
    }

    #[test]
    fn parses_card() {
        let mut d = fixture();
        let at = 128;
        d[at] = 0x51;
        d[at + 4..at + 8].copy_from_slice(&0x4000u32.to_le_bytes());
        d[at + 8..at + 10].copy_from_slice(&0xFFFFu16.to_le_bytes());
        d[at + 10..at + 15].copy_from_slice(b"HELLO");
        let mut x = 0u8;
        for b in &d[at..at + 127] {
            x ^= *b;
        }
        d[at + 127] = x;
        let m = parse(&d).unwrap();
        assert_eq!(m.files.len(), 1);
        assert_eq!(m.files[0].title, "HELLO");
        assert_eq!(m.files[0].chain, vec![1]);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8192]).is_none()); // wrong size
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none()); // bad magic
        let mut d = fixture();
        d[255] ^= 0xFF; // corrupt checksum
        assert!(parse(&d).is_none());
    }
}

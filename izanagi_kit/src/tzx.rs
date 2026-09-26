//! ZX Spectrum tape image (`.tzx`) block walk.
//!
//! A TZX file is `ZXTape!\x1A` + major/minor version, then a sequence of
//! blocks. Each block is one ID byte followed by a body whose length is
//! carried differently per block type; this module exposes an iterator
//! that yields every block's id and payload bounds without decoding the
//! pulse-level data.
//!
//! ```
//! use izanagi_kit::tzx::{parse, blocks, DATA, GROUP_START};
//!
//! let mut d = b"ZXTape!\x1A".to_vec();
//! d.extend_from_slice(&[1, 20]); // v1.20
//! d.push(GROUP_START);           // group start: u8 len + name
//! d.extend_from_slice(&[3]); d.extend_from_slice(b"RUN");
//! d.push(DATA);                  // standard-speed data
//! d.extend_from_slice(&[0, 0]);  // pause 0 ms
//! d.extend_from_slice(&[2, 0]);  // len 2
//! d.extend_from_slice(&[0xFF, 0x00]);
//! let t = parse(&d).unwrap();
//! assert_eq!((t.major, t.minor), (1, 20));
//! let ids: Vec<u8> = blocks(&d).map(|b| b.id).collect();
//! assert_eq!(ids, [GROUP_START, DATA]);
//! ```

/// Header size (signature + version).
pub const HEADER: usize = 10;
/// Standard-speed data block.
pub const DATA: u8 = 0x10;
/// Turbo-speed data block.
pub const TURBO: u8 = 0x11;
/// Pure-tone block.
pub const TONE: u8 = 0x12;
/// Pulse sequence block.
pub const PULSES: u8 = 0x13;
/// Pure-data block.
pub const PURE_DATA: u8 = 0x14;
/// Generalised data block (u32 length field).
pub const GEN_DATA: u8 = 0x19;
/// Pause-tape block.
pub const PAUSE: u8 = 0x20;
/// Group-start block.
pub const GROUP_START: u8 = 0x21;
/// Group-end block.
pub const GROUP_END: u8 = 0x22;
/// Jump-to block.
pub const JUMP: u8 = 0x23;
/// Loop-start block.
pub const LOOP_START: u8 = 0x24;
/// Loop-end block.
pub const LOOP_END: u8 = 0x25;
/// Text-description block.
pub const TEXT: u8 = 0x30;
/// Message block.
pub const MESSAGE: u8 = 0x31;
/// Archive-info block.
pub const ARCHIVE: u8 = 0x32;
/// Hardware-type block.
pub const HARDWARE: u8 = 0x33;
/// Glue block (`XTape!` marker).
pub const GLUE: u8 = 0x5A;

/// A TZX file header.
#[derive(Clone, Debug, PartialEq)]
pub struct Tzx {
    /// Major version.
    pub major: u8,
    /// Minor version.
    pub minor: u8,
}

/// One block in the stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// Block ID byte.
    pub id: u8,
    /// Offset of the block body (after its length prefix, if any).
    pub at: usize,
    /// Body length in bytes.
    pub len: usize,
}

fn u16le(d: &[u8], at: usize) -> Option<usize> {
    Some(usize::from(*d.get(at)?) | usize::from(*d.get(at + 1)?) << 8)
}

fn u24le(d: &[u8], at: usize) -> Option<usize> {
    Some(
        usize::from(*d.get(at)?)
            | usize::from(*d.get(at + 1)?) << 8
            | usize::from(*d.get(at + 2)?) << 16,
    )
}

fn u32le(d: &[u8], at: usize) -> Option<usize> {
    Some(
        usize::from(*d.get(at)?)
            | usize::from(*d.get(at + 1)?) << 8
            | usize::from(*d.get(at + 2)?) << 16
            | usize::from(*d.get(at + 3)?) << 24,
    )
}

/// Parse the file header only. `None` when the signature is missing.
pub fn parse(d: &[u8]) -> Option<Tzx> {
    if d.len() < HEADER || &d[..8] != b"ZXTape!\x1A" {
        return None;
    }
    Some(Tzx {
        major: d[8],
        minor: d[9],
    })
}

/// Body (prefix_len, body_len) for the block whose id sits at `at`,
/// or `None` when the block is truncated or unknown.
fn block_extent(d: &[u8], at: usize) -> Option<(usize, usize)> {
    let id = *d.get(at)?;
    let p = at + 1;
    let extent = match id {
        DATA => (4, u16le(d, p + 2)?),
        TURBO => (18, u24le(d, p + 15)?),
        TONE => (4, 0),
        PULSES => (1, usize::from(*d.get(p)?) * 2),
        PURE_DATA => (10, u24le(d, p + 7)?),
        0x15 => (8, u24le(d, p + 5)?),
        0x18 => (4, u32le(d, p)?),
        GEN_DATA | 0x2B => (4, u32le(d, p)?),
        PAUSE => (2, 0),
        GROUP_START | TEXT => (1, usize::from(*d.get(p)?)),
        GROUP_END | LOOP_END | 0x27 => (0, 0),
        JUMP => (2, 0),
        LOOP_START => (2, 0),
        0x26 => (2, u16le(d, p)? * 2),
        0x28 => (2, u16le(d, p)?),
        0x2A => (4, u32le(d, p)?),
        MESSAGE => (2, usize::from(*d.get(p + 1)?)),
        ARCHIVE => (2, u16le(d, p)?),
        HARDWARE => (1, usize::from(*d.get(p)?) * 3),
        GLUE => (9, 0),
        _ => return None,
    };
    Some(extent)
}

/// Iterator over the block sequence (stops before any truncated or
/// unknown block).
pub fn blocks(d: &[u8]) -> Blocks<'_> {
    Blocks { d, at: HEADER }
}

/// See [`blocks`].
#[derive(Clone)]
pub struct Blocks<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block;
    fn next(&mut self) -> Option<Block> {
        let (prefix, len) = block_extent(self.d, self.at)?;
        let body = self.at + 1 + prefix;
        if body + len > self.d.len() {
            return None;
        }
        let b = Block {
            id: self.d[self.at],
            at: body,
            len,
        };
        self.at = body + len;
        Some(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn tape(blocks: &[u8]) -> Vec<u8> {
        let mut d = b"ZXTape!\x1A".to_vec();
        d.extend_from_slice(&[1, 20]);
        d.extend_from_slice(blocks);
        d
    }

    #[test]
    fn header_parse() {
        let t = parse(&tape(&[])).unwrap();
        assert_eq!((t.major, t.minor), (1, 20));
    }

    #[test]
    fn block_walk() {
        let mut t = Vec::new();
        t.push(GROUP_START);
        t.extend_from_slice(&[3]);
        t.extend_from_slice(b"RUN");
        t.push(DATA);
        t.extend_from_slice(&[100, 0, 4, 0]); // pause, len
        t.extend_from_slice(&[0xFF, 1, 2, 3]);
        t.push(PAUSE);
        t.extend_from_slice(&[0xE8, 0x03]);
        t.push(GROUP_END);
        let d = tape(&t);
        let got: Vec<(u8, usize)> = blocks(&d).map(|b| (b.id, b.len)).collect();
        assert_eq!(
            got,
            [(GROUP_START, 3), (DATA, 4), (PAUSE, 0), (GROUP_END, 0)]
        );
    }

    #[test]
    fn turbo_and_text() {
        let mut t = Vec::new();
        t.push(TEXT);
        t.extend_from_slice(&[4]);
        t.extend_from_slice(b"INFO");
        t.push(TURBO);
        t.extend_from_slice(&[0; 15]); // timing fields
        t.extend_from_slice(&[3, 0, 0]); // u24 len
        t.extend_from_slice(&[9, 9, 9]);
        let d = tape(&t);
        let got: Vec<(u8, usize)> = blocks(&d).map(|b| (b.id, b.len)).collect();
        assert_eq!(got, [(TEXT, 4), (TURBO, 3)]);
    }

    #[test]
    fn truncated_block_stops_iteration() {
        let mut t = Vec::new();
        t.push(DATA);
        t.extend_from_slice(&[0, 0, 10, 0]); // claims 10 body bytes
        t.extend_from_slice(&[1, 2]); // only 2 present
        let d = tape(&t);
        assert_eq!(blocks(&d).count(), 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"ZXTape!\x1B01").is_none());
        // unknown block id stops the walk
        let d = tape(&[0xFE, 0, 0, 0]);
        assert_eq!(blocks(&d).count(), 0);
    }
}

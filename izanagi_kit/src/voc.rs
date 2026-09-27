//! Creative Voice File (`.voc`) block stream.
//!
//! The 26-byte header is `"Creative Voice File"` + `0x1A`, then a
//! u16 LE data offset (usually 26), u16 version, and u16 check word
//! equal to `!version + 0x1234`. Blocks then follow as
//! `{type:u8, size:LE24, data}` until a type-0 terminator.
//!
//! ```
//! use izanagi_kit::voc::{parse, blocks, BlockKind};
//! let mut d = b"Creative Voice File\x1A".to_vec();
//! d.extend_from_slice(&[0x1A, 0x00, 0x14, 0x01]); // offset, v1.20
//! d.extend_from_slice(&[0x1F, 0x11]);            // !0x0114 + 0x1234 = 0x111F
//! d.extend_from_slice(&[5, 2, 0, 0]);            // text block, len 2
//! d.extend_from_slice(b"hi");
//! d.push(0);                                     // terminator
//! let v = parse(&d).unwrap();
//! let bs = blocks(&d, &v);
//! assert_eq!(bs[0].kind, BlockKind::Text);
//! ```

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some((*d.get(o)? as u16) | (*d.get(o + 1)? as u16) << 8)
}

fn le24(d: &[u8], o: usize) -> Option<u32> {
    Some((*d.get(o)? as u32) | (*d.get(o + 1)? as u32) << 8 | (*d.get(o + 2)? as u32) << 16)
}

/// Fixed banner including its `0x1A` byte.
pub const BANNER: &[u8; 20] = b"Creative Voice File\x1A";

/// Classified block type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// `0` — terminator (no size field).
    Terminator,
    /// `1` — sound data (time-constant byte + codec byte + samples).
    SoundData,
    /// `2` — continuation of the previous sound block.
    SoundContinue,
    /// `3` — silence period (u16 length + u8 time constant).
    Silence,
    /// `4` — playback-position marker (u16).
    Marker,
    /// `5` — ASCII annotation text.
    Text,
    /// `6`/`7` — repeat loop start / end (u16 count).
    Repeat,
    /// `7` — repeat loop end.
    EndRepeat,
    /// `8` — extended info (Voc 1.20+: time constant, codec, channels).
    Extra,
    /// `9` — sound data with new-format header (rate, bits, channels…).
    SoundNew,
    /// Any other block id.
    Other(u8),
}

/// One block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Classified type.
    pub kind: BlockKind,
    /// Declared data size in bytes (0 for the terminator).
    pub size: usize,
    /// Offset of the block data.
    pub at: usize,
}

/// Parsed VOC header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voc {
    /// Data offset (start of the block stream).
    pub offset: u16,
    /// Format version (`0x010A` or `0x0114`).
    pub version: u16,
    /// Check word — should equal `!version + 0x1234`; verified.
    pub check: u16,
}

impl Voc {
    /// `true` when `check == !version + 0x1234` (always true after
    /// `parse`; exposed for manual header construction).
    pub fn check_ok(&self) -> bool {
        self.check == (!self.version).wrapping_add(0x1234)
    }
}

/// Parse the 26-byte VOC header, verifying the banner and the
/// `check = !version + 0x1234` word. `None` otherwise.
pub fn parse(d: &[u8]) -> Option<Voc> {
    if d.get(0..20)? != BANNER {
        return None;
    }
    let offset = le16(d, 20)?;
    let version = le16(d, 22)?;
    let check = le16(d, 24)?;
    let v = Voc {
        offset,
        version,
        check,
    };
    if !v.check_ok() || offset as usize > d.len() || offset < 26 {
        return None;
    }
    Some(v)
}

/// Read the block header at `at`. `Terminator` blocks carry no size
/// and occupy one byte.
pub fn block_at(d: &[u8], at: usize) -> Option<Block> {
    let ty = *d.get(at)?;
    let kind = match ty {
        0 => BlockKind::Terminator,
        1 => BlockKind::SoundData,
        2 => BlockKind::SoundContinue,
        3 => BlockKind::Silence,
        4 => BlockKind::Marker,
        5 => BlockKind::Text,
        6 => BlockKind::Repeat,
        7 => BlockKind::EndRepeat,
        8 => BlockKind::Extra,
        9 => BlockKind::SoundNew,
        other => BlockKind::Other(other),
    };
    if kind == BlockKind::Terminator {
        return Some(Block { kind, size: 0, at });
    }
    let size = le24(d, at + 1)? as usize;
    let data = at.checked_add(4)?;
    if data.checked_add(size)? > d.len() {
        return None;
    }
    Some(Block {
        kind,
        size,
        at: data,
    })
}

/// Walk blocks from `v.offset` up to and including the terminator;
/// stops early on a malformed block.
pub fn blocks(d: &[u8], v: &Voc) -> Vec<Block> {
    let mut out = Vec::new();
    let mut at = v.offset as usize;
    while at < d.len() {
        match block_at(d, at) {
            Some(b) => {
                let done = b.kind == BlockKind::Terminator;
                let next = if done { at + 1 } else { b.at + b.size };
                out.push(b);
                if done {
                    break;
                }
                at = next;
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = BANNER.to_vec();
        // offset 0x1A, version 0x0114, check = !0x0114 + 0x1234 = 0x111F
        d.extend_from_slice(&[0x1A, 0x00, 0x14, 0x01, 0x1F, 0x11]);
        // silence block: 512 samples
        d.extend_from_slice(&[3, 3, 0, 0, 0x00, 0x02, 0xA6]);
        // text
        d.extend_from_slice(&[5, 2, 0, 0]);
        d.extend_from_slice(b"hi");
        // marker 0x0040
        d.extend_from_slice(&[4, 2, 0, 0, 0x40, 0x00]);
        d.push(0);
        d
    }

    #[test]
    fn header_and_blocks() {
        let d = fixture();
        let v = parse(&d).unwrap();
        assert_eq!(v.version, 0x0114);
        assert!(v.check_ok());
        let first = block_at(&d, usize::from(v.offset)).unwrap();
        assert_eq!(first.kind, BlockKind::Silence);
        let bs = blocks(&d, &v);
        assert_eq!(bs[0], first);
        assert_eq!(bs.len(), 4);
        assert_eq!(bs[0].kind, BlockKind::Silence);
        assert_eq!(bs[1].kind, BlockKind::Text);
        assert_eq!(&d[bs[1].at..bs[1].at + 2], b"hi");
        assert_eq!(bs[2].kind, BlockKind::Marker);
        assert_eq!(bs[3].kind, BlockKind::Terminator);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut bad = fixture();
        bad[0] = b'X';
        assert!(parse(&bad).is_none());
        // bad check word
        let mut bad2 = fixture();
        bad2[24] = 0;
        assert!(parse(&bad2).is_none());
        // bad offset
        let mut bad3 = fixture();
        bad3[20] = 0x10; // < 26
        assert!(parse(&bad3).is_none());
        // truncated mid-block
        let mut t = fixture();
        t.truncate(30);
        let v = parse(&t).unwrap();
        let bs = blocks(&t, &v);
        assert!(bs.is_empty() || bs.last().unwrap().kind != BlockKind::Terminator);
    }
}

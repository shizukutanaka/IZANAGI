//! RAR archive headers — RAR 1.5-4.x block chain and the RAR5
//! signature.
//!
//! RAR4 files open with `Rar!\x1A\x07\x00` followed by a chain of
//! blocks: `HEAD_CRC` u16, `HEAD_TYPE` u8 (`0x72` main, `0x73`
//! file, `0x74` comment, `0x75` old av, `0x76` old subblock,
//! `0x7A` new subblock, `0x7B` end), `HEAD_FLAGS` u16, `HEAD_SIZE`
//! u16, plus `ADD_SIZE` u32 when `LONG_BLOCK` (`0x8000`) is set.
//! RAR5 uses `Rar!\x1A\x07\x01\x00` and a varint-coded header —
//! only the signature/version is classified here.
//!
//! ```
//! use izanagi_kit::rar::{parse, Kind, TYPE_MAIN};
//!
//! let mut d = b"Rar!\x1a\x07\x00".to_vec();
//! // main header: crc, type 0x72, flags 0, size 13
//! d.extend_from_slice(&[0xcf, 0x90, TYPE_MAIN, 0, 0, 13, 0]);
//! d.extend_from_slice(&[0; 6]); // reserved1+2 + more
//! let r = parse(&d).unwrap();
//! assert_eq!(r.kind, Kind::Rar4);
//! assert_eq!(r.blocks[0].kind, TYPE_MAIN);
//! ```

/// RAR1.5-4 signature.
pub const MAGIC4: [u8; 7] = *b"Rar!\x1a\x07\x00";
/// RAR5 signature.
pub const MAGIC5: [u8; 8] = *b"Rar!\x1a\x07\x01\x00";
/// Block flag: `ADD_SIZE` u32 follows `HEAD_SIZE`.
pub const LONG_BLOCK: u16 = 0x8000;
/// Main archive header.
pub const TYPE_MAIN: u8 = 0x72;
/// File member header.
pub const TYPE_FILE: u8 = 0x73;
/// Comment.
pub const TYPE_COMMENT: u8 = 0x74;
/// Service headers (ACL/stream/...) in RAR3+.
pub const TYPE_SERVICE: u8 = 0x7a;
/// End-of-archive.
pub const TYPE_END: u8 = 0x7b;

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// Archive generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// RAR 1.5-4.x block chain.
    Rar4,
    /// RAR5 (varint headers).
    Rar5,
}

/// One RAR4 block header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    /// `HEAD_TYPE`.
    pub kind: u8,
    /// `HEAD_FLAGS`.
    pub flags: u16,
    /// `HEAD_SIZE` (header bytes only).
    pub head_size: u16,
    /// `ADD_SIZE` — data bytes following the header, when the
    /// `LONG_BLOCK` flag is set.
    pub add_size: u32,
    /// File offset of the header.
    pub at: usize,
}

impl Block {
    /// File offset just past the block (header + data).
    pub fn end(&self) -> usize {
        self.at + usize::from(self.head_size) + self.add_size as usize
    }
    /// Data region inside the file, when present.
    pub fn data<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        if self.flags & LONG_BLOCK == 0 || self.add_size == 0 {
            return None;
        }
        d.get(self.at + usize::from(self.head_size)..self.end())
    }
}

/// A parsed RAR signature + blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rar {
    /// Signature generation.
    pub kind: Kind,
    /// RAR4 blocks in file order (empty for RAR5).
    pub blocks: Vec<Block>,
}

/// Parse a RAR signature and, for RAR4, the block chain. Walking
/// stops at `TYPE_END`, a malformed header, or EOF; a file whose
/// first block is truncated returns `None`.
pub fn parse(d: &[u8]) -> Option<Rar> {
    if d.get(..8)? == MAGIC5.as_slice() {
        return Some(Rar {
            kind: Kind::Rar5,
            blocks: Vec::new(),
        });
    }
    if d.get(..7)? != MAGIC4.as_slice() {
        return None;
    }
    let mut blocks = Vec::new();
    let mut at = 7usize;
    while let Some(head) = d.get(at..at + 7) {
        let kind = head[2];
        let flags = le16(d, at + 3)?;
        let head_size = le16(d, at + 5)?;
        if head_size < 7 {
            return None;
        }
        let add_size = if flags & LONG_BLOCK != 0 {
            le32(d, at + 7)?
        } else {
            0
        };
        let b = Block {
            kind,
            flags,
            head_size,
            add_size,
            at,
        };
        let end = b.end();
        if end > d.len() {
            return None;
        }
        blocks.push(b);
        if kind == TYPE_END {
            break;
        }
        at = end;
        if at == d.len() {
            break;
        }
    }
    if blocks.is_empty() {
        return None;
    }
    Some(Rar {
        kind: Kind::Rar4,
        blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(d: &mut Vec<u8>, kind: u8, flags: u16, extra: &[u8], data: &[u8]) {
        let head_size = (7 + extra.len() + if flags & LONG_BLOCK != 0 { 4 } else { 0 }) as u16;
        d.extend_from_slice(&[0, 0, kind, flags as u8, (flags >> 8) as u8]);
        d.extend_from_slice(&[head_size as u8, (head_size >> 8) as u8]);
        if flags & LONG_BLOCK != 0 {
            let l = data.len() as u32;
            d.extend_from_slice(&[l as u8, (l >> 8) as u8, (l >> 16) as u8, (l >> 24) as u8]);
        }
        d.extend_from_slice(extra);
        d.extend_from_slice(data);
    }

    #[test]
    fn rar4_chain() {
        let mut d = MAGIC4.to_vec();
        block(&mut d, TYPE_MAIN, 0, &[0; 6], &[]);
        block(&mut d, TYPE_FILE, LONG_BLOCK, &[0x30, 0, 0, 0], b"FILEDATA");
        block(&mut d, TYPE_END, 0, &[], &[]);
        let r = parse(&d).unwrap();
        assert_eq!(r.kind, Kind::Rar4);
        assert_eq!(r.blocks.len(), 3);
        let f = r.blocks[1];
        assert_eq!(f.kind, TYPE_FILE);
        assert_eq!(f.flags & LONG_BLOCK, LONG_BLOCK);
        assert_eq!(f.data(&d), Some(&b"FILEDATA"[..]));
        assert_eq!(r.blocks[2].kind, TYPE_END);
    }

    #[test]
    fn rar5_and_rejects() {
        assert_eq!(parse(&MAGIC5).unwrap().kind, Kind::Rar5);
        assert!(parse(b"Rar!\x1a\x07").is_none()); // truncated magic
        assert!(parse(b"Rar!\x1a\x07\x02junk").is_none()); // unknown version byte
                                                           // bad head_size
        let mut d = MAGIC4.to_vec();
        d.extend_from_slice(&[0, 0, TYPE_MAIN, 0, 0, 3, 0]);
        assert!(parse(&d).is_none());
    }
}

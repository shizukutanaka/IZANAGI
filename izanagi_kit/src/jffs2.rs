//! JFFS2 (Journalling Flash File System v2) node-header scanning.
//!
//! Every node starts with `{magic u16, nodetype u16, totlen u32,
//! hdr_crc u32}` (little-endian on typical NOR/NAND images; magic bytes
//! `85 19`). `nodetype`: DIRENT `0xE001`, INODE `0xE002`,
//! CLEANMARKER `0x2003`, PADDING `0x2004`, SUMMARY `0x2006`,
//! XATTR `0xE004`, XREF `0xE005`. `hdr_crc` = CRC32 of the first 8
//! bytes. Padding/cleanmarker nodes may have `totlen` < 12 semantics —
//! they still carry the 12-byte header.
//!
//! ```
//! use izanagi_kit::{crc, jffs2};
//! let mut d = vec![0u8; 24];
//! d[0..2].copy_from_slice(&0x1985u16.to_le_bytes());
//! d[2..4].copy_from_slice(&0xE002u16.to_le_bytes()); // INODE
//! d[4..8].copy_from_slice(&24u32.to_le_bytes()); // totlen
//! let c = crc::crc32(&d[..8]);
//! d[8..12].copy_from_slice(&c.to_le_bytes());
//! let j = jffs2::parse(&d).unwrap();
//! assert_eq!(j.nodes[0].kind, jffs2::Kind::Inode);
//! ```

use crate::crc;
use std::vec::Vec;

/// Node magic (LE bytes `85 19`).
pub const MAGIC: u16 = 0x1985;

/// JFFS2 node type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `0xE001` — directory entry.
    Dirent,
    /// `0xE002` — inode + data.
    Inode,
    /// `0x2003` — clean marker.
    CleanMarker,
    /// `0x2004` — padding.
    Padding,
    /// `0x2006` — erase-block summary.
    Summary,
    /// `0xE004` — extended attribute.
    Xattr,
    /// `0xE005` — xattr reference.
    Xref,
    /// Unrecognised node type.
    Other(u16),
}

impl Kind {
    fn from_u16(v: u16) -> Kind {
        match v {
            0xE001 => Kind::Dirent,
            0xE002 => Kind::Inode,
            0x2003 => Kind::CleanMarker,
            0x2004 => Kind::Padding,
            0x2006 => Kind::Summary,
            0xE004 => Kind::Xattr,
            0xE005 => Kind::Xref,
            o => Kind::Other(o),
        }
    }
}

/// One JFFS2 node header.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Node kind.
    pub kind: Kind,
    /// `totlen` — full node length including header.
    pub total_len: u32,
    /// Byte offset of this node's header.
    pub offset: usize,
    /// Byte offset where the node body starts (`offset` + 12).
    pub data_offset: usize,
}

/// A scanned JFFS2 image.
#[derive(Clone, Debug, PartialEq)]
pub struct Jffs2 {
    /// Node headers in file order.
    pub nodes: Vec<Node>,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Walks the node chain: every node must carry a valid magic and a
/// matching `hdr_crc`, and `totlen` must tile the input. Trailing
/// `0xFF` erased-flash bytes are ignored.
pub fn parse(d: &[u8]) -> Option<Jffs2> {
    let mut nodes = Vec::new();
    let mut at = 0usize;
    loop {
        // Skip erased-flash tail.
        while at < d.len() && d[at] == 0xFF {
            at += 1;
        }
        if at == d.len() {
            break;
        }
        if at.checked_add(12)? > d.len() {
            return None;
        }
        if u16le(d, at)? != MAGIC {
            return None;
        }
        let ty = u16le(d, at + 2)?;
        let totlen = u32le(d, at + 4)? as usize;
        let hcrc = u32le(d, at + 8)?;
        if crc::crc32(d.get(at..at + 8)?) != hcrc {
            return None;
        }
        if totlen < 12 || at.checked_add(totlen)? > d.len() {
            return None;
        }
        nodes.push(Node {
            kind: Kind::from_u16(ty),
            total_len: totlen as u32,
            offset: at,
            data_offset: at + 12,
        });
        at += totlen;
    }
    if nodes.is_empty() {
        return None;
    }
    Some(Jffs2 { nodes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(ty: u16, body: &[u8]) -> Vec<u8> {
        let totlen = (12 + body.len()) as u32;
        let mut d = Vec::new();
        d.extend_from_slice(&MAGIC.to_le_bytes());
        d.extend_from_slice(&ty.to_le_bytes());
        d.extend_from_slice(&totlen.to_le_bytes());
        let c = crc::crc32(&d);
        d.extend_from_slice(&c.to_le_bytes());
        d.extend_from_slice(body);
        d
    }

    #[test]
    fn walks_nodes_and_erased_tail() {
        let mut d = node(0xE001, &[0u8; 20]);
        d.extend_from_slice(&node(0xE002, &[0u8; 8]));
        d.extend_from_slice(&node(0x2004, &[]));
        d.extend_from_slice(&[0xFF; 16]); // erased flash tail
        let j = parse(&d).unwrap();
        assert_eq!(j.nodes.len(), 3);
        assert_eq!(j.nodes[0].kind, Kind::Dirent);
        assert_eq!(j.nodes[1].kind, Kind::Inode);
        assert_eq!(j.nodes[2].kind, Kind::Padding);
        assert_eq!(j.nodes[2].offset, 32 + 20);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0xFF; 64]).is_none()); // all erased: no nodes
        let mut d = node(0xE002, &[]);
        d[8] ^= 1; // bad hdr crc
        assert!(parse(&d).is_none());
        let mut d = node(0xE002, &[]);
        d[0] = 0x00; // bad magic
        assert!(parse(&d).is_none());
    }
}

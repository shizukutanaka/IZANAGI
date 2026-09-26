//! pcapng — the Next-Generation capture format (draft-tuexen):
//! a chain of `type + total_len + body + total_len` blocks.
//! Section Header (`0x0A0D0D0A`) selects byte order via its BOM;
//! Interface Description (1) and Enhanced Packet (6) bodies get
//! typed views. `pcap` covers the classic file format.
//!
//! ```
//! use izanagi_kit::pcapng;
//! let mut p = Vec::new();
//! p.extend_from_slice(&[0x0A, 0x0D, 0x0D, 0x0A]); // SHB
//! p.extend_from_slice(&[28, 0, 0, 0]);            // total len
//! p.extend_from_slice(&[0x4D, 0x3C, 0x2B, 0x1A]); // BOM (LE file)
//! p.extend_from_slice(&[0, 1, 0, 0]);             // version 1.0
//! p.extend_from_slice(&[0; 8]);                   // section length
//! p.extend_from_slice(&[28, 0, 0, 0]);
//! let g = pcapng::parse(&p).unwrap();
//! assert_eq!(g.sections[0].blocks.len(), 1);
//! assert!(g.le);
//! ```

use std::vec::Vec;

/// Block type constants.
pub const SHB: u32 = 0x0A0D_0D0A;
/// Interface Description Block.
pub const IDB: u32 = 1;
/// Simple Packet Block.
pub const SPB: u32 = 3;
/// Enhanced Packet Block.
pub const EPB: u32 = 6;

fn rl16(d: &[u8], at: usize) -> Option<u32> {
    Some(*d.get(at)? as u32 | (*d.get(at + 1)? as u32) << 8)
}
fn rl32(d: &[u8], at: usize) -> Option<u32> {
    Some(rl16(d, at)? | rl16(d, at + 2)? << 16)
}
fn rb32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (*d.get(at)? as u32) << 24
            | (*d.get(at + 1)? as u32) << 16
            | (*d.get(at + 2)? as u32) << 8
            | *d.get(at + 3)? as u32,
    )
}

/// One block: type + byte range of the body.
#[derive(Clone, Debug)]
pub struct Block {
    /// Block type code.
    pub ty: u32,
    /// Body offset inside the file.
    pub offset: usize,
    /// Body length (excludes the two length words).
    pub size: usize,
}

/// A section opened by an SHB.
#[derive(Clone, Debug)]
pub struct Section {
    /// Blocks until the next SHB (exclusive).
    pub blocks: Vec<Block>,
}

/// A parsed pcapng file.
#[derive(Clone, Debug)]
pub struct PcapNg {
    /// True when the section is little-endian (`1A2B3C4D` BOM as
    /// stored little-endian). Mixed-endian files beyond the first
    /// section are rejected.
    pub le: bool,
    /// Sections (one per SHB).
    pub sections: Vec<Section>,
    /// Snaplen of each IDB, in file order.
    pub snaplens: Vec<u32>,
    /// Linktype of each IDB.
    pub linktypes: Vec<u16>,
}

/// Parses the block chain. Every block's two length words must
/// agree; block bodies are padded to 32 bits. `None` on bad SHB,
/// length mismatch, or truncation.
pub fn parse(d: &[u8]) -> Option<PcapNg> {
    if d.is_empty() {
        return Some(PcapNg {
            le: true,
            sections: Vec::new(),
            snaplens: Vec::new(),
            linktypes: Vec::new(),
        });
    }
    // First SHB fixes the file's byte order via the BOM field.
    if rb32(d, 0)? != SHB {
        return None;
    }
    let le = match d.get(8..12)? {
        [0x4D, 0x3C, 0x2B, 0x1A] => true,
        [0x1A, 0x2B, 0x3C, 0x4D] => false,
        _ => return None,
    };
    let r32 = if le { rl32 } else { rb32 };
    let mut sections = Vec::new();
    let mut snaplens = Vec::new();
    let mut linktypes = Vec::new();
    let mut i = 0usize;
    let mut cur: Option<Section> = None;
    while i < d.len() {
        // SHB reads the same in both orders; others use section order
        let ty = if rb32(d, i)? == SHB { SHB } else { r32(d, i)? };
        let len = r32(d, i + 4)? as usize;
        if len < 12 {
            return None;
        }
        let end = i.checked_add(len)?;
        if end > d.len() {
            return None;
        }
        if r32(d, end - 4)? as usize != len {
            return None; // trailing length must match
        }
        let body_at = i + 8;
        let body_len = len - 12;
        if ty == SHB {
            if let Some(s) = cur.take() {
                sections.push(s);
            }
            cur = Some(Section { blocks: Vec::new() });
        }
        let blk = Block {
            ty,
            offset: body_at,
            size: body_len,
        };
        if ty == IDB {
            linktypes.push(if le {
                rl16(d, body_at)? as u16
            } else {
                (rb32(d, body_at)? >> 16) as u16
            });
            snaplens.push(r32(d, body_at + 4)?);
        }
        if let Some(s) = cur.as_mut() {
            s.blocks.push(blk);
        }
        i = end;
    }
    if let Some(s) = cur.take() {
        sections.push(s);
    }
    Some(PcapNg {
        le,
        sections,
        snaplens,
        linktypes,
    })
}

/// Body bytes of `b`.
pub fn body<'a>(d: &'a [u8], b: &Block) -> Option<&'a [u8]> {
    d.get(b.offset..b.offset.checked_add(b.size)?)
}

/// EPB payload: interface id, timestamp (32.32 hi/lo as written),
/// captured/original lengths, and the packet bytes.
#[derive(Clone, Debug)]
pub struct Packet {
    /// Interface the packet arrived on (IDB index).
    pub iface: u32,
    /// Timestamp high word.
    pub ts_hi: u32,
    /// Timestamp low word.
    pub ts_lo: u32,
    /// Captured length (may be < orig).
    pub captured: u32,
    /// Original wire length.
    pub original: u32,
    /// Packet bytes (`captured` long, unpadded).
    pub data: Vec<u8>,
}

/// Decodes every EPB across all sections.
pub fn packets(d: &[u8], g: &PcapNg) -> Vec<Packet> {
    let r32 = if g.le { rl32 } else { rb32 };
    let mut out = Vec::new();
    for s in &g.sections {
        for b in &s.blocks {
            if b.ty != EPB {
                continue;
            }
            let Some(bb) = body(d, b) else {
                continue;
            };
            if bb.len() < 20 {
                continue;
            }
            let iface = r32(bb, 0).unwrap_or(0);
            let hi = r32(bb, 4).unwrap_or(0);
            let lo = r32(bb, 8).unwrap_or(0);
            let cap = r32(bb, 12).unwrap_or(0) as usize;
            let orig = r32(bb, 16).unwrap_or(0);
            if 20 + cap > bb.len() {
                continue;
            }
            out.push(Packet {
                iface,
                ts_hi: hi,
                ts_lo: lo,
                captured: cap as u32,
                original: orig,
                data: bb[20..20 + cap].to_vec(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(ty: [u8; 4], body: &[u8]) -> Vec<u8> {
        let pad = (4 - body.len() % 4) % 4;
        let len = (12 + body.len() + pad) as u32;
        let mut b = Vec::new();
        b.extend_from_slice(&ty);
        b.extend_from_slice(&[
            len as u8,
            (len >> 8) as u8,
            (len >> 16) as u8,
            (len >> 24) as u8,
        ]);
        b.extend_from_slice(body);
        b.resize(b.len() + pad, 0);
        b.extend_from_slice(&[
            len as u8,
            (len >> 8) as u8,
            (len >> 16) as u8,
            (len >> 24) as u8,
        ]);
        b
    }

    fn shb() -> Vec<u8> {
        // SHB type is byte-order symmetric; rest of file is LE here
        block(
            [0x0A, 0x0D, 0x0D, 0x0A],
            &[0x4D, 0x3C, 0x2B, 0x1A, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )
    }

    #[test]
    fn parses_shb_idb_epb() {
        let mut p = shb();
        p.extend_from_slice(&block([1, 0, 0, 0], &[1, 0, 0, 0, 255, 255, 0, 0]));
        let mut epb_body = vec![0; 20];
        epb_body[12] = 4; // captured = 4 (LE)
        epb_body[16] = 4;
        epb_body.extend_from_slice(b"data");
        p.extend_from_slice(&block([6, 0, 0, 0], &epb_body));
        let g = parse(&p).unwrap();
        assert!(g.le);
        assert_eq!(g.linktypes, vec![1]);
        let pk = packets(&p, &g);
        assert_eq!(pk.len(), 1);
        assert_eq!(pk[0].data, b"data".to_vec());
        assert_eq!(pk[0].captured, 4);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_some()); // empty file = empty capture
        assert!(parse(&[0; 8]).is_none()); // not SHB
        let mut p = shb();
        p.truncate(p.len() - 4); // lose trailing length
        assert!(parse(&p).is_none());
        let mut bad = shb();
        bad[8] = 0xFF; // corrupt BOM
        assert!(parse(&bad).is_none());
        let mut bad2 = shb();
        let n = bad2.len();
        bad2[n - 4] ^= 0xFF; // trailing length mismatch
        assert!(parse(&bad2).is_none());
    }

    #[test]
    fn two_sections() {
        let mut p = shb();
        p.extend_from_slice(&block([1, 0, 0, 0], &[1, 0, 0, 0, 0, 0, 0, 0]));
        p.extend_from_slice(&shb());
        let g = parse(&p).unwrap();
        assert_eq!(g.sections.len(), 2);
    }
}

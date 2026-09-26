//! pcapng capture files (draft-ietf-opsawg-pcapng, obsoleting the
//! original pcapntw draft) — the block-structured successor to the
//! classic `.pcap` in [`crate::pcap`]. Every block is
//! `type:u32 len:u32 body… len:u32` (total length repeated at the tail,
//! body 32-bit padded). The Section Header Block `0x0A0D0D0A` carries a
//! byte-order magic `0x1A2B3C4D` that fixes endianness for the rest of
//! the section. [`parse`] walks blocks and collects interface
//! descriptions (IDB), enhanced packets (EPB), and simple packets (SPB);
//! [`options`] decodes the `code:u16 len:u16 value` trailer area, and
//! [`Epb::ts_ns`] applies the `if_tsresol` interface option.
//!
//! ```
//! use izanagi_kit::pcapng::parse;
//! let mut b = Vec::new();
//! // SHB: type, len, byte-order magic, v1.0, section len -1, len
//! b.extend_from_slice(&[0x0A, 0x0D, 0x0D, 0x0A]);
//! b.extend_from_slice(&28u32.to_le_bytes());
//! b.extend_from_slice(&[0x4D, 0x3C, 0x2B, 0x1A]);
//! b.extend_from_slice(&1u16.to_le_bytes());
//! b.extend_from_slice(&0u16.to_le_bytes());
//! b.extend_from_slice(&(-1i64).to_le_bytes());
//! b.extend_from_slice(&28u32.to_le_bytes());
//! let p = parse(&b).unwrap();
//! assert!(p.le);
//! ```

use std::vec::Vec;

/// Section Header Block type.
pub const SHB: u32 = 0x0A0D_0D0A;
/// Interface Description Block type.
pub const IDB: u32 = 0x0000_0001;
/// Packet Block (obsolete) type.
pub const PB: u32 = 0x0000_0002;
/// Simple Packet Block type.
pub const SPB: u32 = 0x0000_0003;
/// Name Resolution Block type.
pub const NRB: u32 = 0x0000_0004;
/// Interface Statistics Block type.
pub const ISB: u32 = 0x0000_0005;
/// Enhanced Packet Block type.
pub const EPB: u32 = 0x0000_0006;
/// Decryption Secrets Block type.
pub const DSB: u32 = 0x0000_000A;
/// Custom Block types (both ranges carry a PEN in the body).
pub const CUSTOM: [u32; 2] = [0x0000_0BAD, 0x0BAD_0BAD];

/// A raw block. `body` spans everything between the two length fields
/// (options included); the section's endianness governs its fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// Block type code.
    pub kind: u32,
    /// Offset of the type field in the input.
    pub at: usize,
    /// Total block length including headers (always 32-bit aligned).
    pub total: u32,
    /// Offset of the body (after the `len` field).
    pub body_at: usize,
}

/// Registry name for a block type (`"SHB"`, `"IDB"`, `"EPB"`…),
/// `"CUSTOM"` for the custom range, `"?"` for unassigned codes.
pub fn kind_name(kind: u32) -> &'static str {
    match kind {
        SHB => "SHB",
        IDB => "IDB",
        PB => "PB",
        SPB => "SPB",
        NRB => "NRB",
        ISB => "ISB",
        EPB => "EPB",
        DSB => "DSB",
        0x0000_0BAD | 0x0BAD_0BAD => "CUSTOM",
        _ => "?",
    }
}

fn u32e(le: bool, d: &[u8], i: usize) -> Option<u32> {
    let b = d.get(i..i + 4)?;
    Some(if le {
        u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16 | u32::from(b[3]) << 24
    } else {
        u32::from(b[0]) << 24 | u32::from(b[1]) << 16 | u32::from(b[2]) << 8 | u32::from(b[3])
    })
}

fn u16e(le: bool, d: &[u8], i: usize) -> Option<u16> {
    let b = d.get(i..i + 2)?;
    Some(if le {
        u16::from(b[0]) | u16::from(b[1]) << 8
    } else {
        u16::from(b[0]) << 8 | u16::from(b[1])
    })
}

/// One interface description (from an IDB).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iface {
    /// Link-layer type (`LINKTYPE_ETHERNET` = 1, `LINKTYPE_RAW` = 101…).
    pub link: u16,
    /// Snapshot length limit.
    pub snaplen: u32,
    /// `if_tsresol` option: timestamp ticks are `10^-n` seconds by
    /// default (n=6), or `2^-n` when the option's MSB is set.
    pub tsresol: u8,
    /// True when `tsresol`'s MSB was set (power-of-two units).
    pub tsresol_binary: bool,
}

/// One enhanced packet (from an EPB); data stays in the input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Epb {
    /// Interface index this packet was captured on.
    pub iface: u32,
    /// `ts_hi << 32 | ts_lo` raw timestamp ticks.
    pub ts: u64,
    /// Captured length.
    pub cap: u32,
    /// On-wire length.
    pub orig: u32,
    /// Offset of the captured bytes in the input.
    pub data_at: usize,
}

impl Epb {
    /// Timestamp in nanoseconds, applying the interface's `if_tsresol`.
    /// Saturates at `u64::MAX` rather than overflowing.
    pub fn ts_ns(&self, ifaces: &[Iface]) -> Option<u64> {
        let f = ifaces.get(self.iface as usize)?;
        let (unit, per_sec) = if f.tsresol_binary {
            (2u128, f.tsresol as u32)
        } else {
            (10u128, f.tsresol as u32)
        };
        let denom = unit.checked_pow(per_sec)?;
        let ns = (self.ts as u128)
            .checked_mul(1_000_000_000)?
            .checked_div(denom)?;
        u64::try_from(ns).ok()
    }
}

/// A parsed pcapng stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Pcapng {
    /// True when the current section is little-endian (the common case).
    pub le: bool,
    /// All blocks in file order.
    pub blocks: Vec<Block>,
    /// Interface descriptions in IDB order (EPB `iface` indexes this).
    pub ifaces: Vec<Iface>,
    /// Enhanced packets in file order.
    pub packets: Vec<Epb>,
    /// Simple-packet `(orig_len, data_at)` records.
    pub simple: Vec<(u32, usize)>,
}

/// `(code, value)` option decode for a block's option area — the area
/// ends at `opt_endofopt` (0) or the block end.
pub fn options(le: bool, d: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut v = Vec::new();
    let mut i = 0usize;
    while i + 4 <= d.len() {
        let (Some(code), Some(len)) = (u16e(le, d, i), u16e(le, d, i + 2)) else {
            break;
        };
        i += 4;
        if code == 0 {
            break; // opt_endofopt
        }
        let n = len as usize;
        let val = d.get(i..i + n).map(|s| s.to_vec());
        let Some(val) = val else { break };
        v.push((code, val));
        i += (n + 3) & !3; // value padded to 32 bits
    }
    v
}

/// Parse a pcapng file. `None` on a bad SHB magic, a total length that
/// isn't self-consistent or 32-bit aligned, or a truncated body. A
/// second SHB mid-file starts a new (possibly endianness-flipped)
/// section — handled, since captures get concatenated.
pub fn parse(d: &[u8]) -> Option<Pcapng> {
    let mut le = true;
    let mut first = true;
    let mut i = 0usize;
    let mut blocks = Vec::new();
    let mut ifaces = Vec::new();
    let mut packets = Vec::new();
    let mut simple = Vec::new();
    while i < d.len() {
        // SHB's type code 0x0A0D0D0A is a byte palindrome: readable in
        // either order. For an SHB the byte-order magic inside the body
        // is consulted *before* the length fields, since a new section
        // may flip endianness. Other block types are read in the
        // current section's endianness (`le` starts LE for the first
        // block, which must be an SHB anyway).
        let raw_kind = u32e(le, d, i)?;
        if raw_kind == SHB {
            match d.get(i + 8..i + 12)? {
                [0x4D, 0x3C, 0x2B, 0x1A] => le = true,
                [0x1A, 0x2B, 0x3C, 0x4D] => le = false,
                _ => return None,
            }
            first = false;
        } else if first {
            return None; // file must start with SHB
        }
        let total = u32e(le, d, i + 4)? as usize;
        if total < 12 || total % 4 != 0 {
            return None;
        }
        let end = i.checked_add(total)?;
        if end > d.len() {
            return None;
        }
        if u32e(le, d, end - 4)? as usize != total {
            return None; // trailing length mismatch
        }
        let body_at = i + 8;
        blocks.push(Block {
            kind: raw_kind,
            at: i,
            total: total as u32,
            body_at,
        });
        let body = d.get(body_at..end - 4)?;
        match raw_kind {
            IDB => {
                let link = u16e(le, body, 0)?;
                let snaplen = u32e(le, body, 4)?;
                let mut tsresol = 6;
                let mut tsresol_binary = false;
                for (code, val) in options(le, &body[8.min(body.len())..]) {
                    if code == 9 && !val.is_empty() {
                        let t = val[0];
                        tsresol_binary = t & 0x80 != 0;
                        tsresol = t & 0x7F;
                    }
                }
                ifaces.push(Iface {
                    link,
                    snaplen,
                    tsresol,
                    tsresol_binary,
                });
            }
            EPB => {
                let iface = u32e(le, body, 0)?;
                let hi = u32e(le, body, 4)? as u64;
                let lo = u32e(le, body, 8)? as u64;
                let cap = u32e(le, body, 12)?;
                let orig = u32e(le, body, 16)?;
                let cap_padded = cap.checked_add(3)? & !3;
                if (20 + cap_padded) as usize > body.len() {
                    return None;
                }
                packets.push(Epb {
                    iface,
                    ts: (hi << 32) | lo,
                    cap,
                    orig,
                    data_at: body_at + 20,
                });
            }
            SPB => {
                let orig = u32e(le, body, 0)?;
                simple.push((orig, body_at + 4));
            }
            _ => {}
        }
        i = end;
    }
    if first {
        return None;
    }
    Some(Pcapng {
        le,
        blocks,
        ifaces,
        packets,
        simple,
    })
}

/// Captured bytes of an EPB.
pub fn packet<'a>(d: &'a [u8], e: &Epb) -> Option<&'a [u8]> {
    d.get(e.data_at..e.data_at + e.cap as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blk(kind: u32, body: &[u8], le: bool) -> Vec<u8> {
        let mut pad = body.to_vec();
        while pad.len() % 4 != 0 {
            pad.push(0);
        }
        let total = (pad.len() + 12) as u32;
        let mut d = Vec::new();
        d.extend_from_slice(&kind.to_le_bytes());
        d.extend_from_slice(&if le {
            total.to_le_bytes()
        } else {
            total.to_be_bytes()
        });
        d.extend_from_slice(&pad);
        d.extend_from_slice(&if le {
            total.to_le_bytes()
        } else {
            total.to_be_bytes()
        });
        d
    }

    fn shb(le: bool) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&if le {
            [0x4D, 0x3C, 0x2B, 0x1A]
        } else {
            [0x1A, 0x2B, 0x3C, 0x4D]
        });
        body.extend_from_slice(&if le {
            1u16.to_le_bytes()
        } else {
            1u16.to_be_bytes()
        });
        body.extend_from_slice(&if le {
            0u16.to_le_bytes()
        } else {
            0u16.to_be_bytes()
        });
        body.extend_from_slice(&if le {
            (-1i64).to_le_bytes()
        } else {
            (-1i64).to_be_bytes()
        });
        blk(SHB, &body, le)
    }

    #[test]
    fn section_and_idb() {
        let mut d = shb(true);
        let mut idb_body = Vec::new();
        idb_body.extend_from_slice(&1u16.to_le_bytes()); // Ethernet
        idb_body.extend_from_slice(&[0; 2]);
        idb_body.extend_from_slice(&65535u32.to_le_bytes()); // snaplen
                                                             // if_tsresol = 9 → ns units
        idb_body.extend_from_slice(&9u16.to_le_bytes());
        idb_body.extend_from_slice(&1u16.to_le_bytes());
        idb_body.extend_from_slice(&[9, 0, 0, 0]);
        d.extend_from_slice(&blk(IDB, &idb_body, true));
        let p = parse(&d).unwrap();
        assert!(p.le);
        assert_eq!(p.ifaces.len(), 1);
        assert_eq!(p.ifaces[0].link, 1);
        assert_eq!(p.ifaces[0].tsresol, 9);
        assert_eq!(kind_name(p.blocks[1].kind), "IDB");
    }

    #[test]
    fn epb_and_tsresol() {
        let mut d = shb(true);
        let mut idb_body = Vec::new();
        idb_body.extend_from_slice(&1u16.to_le_bytes());
        idb_body.extend_from_slice(&[0; 2]);
        idb_body.extend_from_slice(&65535u32.to_le_bytes());
        d.extend_from_slice(&blk(IDB, &idb_body, true));
        let mut epb_body = Vec::new();
        epb_body.extend_from_slice(&0u32.to_le_bytes()); // iface 0
        epb_body.extend_from_slice(&0u32.to_le_bytes()); // ts hi
        epb_body.extend_from_slice(&1_500_000u32.to_le_bytes()); // ts lo = 1.5s @us
        epb_body.extend_from_slice(&3u32.to_le_bytes()); // caplen
        epb_body.extend_from_slice(&4u32.to_le_bytes()); // origlen
        epb_body.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0]); // data + pad
        d.extend_from_slice(&blk(EPB, &epb_body, true));
        let p = parse(&d).unwrap();
        assert_eq!(p.packets.len(), 1);
        let e = &p.packets[0];
        assert_eq!(e.ts_ns(&p.ifaces), Some(1_500_000_000)); // default µs
        assert_eq!(packet(&d, e).unwrap(), &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn big_endian_section() {
        let p = parse(&shb(false)).unwrap();
        assert!(!p.le);
    }

    #[test]
    fn option_walk() {
        // opt_custom(2988) "ab" padded to 32 bits, then opt_endofopt
        let mut o = Vec::new();
        o.extend_from_slice(&2988u16.to_le_bytes());
        o.extend_from_slice(&2u16.to_le_bytes());
        o.extend_from_slice(b"ab\0\0");
        o.extend_from_slice(&0u16.to_le_bytes());
        o.extend_from_slice(&0u16.to_le_bytes());
        let opts = options(true, &o);
        assert_eq!(opts.len(), 1);
        assert_eq!(opts[0].0, 2988);
        assert_eq!(opts[0].1, b"ab");
        assert!(options(true, &o[..4]).is_empty()); // no terminator, no opts
    }

    #[test]
    fn bad_inputs() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0; 40]).is_none()); // bad SHB magic
        let mut bad = shb(true);
        bad.truncate(bad.len() - 1); // trailing length mismatch
        assert!(parse(&bad).is_none());
        let mut bad2 = blk(IDB, &[1, 0, 0, 0, 255, 255, 0, 0], true);
        bad2.extend_from_slice(&shb(true)); // SHB not first
        assert!(parse(&bad2).is_none());
    }
}

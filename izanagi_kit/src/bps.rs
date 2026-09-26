//! BPS — byuu's BPS patch format (UPS's successor; companion to
//! `crate::ups`/`crate::ips`). A file opens with `BPS1`, then three
//! varints — source size, target size, metadata length — the metadata
//! bytes, then an action stream until the 12-byte trailer. Each
//! action is a varint `v`: `action = v & 3` (0 `SourceRead`, 1
//! `TargetRead` — `len` literal bytes follow — 2 `SourceCopy`,
//! 3 `TargetCopy` — a signed varint offset follows) and
//! `len = (v >> 2) + 1`. The trailer is `{source, target, patch}`
//! u32LE CRCs.
//!
//! ```
//! use izanagi_kit::bps::{parse, actions, Action};
//! let mut d = b"BPS1".to_vec();
//! d.extend_from_slice(&[0x8A, 0x94, 0x80]); // sizes 10/20 + empty meta
//! d.push(3 << 2 | 0x80);               // action 0 (SourceRead), len 4
//! d.extend_from_slice(&[0; 12]);
//! let b = parse(&d).unwrap();
//! let a = actions(&b, &d).next().unwrap();
//! assert_eq!(a, Action::SourceRead(4));
//! ```

/// File magic.
pub const MAGIC: &[u8; 4] = b"BPS1";
/// Trailing CRC block length.
pub const TRAILER: usize = 12;

/// A parsed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bps {
    /// Declared input size.
    pub source_size: u64,
    /// Declared output size.
    pub target_size: u64,
    /// The metadata payload (often XML).
    pub metadata: Vec<u8>,
    /// Byte offset of the action stream.
    pub actions_at: usize,
}

/// One decoded action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Copy `n` bytes from the source at the output position.
    SourceRead(u64),
    /// Insert `n` literal bytes (they follow in the stream).
    TargetRead(u64, Vec<u8>),
    /// Copy `n` source bytes at `offset`.
    SourceCopy(u64, i64),
    /// Copy `n` previously-written output bytes at `offset`.
    TargetCopy(u64, i64),
}

/// BPS varint (same LSB-first ladder as UPS).
pub fn varint(d: &[u8], at: usize) -> Option<(u64, usize)> {
    crate::ups::varint(d, at)
}

/// A signed (zigzag-ish) varint: bit 0 is the sign, rest the value.
fn svarint(d: &[u8], at: usize) -> Option<(i64, usize)> {
    let (v, next) = varint(d, at)?;
    let s = if v & 1 == 1 { -1 } else { 1 };
    Some((s * i64::try_from(v >> 1).ok()?, next))
}

/// Parse the header; `None` without `BPS1`.
pub fn parse(d: &[u8]) -> Option<Bps> {
    if d.get(..4)? != MAGIC {
        return None;
    }
    let (source_size, at) = varint(d, 4)?;
    let (target_size, at) = varint(d, at)?;
    let (mlen, at) = varint(d, at)?;
    let meta_end = at.checked_add(usize::try_from(mlen).ok()?)?;
    let metadata = d.get(at..meta_end)?.to_vec();
    d.get(..meta_end.checked_add(TRAILER)?)?;
    Some(Bps {
        source_size,
        target_size,
        metadata,
        actions_at: meta_end,
    })
}

/// The three trailing CRCs (source, target, patch), little-endian.
pub fn crcs(d: &[u8]) -> Option<[u32; 3]> {
    crate::ups::crcs(d)
}

/// Iterate actions until the trailer. A truncated literal/offset ends
/// iteration early.
pub fn actions<'d>(b: &Bps, d: &'d [u8]) -> impl Iterator<Item = Action> + 'd {
    let mut at = b.actions_at;
    let end = d.len().saturating_sub(TRAILER);
    core::iter::from_fn(move || {
        if at >= end {
            return None;
        }
        let (v, next) = varint(d, at)?;
        at = next;
        let len = (v >> 2).checked_add(1)?;
        match v & 3 {
            0 => Some(Action::SourceRead(len)),
            1 => {
                let bytes = d.get(at..at.checked_add(usize::try_from(len).ok()?)?)?;
                at += bytes.len();
                Some(Action::TargetRead(len, bytes.to_vec()))
            }
            2 => {
                let (o, next) = svarint(d, at)?;
                at = next;
                Some(Action::SourceCopy(len, o))
            }
            _ => {
                let (o, next) = svarint(d, at)?;
                at = next;
                Some(Action::TargetCopy(len, o))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"BPS1".to_vec();
        d.extend_from_slice(&[0xC0, 0x00, 0x80, 0x83]); // 64, 128, metalen 3
        d.extend_from_slice(b"<x>");
        // action: TargetRead(2) + 2 bytes → v=5
        d.push(0x85);
        d.extend_from_slice(&[0xAA, 0xBB]);
        // action: SourceCopy(1) at offset +3 → v=2, svarint 6
        d.push(0x82);
        d.push(0x86);
        d.extend_from_slice(&[0; 12]);
        d
    }

    #[test]
    fn parses_header() {
        let d = fixture();
        let b = parse(&d).unwrap();
        assert_eq!((b.source_size, b.target_size), (64, 128));
        assert_eq!(b.metadata, b"<x>");
        assert_eq!(b.actions_at, 11);
        assert_eq!(crcs(&d).unwrap(), [0; 3]);
    }

    #[test]
    fn action_walk() {
        let d = fixture();
        let b = parse(&d).unwrap();
        let as_: Vec<_> = actions(&b, &d).collect();
        assert_eq!(as_.len(), 2);
        assert_eq!(as_[0], Action::TargetRead(2, vec![0xAA, 0xBB]));
        assert_eq!(as_[1], Action::SourceCopy(1, 3));
    }

    #[test]
    fn svarint_signed() {
        assert_eq!(svarint(&[0x86], 0), Some((3, 1)));
        assert_eq!(svarint(&[0x87], 0), Some((-3, 1)));
        assert_eq!(svarint(&[0x80], 0), Some((0, 1)));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"BPS2").is_none());
        // metadata claims 3 bytes but trailer wouldn't fit
        assert!(parse(b"BPS1\x81\x82\x83abc").is_none());
    }
}

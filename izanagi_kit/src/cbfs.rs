//! coreboot CBFS file entries.
//!
//! A CBFS region is a chain of file entries, each starting with the
//! 8-byte `LARCHIVE` magic followed by big-endian length, type,
//! checksum and data-offset fields, then a NUL-terminated file
//! name. Entry headers are 64-byte aligned; `parse` walks the chain
//! until the magic no longer matches.
//!
//! ```
//! use izanagi_kit::cbfs::{parse, MAGIC, ENTRY};
//!
//! let mut d = vec![0u8; 256];
//! d[..8].copy_from_slice(MAGIC);
//! let put = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = (v >> 24) as u8; d[at + 1] = (v >> 16) as u8;
//!     d[at + 2] = (v >> 8) as u8; d[at + 3] = v as u8;
//! };
//! put(&mut d, 8, 16);    // payload length
//! put(&mut d, 12, 0x20); // type: raw
//! put(&mut d, 16, 0);    // checksum
//! put(&mut d, 20, 64);   // data offset within entry
//! d[24..33].copy_from_slice(b"bootblock");
//! let c = parse(&d).unwrap();
//! assert_eq!(c.entries.len(), 1);
//! assert_eq!(c.entries[0].name(), "bootblock");
//! ```

/// File-entry magic.
pub const MAGIC: &[u8; 8] = b"LARCHIVE";
/// Minimum entry header size before the name.
pub const ENTRY: usize = 24;
/// Entry alignment in bytes.
pub const ALIGN: usize = 64;

/// Known entry types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 0x10 — boot stage.
    Stage,
    /// 0x20 — raw binary.
    Raw,
    /// 0x30 — executable payload.
    Payload,
    /// 0x40 — CBFS option ROM.
    OptionRom,
    /// 0x50 — bootsplash.
    Bootsplash,
    /// 0x60 — empty/deleted slot.
    Deleted,
    /// Any other type number.
    Other(u32),
}

impl Kind {
    /// Classify a raw type word.
    pub fn of(v: u32) -> Kind {
        match v {
            0x10 => Kind::Stage,
            0x20 => Kind::Raw,
            0x30 => Kind::Payload,
            0x40 => Kind::OptionRom,
            0x50 => Kind::Bootsplash,
            0x60 => Kind::Deleted,
            o => Kind::Other(o),
        }
    }
}

/// One file entry; `name_bytes` borrows the region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CbfsEntry<'a> {
    /// File offset of the entry.
    pub at: usize,
    /// Payload length.
    pub len: u32,
    /// Classified entry type.
    pub kind: Kind,
    /// Header checksum.
    pub checksum: u32,
    /// Offset from the entry start to the payload.
    pub offset: u32,
    /// File name bytes (NUL included when present).
    pub name_bytes: &'a [u8],
}

impl CbfsEntry<'_> {
    /// File name without the trailing NUL.
    pub fn name(&self) -> &str {
        let end = self
            .name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name_bytes.len());
        core::str::from_utf8(&self.name_bytes[..end]).unwrap_or("")
    }
    /// File offset of the payload.
    pub fn data_at(&self) -> usize {
        self.at + usize::try_from(self.offset).unwrap_or(u32::MAX as usize)
    }
    /// Offset where the next 64-aligned entry starts.
    pub fn next_at(&self) -> usize {
        let end = self
            .at
            .saturating_add(usize::try_from(self.offset).unwrap_or(u32::MAX as usize))
            .saturating_add(usize::try_from(self.len).unwrap_or(u32::MAX as usize));
        end.div_ceil(ALIGN) * ALIGN
    }
}

/// A walked CBFS region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cbfs<'a> {
    /// Entries in file order.
    pub entries: Vec<CbfsEntry<'a>>,
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

/// Walk the entry chain from offset 0. Returns `None` only when
/// the region starts with something that is not `LARCHIVE`.
pub fn parse(d: &[u8]) -> Option<Cbfs<'_>> {
    if d.get(..8)? != MAGIC {
        return None;
    }
    let mut entries = Vec::new();
    let mut at = 0usize;
    loop {
        match d.get(at..at + 8) {
            Some(m) if m == MAGIC.as_slice() => {}
            _ => break,
        }
        let len = be32(d, at + 8)?;
        let kind = Kind::of(be32(d, at + 12)?);
        let checksum = be32(d, at + 16)?;
        let offset = be32(d, at + 20)?;
        let data = at.checked_add(usize::try_from(offset).ok()?)?;
        if offset < ENTRY as u32 || data > d.len() {
            return None;
        }
        let name_bytes = d.get(at + ENTRY..data)?;
        let e = CbfsEntry {
            at,
            len,
            kind,
            checksum,
            offset,
            name_bytes,
        };
        let next = e.next_at();
        entries.push(e);
        if next <= at || next >= d.len() {
            break;
        }
        at = next;
    }
    Some(Cbfs { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(d: &mut [u8], at: usize, v: u32) {
        d[at] = (v >> 24) as u8;
        d[at + 1] = (v >> 16) as u8;
        d[at + 2] = (v >> 8) as u8;
        d[at + 3] = v as u8;
    }

    fn entry(d: &mut [u8], at: usize, len: u32, kind: u32, name: &str) {
        d[at..at + 8].copy_from_slice(MAGIC);
        put(d, at + 8, len);
        put(d, at + 12, kind);
        put(d, at + 20, 64);
        d[at + 24..at + 24 + name.len()].copy_from_slice(name.as_bytes());
    }

    #[test]
    fn walks_two_entries() {
        let mut d = vec![0u8; 512];
        entry(&mut d, 0, 32, 0x20, "bootblock");
        entry(&mut d, 128, 16, 0x30, "fallback/payload"); // 0+64+32=96 -> align 128
        let c = parse(&d).unwrap();
        assert_eq!(c.entries.len(), 2);
        assert_eq!(c.entries[0].kind, Kind::Raw);
        assert_eq!(c.entries[0].name(), "bootblock");
        assert_eq!(c.entries[0].data_at(), 64);
        assert_eq!(c.entries[1].kind, Kind::Payload);
        assert_eq!(c.entries[1].name(), "fallback/payload");
        assert_eq!(c.entries[1].at, 128);
    }

    #[test]
    fn stops_at_padding() {
        let mut d = vec![0u8; 256];
        entry(&mut d, 0, 8, 0x60, "empty-slot");
        let c = parse(&d).unwrap();
        assert_eq!(c.entries.len(), 1);
        assert_eq!(c.entries[0].kind, Kind::Deleted);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        let mut d = vec![0u8; 128];
        entry(&mut d, 0, 0, 0x20, "x");
        put(&mut d, 20, 16); // offset below name region
        assert!(parse(&d).is_none());
    }
}

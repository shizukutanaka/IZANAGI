//! Classic Unix `a.out` header (exec(2) / a.out(5)).
//!
//! The 32-byte header is a machine-id+magic word followed by text,
//! data and BSS sizes, symbol count, entry address and the
//! text/data relocation sizes — all host-endian, here little-endian
//! x86/68k/PDP ordering. The low 16 bits carry the well-known magic
//! (`OMAGIC` 0407, `NMAGIC` 0410, `ZMAGIC` 0411, `QMAGIC` 0314);
//! NetBSD packs a 16-bit machine id into the high half.
//!
//! ```
//! use izanagi_kit::aout::{parse, Kind, HEADER};
//!
//! let mut d = vec![0u8; HEADER];
//! let put = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = v as u8; d[at + 1] = (v >> 8) as u8;
//!     d[at + 2] = (v >> 16) as u8; d[at + 3] = (v >> 24) as u8;
//! };
//! put(&mut d, 0, 0x107);     // OMAGIC
//! put(&mut d, 4, 0x400);     // text size
//! put(&mut d, 8, 0x100);     // data size
//! let a = parse(&d).unwrap();
//! assert_eq!(a.kind, Kind::Omagic);
//! assert_eq!(a.text, 0x400);
//! ```

/// Header size in bytes.
pub const HEADER: usize = 32;
/// Magic: non-relocatable, text not read-only (0407).
pub const OMAGIC: u16 = 0o407;
/// Magic: separated text/data, text read-only (0410).
pub const NMAGIC: u16 = 0o410;
/// Magic: demand-paged executable (0411).
pub const ZMAGIC: u16 = 0o411;
/// Magic: demand-paged, header not mapped (0314).
pub const QMAGIC: u16 = 0o314;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// The executable layout family selected by the magic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Impure text, no paging alignment.
    Omagic,
    /// Separate read-only text segment.
    Nmagic,
    /// Demand-paged, header mapped.
    Zmagic,
    /// Demand-paged, header not in the text segment.
    Qmagic,
    /// Any other magic value.
    Other(u16),
}

/// A parsed a.out header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Aout {
    /// Layout family.
    pub kind: Kind,
    /// Low 16 bits of the magic word.
    pub magic: u16,
    /// NetBSD machine id (high 16 bits of the magic word; 0 for
    /// traditional headers).
    pub mid: u16,
    /// Text segment size.
    pub text: u32,
    /// Data segment size.
    pub data: u32,
    /// BSS size.
    pub bss: u32,
    /// Symbol table size.
    pub syms: u32,
    /// Entry address.
    pub entry: u32,
    /// Text relocation size.
    pub trsize: u32,
    /// Data relocation size.
    pub drsize: u32,
}

impl Aout {
    /// File offset of the symbol table (header + text + data +
    /// relocations).
    pub fn syms_at(&self) -> u64 {
        u64::from(HEADER as u32)
            + u64::from(self.text)
            + u64::from(self.data)
            + u64::from(self.trsize)
            + u64::from(self.drsize)
    }
}

/// Parse a header. Returns `None` when fewer than 32 bytes are
/// present. The magic itself is reported via `kind` even when
/// unknown.
pub fn parse(d: &[u8]) -> Option<Aout> {
    if d.len() < HEADER {
        return None;
    }
    let magic_word = le32(d, 0)?;
    let magic = (magic_word & 0xffff) as u16;
    let mid = (magic_word >> 16) as u16;
    let kind = match magic {
        OMAGIC => Kind::Omagic,
        NMAGIC => Kind::Nmagic,
        ZMAGIC => Kind::Zmagic,
        QMAGIC => Kind::Qmagic,
        m => Kind::Other(m),
    };
    Some(Aout {
        kind,
        magic,
        mid,
        text: le32(d, 4)?,
        data: le32(d, 8)?,
        bss: le32(d, 12)?,
        syms: le32(d, 16)?,
        entry: le32(d, 20)?,
        trsize: le32(d, 24)?,
        drsize: le32(d, 28)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(magic: u32) -> Vec<u8> {
        let mut d = vec![0u8; 64];
        let w = |d: &mut [u8], o: usize, v: u32| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
            d[o + 2] = (v >> 16) as u8;
            d[o + 3] = (v >> 24) as u8;
        };
        w(&mut d, 0, magic);
        w(&mut d, 4, 0x1000);
        w(&mut d, 8, 0x400);
        w(&mut d, 12, 0x80);
        w(&mut d, 16, 0x60);
        w(&mut d, 20, 0x2000);
        w(&mut d, 24, 0x10);
        w(&mut d, 28, 0x20);
        d
    }

    #[test]
    fn omagic_fields() {
        let d = fixture(u32::from(OMAGIC));
        let a = parse(&d).unwrap();
        assert_eq!(a.kind, Kind::Omagic);
        assert_eq!(a.mid, 0);
        assert_eq!(a.text, 0x1000);
        assert_eq!(a.data, 0x400);
        assert_eq!(a.bss, 0x80);
        assert_eq!(a.syms, 0x60);
        assert_eq!(a.entry, 0x2000);
        // 32 + 0x1000 + 0x400 + 0x10 + 0x20
        assert_eq!(a.syms_at(), 0x1450);
    }

    #[test]
    fn magic_variants_and_mid() {
        assert_eq!(
            parse(&fixture(u32::from(NMAGIC))).unwrap().kind,
            Kind::Nmagic
        );
        assert_eq!(
            parse(&fixture(u32::from(ZMAGIC))).unwrap().kind,
            Kind::Zmagic
        );
        assert_eq!(
            parse(&fixture(u32::from(QMAGIC))).unwrap().kind,
            Kind::Qmagic
        );
        let other = parse(&fixture(0x1234_9999)).unwrap();
        assert_eq!(other.kind, Kind::Other(0x9999));
        assert_eq!(other.mid, 0x1234);
    }

    #[test]
    fn short_rejects() {
        assert!(parse(&[0u8; 31]).is_none());
    }
}

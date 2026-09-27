//! X11 PCF (Portable Compiled Format) bitmap font table-directory parsing.
//!
//! PCF starts `\x01fcp` (`PCF_FILE_VERSION` low byte first), followed by a
//! u32LE table count and `{type, format, size, offset}` u32LE records.
//! Table types: properties=1, accelerators=2, metrics=4, bitmaps=8,
//! ink-metrics=0x10, BDF-encodings=0x20, swidths=0x40, glyph-names=0x80,
//! BDF-accelerators=0x100.
//!
//! ```
//! use izanagi_kit::pcf;
//! let d = b"\x01fcp\x01\x00\x00\x00\x04\x00\x00\x00\x0e\x00\x00\x00\
//!           \x04\x00\x00\x00\x18\x00\x00\x00\x00\x00\x00\x00";
//! let p = pcf::parse(d).unwrap();
//! assert_eq!(p.tables[0].kind, pcf::Kind::Metrics);
//! ```

use std::vec::Vec;

/// PCF table identifiers (the `type` field's low bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `0x1` — atom/name=value property list.
    Properties,
    /// `0x2` — accelerator/metric summary.
    Accelerators,
    /// `0x4` — per-glyph metrics.
    Metrics,
    /// `0x8` — packed glyph bitmaps.
    Bitmaps,
    /// `0x10` — ink metrics.
    InkMetrics,
    /// `0x20` — encoding→glyph index map.
    BdfEncodings,
    /// `0x40` — scalable widths.
    Swidths,
    /// `0x80` — per-glyph atom names.
    GlyphNames,
    /// `0x100` — BDF-style accelerators.
    BdfAccelerators,
    /// Any other (unknown) table type.
    Other(u32),
}

impl Kind {
    fn of(v: u32) -> Kind {
        match v {
            0x1 => Kind::Properties,
            0x2 => Kind::Accelerators,
            0x4 => Kind::Metrics,
            0x8 => Kind::Bitmaps,
            0x10 => Kind::InkMetrics,
            0x20 => Kind::BdfEncodings,
            0x40 => Kind::Swidths,
            0x80 => Kind::GlyphNames,
            0x100 => Kind::BdfAccelerators,
            v => Kind::Other(v),
        }
    }
}

/// One directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    /// Decoded table kind.
    pub kind: Kind,
    /// Format word (`0xE` nibble = byte/bit/glyph-pad packing flags).
    pub format: u32,
    /// Payload size in bytes.
    pub size: u32,
    /// File offset of the payload.
    pub offset: u32,
}

/// A parsed PCF directory.
#[derive(Clone, Debug, PartialEq)]
pub struct Pcf {
    /// Tables in file order.
    pub tables: Vec<Table>,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses the header + table directory; payloads are bounds-checked.
pub fn parse(d: &[u8]) -> Option<Pcf> {
    if d.get(..4) != Some(b"\x01fcp") {
        return None;
    }
    let count = u32le(d, 4)? as usize;
    if count > 512 {
        return None;
    }
    let mut tables = Vec::with_capacity(count);
    for i in 0..count {
        let at = 8usize.checked_add(i.checked_mul(16)?)?;
        let ty = u32le(d, at)?;
        let format = u32le(d, at + 4)?;
        let size = u32le(d, at + 8)? as usize;
        let offset = u32le(d, at + 12)? as usize;
        if offset.checked_add(size)? > d.len() {
            return None;
        }
        tables.push(Table {
            kind: Kind::of(ty),
            format,
            size: size as u32,
            offset: offset as u32,
        });
    }
    Some(Pcf { tables })
}

/// First table of `kind`, if present.
pub fn find(p: &Pcf, kind: Kind) -> Option<&Table> {
    p.tables.iter().find(|t| t.kind == kind)
}

/// The payload bytes of a directory entry.
pub fn data<'a>(d: &'a [u8], t: &Table) -> Option<&'a [u8]> {
    d.get(t.offset as usize..t.offset as usize + t.size as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(ty: u32, size: u32, off: u32) -> Vec<u8> {
        let mut v = Vec::new();
        for &w in &[ty, 0xe, size, off] {
            v.extend_from_slice(&w.to_le_bytes());
        }
        v
    }

    #[test]
    fn parses_directory() {
        let mut d = b"\x01fcp".to_vec();
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&dir(0x20, 4, 24));
        d.resize(24, 0);
        d.extend_from_slice(&[9, 9, 9, 9]);
        let p = parse(&d).unwrap();
        assert_eq!(p.tables[0].kind, Kind::BdfEncodings);
        assert_eq!(find(&p, Kind::BdfEncodings).unwrap().size, 4);
        assert_eq!(data(&d, &p.tables[0]), Some(&[9u8; 4][..]));
        assert!(find(&p, Kind::Metrics).is_none());
    }

    #[test]
    fn rejects_bad_magic_and_overflow() {
        assert!(parse(b"pcf\x01").is_none());
        let mut d = b"\x01fcp".to_vec();
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&dir(1, 0xFFFF, 0xFFFF));
        assert!(parse(&d).is_none());
    }

    #[test]
    fn kind_mapping() {
        assert_eq!(Kind::of(0x8), Kind::Bitmaps);
        assert_eq!(Kind::of(0x80), Kind::GlyphNames);
        assert_eq!(Kind::of(0xdead), Kind::Other(0xdead));
    }
}

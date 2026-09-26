//! Portable Executable (PE/COFF) image header and section table.
//!
//! A PE image starts with a 64-byte DOS stub (`MZ`, `e_lfanew` at
//! `0x3C`), the `PE\0\0` signature at `e_lfanew`, the 20-byte COFF
//! header (machine, section count, timestamp, optional-header size,
//! characteristics), the optional header (`0x10B` PE32 / `0x20B`
//! PE32+), and then the section table — 40 bytes per section.
//!
//! ```
//! use izanagi_kit::pe::{parse, Machine};
//!
//! let mut d = vec![0u8; 512];
//! d[..2].copy_from_slice(b"MZ");
//! d[60..64].copy_from_slice(&128u32.to_le_bytes());   // e_lfanew
//! d[128..132].copy_from_slice(b"PE\0\0");
//! d[132..134].copy_from_slice(&0x14Cu16.to_le_bytes()); // i386
//! d[134..136].copy_from_slice(&1u16.to_le_bytes());   // 1 section
//! d[148..150].copy_from_slice(&96u16.to_le_bytes());  // opt_size
//! d[152..154].copy_from_slice(&0x10Bu16.to_le_bytes()); // PE32 magic
//! // section table at 152 + 96 = 248
//! d[248..256].copy_from_slice(b".text\0\0\0");
//! d[248 + 16..248 + 20].copy_from_slice(&512u32.to_le_bytes());
//! d[248 + 20..248 + 24].copy_from_slice(&400u32.to_le_bytes());
//! let p = parse(&d).unwrap();
//! assert_eq!(p.machine, Machine::I386);
//! assert_eq!(p.n_sections, 1);
//! let s = p.section(&d, 0).unwrap();
//! assert_eq!(s.name(), ".text");
//! ```

use std::string::String;

/// `MZ` DOS-stub signature length field offset (`e_lfanew` lives at 60).
pub const E_LFANEW_AT: usize = 60;
/// COFF file header size in bytes.
pub const COFF_SIZE: usize = 20;
/// Section-table entry size in bytes.
pub const SECTION_SIZE: usize = 40;

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// COFF machine identifier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Machine {
    /// Intel 386 (`0x014C`).
    I386,
    /// AMD64 (`0x8664`).
    Amd64,
    /// ARM 32-bit (`0x01C0`).
    Arm,
    /// ARM 64-bit (`0xAA64`).
    Arm64,
    /// Any other machine id.
    Other(u16),
}

impl Machine {
    fn of(v: u16) -> Self {
        match v {
            0x014C => Machine::I386,
            0x8664 => Machine::Amd64,
            0x01C0 => Machine::Arm,
            0xAA64 => Machine::Arm64,
            v => Machine::Other(v),
        }
    }
}

/// Optional-header flavour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opt {
    /// No optional header present (`opt_size == 0`).
    None,
    /// PE32 (`0x10B`).
    Pe32,
    /// PE32+ / 64-bit (`0x20B`).
    Pe32Plus,
    /// Any other magic.
    Other(u16),
}

/// A 40-byte section-table entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// Raw section name (up to 8 bytes, NUL-padded).
    pub name: [u8; 8],
    /// Virtual size in memory.
    pub vsize: u32,
    /// Virtual address (RVA) in memory.
    pub vaddr: u32,
    /// Size of the initialized data on disk.
    pub raw_size: u32,
    /// File offset of the raw data.
    pub raw_ptr: u32,
    /// Section characteristics flags.
    pub characteristics: u32,
}

impl Section {
    /// Section name as a string, trimmed at the first NUL.
    pub fn name(&self) -> String {
        let n = self.name.iter().position(|&b| b == 0).unwrap_or(8);
        self.name[..n]
            .iter()
            .map(|&b| if b.is_ascii() { char::from(b) } else { '?' })
            .collect()
    }

    /// The section's raw bytes inside image `d`, or `None` when the
    /// `[raw_ptr, raw_ptr + raw_size)` range leaves the buffer.
    pub fn data<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(
            usize::try_from(self.raw_ptr).ok()?
                ..usize::try_from(self.raw_ptr).ok()? + usize::try_from(self.raw_size).ok()?,
        )
    }
}

/// A parsed PE header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pe {
    /// Offset of the `PE\0\0` signature.
    pub pe_at: usize,
    /// Target machine.
    pub machine: Machine,
    /// Number of section-table entries.
    pub n_sections: u16,
    /// COFF timestamp (seconds since 1970, low entropy is allowed).
    pub timestamp: u32,
    /// Size of the optional header in bytes.
    pub opt_size: u16,
    /// COFF characteristics flags.
    pub characteristics: u16,
    /// Optional-header flavour.
    pub opt: Opt,
    /// File offset of the section table (`pe_at + 4 + 20 + opt_size`).
    pub sections_at: usize,
}

impl Pe {
    /// Read section `i`, or `None` when out of range / out of bounds.
    pub fn section(&self, d: &[u8], i: usize) -> Option<Section> {
        if i >= usize::from(self.n_sections) {
            return None;
        }
        let at = self.sections_at.checked_add(i.checked_mul(SECTION_SIZE)?)?;
        let mut name = [0u8; 8];
        name.copy_from_slice(d.get(at..at + 8)?);
        Some(Section {
            name,
            vsize: u32le(d, at + 8)?,
            vaddr: u32le(d, at + 12)?,
            raw_size: u32le(d, at + 16)?,
            raw_ptr: u32le(d, at + 20)?,
            characteristics: u32le(d, at + 36)?,
        })
    }

    /// Iterator over all `n_sections` entries.
    pub fn sections<'a>(&'a self, d: &'a [u8]) -> Sections<'a> {
        Sections { pe: self, d, i: 0 }
    }
}

/// Iterator over a PE image's section table.
pub struct Sections<'a> {
    pe: &'a Pe,
    d: &'a [u8],
    i: usize,
}

impl<'a> Iterator for Sections<'a> {
    type Item = Section;

    fn next(&mut self) -> Option<Section> {
        let s = self.pe.section(self.d, self.i)?;
        self.i += 1;
        Some(s)
    }
}

/// Parse a PE image. Returns `None` on missing `MZ`/`PE\0\0`, a
/// truncated COFF header, or a section table that leaves the buffer.
pub fn parse(d: &[u8]) -> Option<Pe> {
    if d.get(..2)? != b"MZ" {
        return None;
    }
    let pe_at = usize::try_from(u32le(d, E_LFANEW_AT)?).ok()?;
    if d.get(pe_at..pe_at + 4)? != b"PE\0\0" {
        return None;
    }
    let coff = pe_at + 4;
    let opt_size = u16le(d, coff + 16)?;
    let opt_magic = u16le(d, coff + COFF_SIZE).unwrap_or(0);
    let opt = if opt_size == 0 {
        Opt::None
    } else {
        match opt_magic {
            0x10B => Opt::Pe32,
            0x20B => Opt::Pe32Plus,
            v => Opt::Other(v),
        }
    };
    let p = Pe {
        pe_at,
        machine: Machine::of(u16le(d, coff)?),
        n_sections: u16le(d, coff + 2)?,
        timestamp: u32le(d, coff + 4)?,
        opt_size,
        characteristics: u16le(d, coff + 18)?,
        opt,
        sections_at: coff + COFF_SIZE + usize::from(opt_size),
    };
    // the whole section table must be inside the buffer
    d.get(p.sections_at..p.sections_at + usize::from(p.n_sections) * SECTION_SIZE)?;
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; 1024];
        d[..2].copy_from_slice(b"MZ");
        d[60..64].copy_from_slice(&128u32.to_le_bytes());
        d[128..132].copy_from_slice(b"PE\0\0");
        d[132..134].copy_from_slice(&0x14Cu16.to_le_bytes());
        d[134..136].copy_from_slice(&2u16.to_le_bytes());
        d[136..140].copy_from_slice(&0x5E2A4B00u32.to_le_bytes());
        d[148..150].copy_from_slice(&96u16.to_le_bytes());
        d[150..152].copy_from_slice(&0x0102u16.to_le_bytes());
        d[152..154].copy_from_slice(&0x10Bu16.to_le_bytes());
        // sections at 152 + 96 = 248
        let s0 = 248;
        d[s0..s0 + 8].copy_from_slice(b".text\0\0\0");
        d[s0 + 8..s0 + 12].copy_from_slice(&0x600u32.to_le_bytes());
        d[s0 + 12..s0 + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        d[s0 + 16..s0 + 20].copy_from_slice(&0x200u32.to_le_bytes());
        d[s0 + 20..s0 + 24].copy_from_slice(&400u32.to_le_bytes());
        d[s0 + 36..s0 + 40].copy_from_slice(&0x60000020u32.to_le_bytes());
        let s1 = s0 + 40;
        d[s1..s1 + 8].copy_from_slice(b".data\0\0\0");
        d[s1 + 16..s1 + 20].copy_from_slice(&0x100u32.to_le_bytes());
        d[s1 + 20..s1 + 24].copy_from_slice(&700u32.to_le_bytes());
        d[400] = 0xCC;
        d[400 + 0x200 - 1] = 0x90;
        d
    }

    #[test]
    fn parse_reads_coff_fields() {
        let d = image();
        let p = parse(&d).unwrap();
        assert_eq!(p.pe_at, 128);
        assert_eq!(p.machine, Machine::I386);
        assert_eq!(p.n_sections, 2);
        assert_eq!(p.timestamp, 0x5E2A4B00);
        assert_eq!(p.opt_size, 96);
        assert_eq!(p.characteristics, 0x0102);
        assert_eq!(p.opt, Opt::Pe32);
        assert_eq!(p.sections_at, 248);
    }

    #[test]
    fn sections_walk_and_slice() {
        let d = image();
        let p = parse(&d).unwrap();
        let secs: Vec<_> = p.sections(&d).collect();
        assert_eq!(secs.len(), 2);
        assert_eq!(secs[0].name(), ".text");
        assert_eq!(secs[0].vsize, 0x600);
        assert_eq!(secs[0].vaddr, 0x1000);
        assert_eq!(secs[0].characteristics, 0x60000020);
        let data = secs[0].data(&d).unwrap();
        assert_eq!(data.len(), 0x200);
        assert_eq!(data[0], 0xCC);
        assert_eq!(data[0x1FF], 0x90);
        assert_eq!(secs[1].name(), ".data");
        assert_eq!(p.section(&d, 2), None);
    }

    #[test]
    fn machine_and_opt_variants() {
        assert_eq!(Machine::of(0x8664), Machine::Amd64);
        assert_eq!(Machine::of(0x01C0), Machine::Arm);
        assert_eq!(Machine::of(0xAA64), Machine::Arm64);
        assert_eq!(Machine::of(0x9999), Machine::Other(0x9999));
        let mut d = image();
        d[152..154].copy_from_slice(&0x20Bu16.to_le_bytes());
        assert_eq!(parse(&d).unwrap().opt, Opt::Pe32Plus);
        let mut d2 = image();
        d2[152..154].copy_from_slice(&0x107u16.to_le_bytes());
        assert_eq!(parse(&d2).unwrap().opt, Opt::Other(0x107));
        let mut d3 = image();
        d3[148..150].copy_from_slice(&0u16.to_le_bytes());
        d3[150..152].copy_from_slice(&0u16.to_le_bytes());
        let p3 = parse(&d3).unwrap();
        assert_eq!(p3.opt, Opt::None);
        assert_eq!(p3.sections_at, 152);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&vec![0u8; 512]), None);
        let mut d = image();
        d[0] = b'X';
        assert_eq!(parse(&d), None);
        let mut d2 = image();
        d2[128] = b'X';
        assert_eq!(parse(&d2), None);
        let mut d3 = image();
        d3[60..64].copy_from_slice(&1000u32.to_le_bytes());
        assert_eq!(parse(&d3), None);
    }

    #[test]
    fn section_name_non_ascii_and_full() {
        let d = image();
        let p = parse(&d).unwrap();
        let mut s = p.section(&d, 0).unwrap();
        s.name = *b"12345678";
        assert_eq!(s.name(), "12345678");
        s.name[0] = 0x80;
        assert_eq!(s.name(), "?2345678");
    }

    #[test]
    fn constants() {
        assert_eq!(E_LFANEW_AT, 60);
        assert_eq!(COFF_SIZE, 20);
        assert_eq!(SECTION_SIZE, 40);
    }
}

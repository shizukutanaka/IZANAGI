//! COFF / PE — the Windows object/executable container: the 20-byte
//! COFF header (`machine`, `NumberOfSections`, symbol-table pointer,
//! `SizeOfOptionalHeader`, `Characteristics`), then the optional
//! PE32/PE32+ magic (`0x10B`/`0x20B`), then the 40-byte section table
//! (`Name[8]` + `VirtualSize`/`VirtualAddress`/`SizeOfRawData`/
//! `PointerToRawData` + flags). `.obj`, `.lib`, `.exe` all share it.
//!
//! `elf`/`macho` cover the Unix/Apple side; `coff` is the Windows peer.
//!
//! ```
//! use izanagi_kit::coff;
//! let mut f = vec![0x4C, 0x01]; // IMAGE_FILE_MACHINE_I386
//! f.extend_from_slice(&[1, 0]); // 1 section
//! f.extend_from_slice(&[0; 12]); // timestamp + sym ptr/count
//! f.extend_from_slice(&[0xE0, 0, 0x02, 0x01]); // SizeOfOptionalHeader, characteristics
//! // PE32 optional header (0x10B magic) — zeroed rest for the test
//! f.extend_from_slice(&[0x0B, 0x01]);
//! f.resize(20 + 0xE0, 0);
//! // section: ".text"
//! let mut s = b".text".to_vec(); s.resize(40, 0);
//! f.extend_from_slice(&s);
//! let c = coff::parse(&f).unwrap();
//! assert_eq!(c.machine, 0x014C);
//! assert_eq!(c.pe_magic, Some(0x10B));
//! assert_eq!(c.sections.len(), 1);
//! ```

/// One 40-byte COFF section header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// 8-byte name, NUL-padded or `/`-offset into the string table.
    pub name: String,
    /// Virtual size (total size when loaded).
    pub vsize: u32,
    /// Virtual address (RVA when loaded).
    pub vaddr: u32,
    /// Size of the raw (file) data.
    pub raw_size: u32,
    /// File offset of the raw data.
    pub raw_offset: u32,
    /// `IMAGE_SCN_*` characteristics.
    pub flags: u32,
}

/// A parsed COFF/PE header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coff {
    /// Machine type (`0x014C` i386, `0x8664` amd64, `0x01C0` arm).
    pub machine: u16,
    /// Number of section headers.
    pub nsects: u16,
    /// Size of the optional (PE) header that follows the COFF header.
    pub opt_size: u16,
    /// `0x10B` for PE32, `0x20B` for PE32+, `0x107` for ROM, or `None`
    /// when `opt_size` is zero (a bare `.obj`).
    pub pe_magic: Option<u16>,
    /// Section table.
    pub sections: Vec<Section>,
    /// File offset where the optional header begins.
    pub opt_offset: usize,
}

fn r16(d: &[u8], at: usize) -> Option<u16> {
    let a = *d.get(at)? as u16;
    let b = *d.get(at + 1)? as u16;
    Some((b << 8) | a) // COFF is little-endian
}

fn r32(d: &[u8], at: usize) -> Option<u32> {
    let a = r16(d, at)? as u32;
    let b = r16(d, at + 2)? as u32;
    Some((b << 16) | a)
}

/// Parse a COFF or PE file header plus its section table.
/// `None` on truncation or a section table running past the data.
pub fn parse(d: &[u8]) -> Option<Coff> {
    let machine = r16(d, 0)?;
    let nsects = r16(d, 2)?;
    let opt_size = r16(d, 16)?;
    let opt_offset = 20;
    let pe_magic = if opt_size > 0 {
        let m = r16(d, opt_offset)?;
        match m {
            0x10B | 0x20B | 0x107 => Some(m),
            _ => None, // unknown optional header — still a COFF, just not PE
        }
    } else {
        None
    };
    let mut at = opt_offset.checked_add(opt_size as usize)?;
    let mut sections = Vec::new();
    for _ in 0..nsects {
        let name_bytes = d.get(at..at + 8)?;
        let nul = name_bytes.iter().position(|&b| b == 0).unwrap_or(8);
        let name = String::from_utf8_lossy(&name_bytes[..nul]).to_string();
        let vsize = r32(d, at + 8)?;
        let vaddr = r32(d, at + 12)?;
        let raw_size = r32(d, at + 16)?;
        let raw_offset = r32(d, at + 20)?;
        let flags = r32(d, at + 36)?;
        sections.push(Section {
            name,
            vsize,
            vaddr,
            raw_size,
            raw_offset,
            flags,
        });
        at = at.checked_add(40)?;
    }
    Some(Coff {
        machine,
        nsects,
        opt_size,
        pe_magic,
        sections,
        opt_offset,
    })
}

/// Machine-type name for display.
pub fn machine_name(m: u16) -> &'static str {
    match m {
        0x014C => "i386",
        0x8664 => "amd64",
        0x01C0 => "arm",
        0x01C4 => "armv7",
        0xAA64 => "aarch64",
        0x0200 => "ia64",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pe32() -> Vec<u8> {
        let mut f = vec![0x64, 0x86]; // IMAGE_FILE_MACHINE_AMD64
        f.extend_from_slice(&[2, 0]); // 2 sections
        f.extend_from_slice(&[0x5D, 0x3B, 0x7D, 0x5F]); // timestamp
        f.extend_from_slice(&[0; 8]); // sym table
        f.extend_from_slice(&[0xF0, 0]); // opt_size = 240
        f.extend_from_slice(&[0x22, 0x02]); // characteristics
                                            // PE32+ optional header: magic 0x20B
        f.extend_from_slice(&[0x0B, 0x02]);
        f.resize(20 + 0xF0, 0);
        // .text
        let mut s = b".text".to_vec();
        s.resize(8, 0);
        f.extend_from_slice(&s);
        f.extend_from_slice(&[0x00, 0x10, 0, 0]); // vsize 0x1000
        f.extend_from_slice(&[0x00, 0x10, 0, 0]); // vaddr
        f.extend_from_slice(&[0x00, 0x08, 0, 0]); // raw_size
        f.extend_from_slice(&[0x00, 0x04, 0, 0]); // raw_offset
        f.extend_from_slice(&[0; 12]); // relocs etc
        f.extend_from_slice(&[0x20, 0, 0, 0x60]); // flags EXEC|CODE|ALIGN16
                                                  // .data
        let mut s = b".data".to_vec();
        s.resize(8, 0);
        f.extend_from_slice(&s);
        f.resize(f.len() + 40, 0);
        f
    }

    #[test]
    fn pe32_full_walk() {
        let c = parse(&pe32()).unwrap();
        assert_eq!(c.machine, 0x8664);
        assert_eq!(c.nsects, 2);
        assert_eq!(c.pe_magic, Some(0x20B));
        assert_eq!(c.opt_size, 0xF0);
        assert_eq!(c.sections[0].name, ".text");
        assert_eq!(c.sections[0].vsize, 0x1000);
        assert_eq!(c.sections[0].raw_size, 0x800);
        assert_eq!(c.sections[1].name, ".data");
        assert_eq!(machine_name(0x8664), "amd64");
        assert_eq!(machine_name(0x014C), "i386");
        assert_eq!(machine_name(0xFFFF), "unknown");
    }

    #[test]
    fn obj_no_pe() {
        let mut f = vec![0x4C, 0x01]; // i386
        f.extend_from_slice(&[1, 0]);
        f.extend_from_slice(&[0; 12]);
        f.extend_from_slice(&[0, 0]); // no optional header
        f.extend_from_slice(&[0; 2]);
        let mut s = b".drectve".to_vec();
        s.resize(40, 0);
        f.extend_from_slice(&s);
        let c = parse(&f).unwrap();
        assert_eq!(c.pe_magic, None);
        assert_eq!(c.sections[0].name, ".drectve");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x4C]).is_none()); // truncated
        let mut f = pe32();
        f.truncate(19);
        assert!(parse(&f).is_none());
    }
}

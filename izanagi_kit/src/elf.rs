//! ELF — Executable and Linkable Format (System V ABI / TIS ELF 1.2):
//! e_ident magic and class/data resolution, ELF32/64 header fields,
//! program headers (`PT_LOAD` segments) and section headers with
//! `shstrtab` name resolution. Both endiannesses are honoured from
//! `EI_DATA`; every read is bounds-checked and the API is total.
//!
//! `der`/`tzif`/`pcap` cover certificate/timezone/capture binaries;
//! `elf` covers the container the toolchain actually emits.
//!
//! ```
//! use izanagi_kit::elf;
//! let mut f = vec![0x7f, b'E', b'L', b'F', 2, 1, 1, 0]; // 64-bit LE
//! f.resize(16, 0);
//! f.extend_from_slice(&[2, 0]); // ET_EXEC
//! f.extend_from_slice(&[0x3e, 0]); // x86-64
//! f.extend_from_slice(&[1, 0, 0, 0]); // version
//! f.resize(64, 0); // header size
//! let e = elf::parse(&f).unwrap();
//! assert!(e.is64 && e.le && e.ty == 2 && e.machine == 0x3e);
//! ```

/// A program (segment) header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phdr {
    /// Segment type (1 = `PT_LOAD`, 2 = `PT_DYNAMIC`, …).
    pub ty: u32,
    /// `PF_X/PF_W/PF_R` flags (stored after `ty` in ELF64).
    pub flags: u32,
    /// File offset of the segment.
    pub offset: u64,
    /// Virtual address.
    pub vaddr: u64,
    /// Bytes in file.
    pub filesz: u64,
    /// Bytes in memory (may exceed `filesz` — `.bss`).
    pub memsz: u64,
}

/// A section header (name resolved to the `shstrtab` offset).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shdr {
    /// Offset into the string table.
    pub name: u32,
    /// `SHT_*` type (1 = PROGBITS, 2 = SYMTAB, 3 = STRTAB, …).
    pub ty: u32,
    /// File offset of the section body.
    pub offset: u64,
    /// Body size in bytes.
    pub size: u64,
    /// Section flags.
    pub flags: u64,
    /// Virtual address.
    pub addr: u64,
}

/// A parsed ELF header set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elf {
    /// ELFCLASS64.
    pub is64: bool,
    /// ELFDATA2LSB.
    pub le: bool,
    /// `e_type` (1 REL, 2 EXEC, 3 DYN, 4 CORE).
    pub ty: u16,
    /// `e_machine` (0x3e x86-64, 0x28 ARM, 0xb7 AArch64, 0xf3 RISC-V).
    pub machine: u16,
    /// `e_entry`.
    pub entry: u64,
    /// Program headers.
    pub phdrs: Vec<Phdr>,
    /// Section headers.
    pub shdrs: Vec<Shdr>,
    /// `e_shstrndx` (string-table section index; 0 = none).
    pub shstrndx: u16,
}

fn r16(d: &[u8], at: usize, le: bool) -> Option<u16> {
    let a = *d.get(at)? as u16;
    let b = *d.get(at + 1)? as u16;
    Some(if le { (b << 8) | a } else { (a << 8) | b })
}

fn r32(d: &[u8], at: usize, le: bool) -> Option<u32> {
    let lo = r16(d, at, le)? as u32;
    let hi = r16(d, at + 2, le)? as u32;
    Some(if le { (hi << 16) | lo } else { (lo << 16) | hi })
}

fn r64(d: &[u8], at: usize, le: bool) -> Option<u64> {
    let lo = r32(d, at, le)? as u64;
    let hi = r32(d, at + 4, le)? as u64;
    Some(if le { (hi << 32) | lo } else { (lo << 32) | hi })
}

/// Parse a whole ELF image; `None` on bad magic, unknown class or
/// data encoding, or any header field pointing past the buffer.
pub fn parse(d: &[u8]) -> Option<Elf> {
    if d.get(..4)? != [0x7f, b'E', b'L', b'F'] {
        return None;
    }
    let is64 = match *d.get(4)? {
        1 => false,
        2 => true,
        _ => return None,
    };
    let le = match *d.get(5)? {
        1 => true,
        2 => false,
        _ => return None,
    };
    if *d.get(6)? != 1 {
        return None; // EV_CURRENT
    }
    let ehsize = if is64 { 64 } else { 52 };
    let ty = r16(d, 16, le)?;
    let machine = r16(d, 18, le)?;
    let (entry, phoff, shoff) = if is64 {
        (r64(d, 24, le)?, r64(d, 32, le)?, r64(d, 40, le)?)
    } else {
        (
            r32(d, 24, le)? as u64,
            r32(d, 28, le)? as u64,
            r32(d, 32, le)? as u64,
        )
    };
    let phentsize = r16(d, ehsize - 10, le)? as usize;
    let phnum = r16(d, ehsize - 8, le)? as usize;
    let shentsize = r16(d, ehsize - 6, le)? as usize;
    let shnum = r16(d, ehsize - 4, le)? as usize;
    let shstrndx = r16(d, ehsize - 2, le)?;
    let phentsize = phentsize.max(if is64 { 56 } else { 32 });
    let shentsize = shentsize.max(if is64 { 64 } else { 40 });
    let mut phdrs = Vec::with_capacity(phnum.min(4096));
    for i in 0..phnum {
        let at = usize::try_from(phoff)
            .ok()?
            .checked_add(i.checked_mul(phentsize)?)?;
        if at >= d.len() {
            return None;
        }
        let (ty, flags, offset, vaddr, filesz, memsz) = if is64 {
            (
                r32(d, at, le)?,
                r32(d, at + 4, le)?,
                r64(d, at + 8, le)?,
                r64(d, at + 16, le)?,
                r64(d, at + 32, le)?,
                r64(d, at + 40, le)?,
            )
        } else {
            (
                r32(d, at, le)?,
                r32(d, at + 24, le)?,
                r32(d, at + 4, le)? as u64,
                r32(d, at + 8, le)? as u64,
                r32(d, at + 16, le)? as u64,
                r32(d, at + 20, le)? as u64,
            )
        };
        phdrs.push(Phdr {
            ty,
            flags,
            offset,
            vaddr,
            filesz,
            memsz,
        });
    }
    let mut shdrs = Vec::with_capacity(shnum.min(65535));
    for i in 0..shnum {
        let at = usize::try_from(shoff)
            .ok()?
            .checked_add(i.checked_mul(shentsize)?)?;
        if at >= d.len() {
            return None;
        }
        let (name, ty, flags, addr, offset, size) = if is64 {
            (
                r32(d, at, le)?,
                r32(d, at + 4, le)?,
                r64(d, at + 8, le)?,
                r64(d, at + 16, le)?,
                r64(d, at + 24, le)?,
                r64(d, at + 32, le)?,
            )
        } else {
            (
                r32(d, at, le)?,
                r32(d, at + 4, le)?,
                r32(d, at + 8, le)? as u64,
                r32(d, at + 12, le)? as u64,
                r32(d, at + 16, le)? as u64,
                r32(d, at + 20, le)? as u64,
            )
        };
        shdrs.push(Shdr {
            name,
            ty,
            offset,
            size,
            flags,
            addr,
        });
    }
    Some(Elf {
        is64,
        le,
        ty,
        machine,
        entry,
        phdrs,
        shdrs,
        shstrndx,
    })
}

/// Resolve section `i`'s name through the `shstrtab` section.
pub fn section_name(d: &[u8], e: &Elf, i: usize) -> Option<String> {
    let tab = e.shdrs.get(e.shstrndx as usize)?;
    let sh = e.shdrs.get(i)?;
    let base = usize::try_from(tab.offset)
        .ok()?
        .checked_add(sh.name as usize)?;
    let end = usize::try_from(tab.offset.checked_add(tab.size)?).ok()?;
    let name_bytes = d.get(base..end)?;
    let nul = name_bytes
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(name_bytes.len());
    Some(String::from_utf8_lossy(&name_bytes[..nul]).into_owned())
}

/// Raw bytes of the named section.
pub fn section<'a>(d: &'a [u8], e: &Elf, name: &str) -> Option<&'a [u8]> {
    for i in 0..e.shdrs.len() {
        if section_name(d, e, i).as_deref() == Some(name) {
            let sh = &e.shdrs[i];
            let lo = usize::try_from(sh.offset).ok()?;
            let hi = usize::try_from(sh.offset.checked_add(sh.size)?).ok()?;
            return d.get(lo..hi);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal ELF64-LE with one PT_LOAD and two sections
    /// (`.text` + `.shstrtab`).
    fn fixture64() -> Vec<u8> {
        let mut f = vec![0x7f, b'E', b'L', b'F', 2, 1, 1, 0];
        f.resize(16, 0);
        f.extend_from_slice(&[2, 0, 0x3e, 0]); // EXEC, x86-64
        f.extend_from_slice(&[1, 0, 0, 0]);
        f.extend_from_slice(&[0x78, 0x05, 0x40, 0, 0, 0, 0, 0]); // entry
        f.extend_from_slice(&[64, 0, 0, 0, 0, 0, 0, 0]); // phoff
        f.extend_from_slice(&[0xA0, 0x01, 0, 0, 0, 0, 0, 0]); // shoff = 0x1A0
        f.extend_from_slice(&[0; 4]); // flags
        f.extend_from_slice(&[64, 0]); // ehsize
        f.extend_from_slice(&[56, 0, 1, 0]); // phentsize, phnum
        f.extend_from_slice(&[64, 0, 3, 0]); // shentsize, shnum
        f.extend_from_slice(&[2, 0]); // shstrndx = 2
                                      // phdr: PT_LOAD, R+X, off 0x1000, vaddr 0x400000, filesz=memsz=0x10
        f.extend_from_slice(&[1, 0, 0, 0, 5, 0, 0, 0]);
        f.extend_from_slice(&[0, 0x10, 0, 0, 0, 0, 0, 0]);
        f.extend_from_slice(&[0, 0, 0x40, 0, 0, 0, 0, 0]);
        f.extend_from_slice(&[0; 8]); // paddr
        f.extend_from_slice(&[0x10, 0, 0, 0, 0, 0, 0, 0]); // filesz
        f.extend_from_slice(&[0x10, 0, 0, 0, 0, 0, 0, 0]); // memsz
        f.extend_from_slice(&[0; 8]); // align
        assert_eq!(f.len(), 0x78);
        f.resize(0x1A0, 0);
        // shdr 0 (null)
        f.extend_from_slice(&[0; 64]);
        // shdr 1: name=1, SHT_PROGBITS, off 0x1000 size 0x10
        f.extend_from_slice(&[1, 0, 0, 0, 1, 0, 0, 0]);
        f.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0]); // SHF_ALLOC|EXECINSTR
        f.extend_from_slice(&[0, 0, 0x40, 0, 0, 0, 0, 0]); // addr
        f.extend_from_slice(&[0, 0x10, 0, 0, 0, 0, 0, 0]); // offset
        f.extend_from_slice(&[0x10, 0, 0, 0, 0, 0, 0, 0]); // size
        f.extend_from_slice(&[0; 24]); // link, info, addralign, entsize
                                       // shdr 2: name=7, SHT_STRTAB, off 0x1200 size 17
        f.extend_from_slice(&[7, 0, 0, 0, 3, 0, 0, 0]);
        f.extend_from_slice(&[0; 16]);
        f.extend_from_slice(&[0, 0x12, 0, 0, 0, 0, 0, 0]);
        f.extend_from_slice(&[17, 0, 0, 0, 0, 0, 0, 0]);
        f.extend_from_slice(&[0; 24]);
        f.resize(0x1000, 0);
        f.extend_from_slice(&[0xCC; 0x10]); // .text body
        f.resize(0x1200, 0);
        f.extend_from_slice(b"\x00.text\x00.shstrtab\x00");
        f
    }

    #[test]
    fn elf64_full_walk() {
        let f = fixture64();
        let e = parse(&f).unwrap();
        assert!(e.is64 && e.le);
        assert_eq!(e.ty, 2);
        assert_eq!(e.machine, 0x3e);
        assert_eq!(e.entry, 0x400578);
        assert_eq!(e.phdrs.len(), 1);
        assert_eq!(e.phdrs[0].ty, 1);
        assert_eq!(e.phdrs[0].flags, 5);
        assert_eq!(e.phdrs[0].offset, 0x1000);
        assert_eq!(e.phdrs[0].filesz, 0x10);
        assert_eq!(e.shdrs.len(), 3);
        assert_eq!(section_name(&f, &e, 1).unwrap(), ".text");
        assert_eq!(section_name(&f, &e, 2).unwrap(), ".shstrtab");
        assert_eq!(section(&f, &e, ".text").unwrap(), &[0xCC; 0x10]);
        assert!(section(&f, &e, ".data").is_none());
    }

    #[test]
    fn elf32_be() {
        let mut f = vec![0x7f, b'E', b'L', b'F', 1, 2, 1, 0]; // 32-bit BE
        f.resize(16, 0);
        f.extend_from_slice(&[0, 3, 0, 8]); // DYN, MIPS
        f.extend_from_slice(&[0, 0, 0, 1]);
        f.extend_from_slice(&[0x10, 0x00, 0x00, 0x40]); // entry
        f.extend_from_slice(&[0, 0, 0, 0x34]); // phoff
        f.extend_from_slice(&[0, 0, 0, 0]); // shoff
        f.extend_from_slice(&[0; 4]);
        f.extend_from_slice(&[0, 52]); // ehsize
        f.extend_from_slice(&[0, 32, 0, 0]); // phentsize, phnum = 0
        f.extend_from_slice(&[0, 40, 0, 0]); // shentsize, shnum = 0
        f.extend_from_slice(&[0, 0]); // shstrndx
        let e = parse(&f).unwrap();
        assert!(!e.is64 && !e.le && e.ty == 3 && e.machine == 8);
        assert_eq!(e.entry, 0x1000_0040);
        assert!(e.phdrs.is_empty() && e.shdrs.is_empty());
    }

    #[test]
    fn malformed_inputs_degrade() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x7fELF\x09\x01\x01").is_none()); // bad class
        assert!(parse(b"\x7fELF\x01\x09\x01").is_none()); // bad data
        let mut f = fixture64();
        f[6] = 2; // EV != 1
        assert!(parse(&f).is_none());
        let mut f = fixture64();
        f.truncate(0x60); // phdr points past end
        assert!(parse(&f).is_none());
        let f = fixture64();
        let e = parse(&f).unwrap();
        // bad shstrndx falls out of range → None, not panic
        let mut e2 = e.clone();
        e2.shstrndx = 9;
        assert!(section_name(&f, &e2, 1).is_none());
    }
}

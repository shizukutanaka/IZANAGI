//! Mach-O — the macOS executable/object format: `MH_MAGIC`/`MH_CIGAM`/
//! `MH_MAGIC_64`/`MH_CIGAM_64` thin binaries and the `FAT_MAGIC`
//! (`0xCAFEBABE` BE) multi-arch container. Thin headers expose
//! `cputype`/`filetype` and the load-command stream (`LC_SEGMENT`,
//! `LC_SYMTAB`, `LC_UUID`, `LC_MAIN`, …) walked as `(type, size, offset)`
//! triples; fat headers expose the per-architecture slice table.
//!
//! `elf` covers the toolchain target; `macho` covers the Apple side.
//!
//! ```
//! use izanagi_kit::macho;
//! let mut f = vec![0xCF, 0xFA, 0xED, 0xFE]; // MH_MAGIC_64 (LE)
//! f.extend_from_slice(&[0x07,0,0,1, 0,0,0,0, 2,0,0,0, 1,0,0,0]); // cpu x86_64, exec, ncmds=1
//! f.extend_from_slice(&[0x38,0,0,0, 0,0,0,0]); // sizeofcmds, flags
//! f.extend_from_slice(&[0,0,0,0]); // reserved (64-bit only)
//! f.extend_from_slice(&[0x19,0,0,0, 0x38,0,0,0]); // LC_SEGMENT_64
//! f.resize(72 + 0x38, 0);
//! let m = macho::parse(&f).unwrap();
//! assert!(m.is64 && m.cputype == 0x01000007 && m.cmds.len() == 1);
//! assert_eq!(m.cmds[0].ty, 0x19);
//! ```

/// One load command (`cmd`, `cmdsize`, file offset of the command).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cmd {
    /// Load-command type (`LC_SEGMENT`=1, `LC_SYMTAB`=2, `LC_UUID`=0x1B,
    /// `LC_SEGMENT_64`=0x19, `LC_MAIN`=0x80000028, …).
    pub ty: u32,
    /// Total size of this command including the 8-byte header.
    pub size: u32,
    /// File offset where the command starts.
    pub offset: usize,
}

/// A fat-binary architecture slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arch {
    /// CPU type (`0x01000007` = x86_64, `0x0100000C` = arm64).
    pub cputype: u32,
    /// Byte offset of the slice inside the fat file.
    pub offset: u32,
    /// Slice length in bytes.
    pub size: u32,
}

/// A parsed Mach-O header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachO {
    /// `true` for the 64-bit (0xFEEDFACF) layout.
    pub is64: bool,
    /// `true` when the file is big-endian (`MH_CIGAM`); thin files on
    /// disk are almost always little-endian.
    pub be: bool,
    /// CPU type field.
    pub cputype: u32,
    /// `MH_EXECUTE`=2, `MH_DYLIB`=6, `MH_BUNDLE`=8, `MH_OBJECT`=1, …
    pub filetype: u32,
    /// Declared number of load commands.
    pub ncmds: u32,
    /// Resolved load commands (truncated at `ncmds` or file end).
    pub cmds: Vec<Cmd>,
}

fn r16(d: &[u8], at: usize, be: bool) -> Option<u32> {
    let a = *d.get(at)? as u32;
    let b = *d.get(at + 1)? as u32;
    Some(if be { (a << 8) | b } else { (b << 8) | a })
}

fn r32(d: &[u8], at: usize, be: bool) -> Option<u32> {
    let a = r16(d, at, be)?;
    let b = r16(d, at + 2, be)?;
    Some(if be { (a << 16) | b } else { (b << 16) | a })
}

/// Parse a thin Mach-O header and walk its load-command table.
/// `None` on bad magic, truncation, or a corrupt command stream
/// (a `cmdsize` < 8 or one that runs off the end is a hard error —
/// the file is genuinely malformed, not merely unusual).
pub fn parse(d: &[u8]) -> Option<MachO> {
    let magic = r32(d, 0, false)?; // read LE first
    let (is64, be) = match magic {
        0xFEEDFACE => (false, false),
        0xFEEDFACF => (true, false),
        0xCEFAEDFE => (false, true),
        0xCFFAEDFE => (true, true),
        _ => return None,
    };
    let cputype = r32(d, 4, be)?;
    let filetype = r32(d, 12, be)?;
    let ncmds = r32(d, 16, be)?;
    let mut at = if is64 { 32 } else { 28 };
    let mut cmds = Vec::new();
    for _ in 0..ncmds {
        let ty = r32(d, at, be)?;
        let size = r32(d, at + 4, be)?;
        if size < 8 || at.checked_add(size as usize)? > d.len() {
            return None;
        }
        cmds.push(Cmd {
            ty,
            size,
            offset: at,
        });
        at += size as usize;
    }
    Some(MachO {
        is64,
        be,
        cputype,
        filetype,
        ncmds,
        cmds,
    })
}

/// Parse a fat (multi-arch) Mach-O: `FAT_MAGIC`/`FAT_CIGAM` header
/// plus `nfat` architecture entries. `None` when the magic isn't fat.
pub fn parse_fat(d: &[u8]) -> Option<Vec<Arch>> {
    let magic = r32(d, 0, true)?; // fat headers are natively BE-read
                                  // FAT_MAGIC files store every field big-endian; FAT_CIGAM
                                  // (the byte-swapped magic) signals little-endian fields.
    let be = match magic {
        0xCAFEBABE => true,
        0xBEBAFECA => false,
        _ => return None,
    };
    let nfat = r32(d, 4, be)?;
    let mut out = Vec::new();
    for i in 0..nfat {
        let at = 8usize.checked_add((i as usize).checked_mul(20)?)?;
        let cputype = r32(d, at, be)?;
        let offset = r32(d, at + 8, be)?;
        let size = r32(d, at + 12, be)?;
        out.push(Arch {
            cputype,
            offset,
            size,
        });
    }
    Some(out)
}

/// Load-command type name for display (`LC_SEGMENT_64`, `LC_SYMTAB`, …).
pub fn cmd_name(ty: u32) -> &'static str {
    match ty {
        0x1 => "LC_SEGMENT",
        0x2 => "LC_SYMTAB",
        0x4 => "LC_THREAD",
        0xB => "LC_DYSYMTAB",
        0xC => "LC_LOAD_DYLIB",
        0x18 => "LC_DYLD_INFO",
        0x19 => "LC_SEGMENT_64",
        0x1B => "LC_UUID",
        0x1D => "LC_CODE_SIGNATURE",
        0x1F => "LC_REEXPORT_DYLIB",
        0x21 => "LC_ENCRYPTION_INFO",
        0x26 => "LC_VERSION_MIN_MACOSX",
        0x80000028 => "LC_MAIN",
        _ => "LC_?",
    }
}

/// The `LC_SEGMENT`/`LC_SEGMENT_64` name field (16-byte `segname`).
pub fn seg_name(d: &[u8], cmd: &Cmd) -> Option<String> {
    let at = cmd.offset.checked_add(8)?;
    let mut s = String::new();
    for i in 0..16 {
        let b = *d.get(at + i)?;
        if b == 0 {
            break;
        }
        s.push(b as char);
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thin64() -> Vec<u8> {
        let mut f = vec![0xCF, 0xFA, 0xED, 0xFE];
        f.extend_from_slice(&[0x07, 0, 0, 1]); // cputype x86_64
        f.extend_from_slice(&[3, 0, 0, 0]); // cpusubtype
        f.extend_from_slice(&[2, 0, 0, 0]); // MH_EXECUTE
        f.extend_from_slice(&[3, 0, 0, 0]); // ncmds = 3
        f.extend_from_slice(&[0x88, 0, 0, 0]); // sizeofcmds
        f.extend_from_slice(&[0, 0, 0, 0x20]); // flags
        f.extend_from_slice(&[0, 0, 0, 0]); // reserved
                                            // LC_SEGMENT_64 (0x38) with segname "__TEXT"
        f.extend_from_slice(&[0x19, 0, 0, 0, 0x38, 0, 0, 0]);
        let mut name = b"__TEXT".to_vec();
        name.resize(16, 0);
        f.extend_from_slice(&name);
        f.resize(32 + 0x38, 0);
        // LC_SYMTAB (0x18)
        f.extend_from_slice(&[0x02, 0, 0, 0, 0x18, 0, 0, 0]);
        f.resize(32 + 0x38 + 0x18, 0);
        // LC_UUID (0x18)
        f.extend_from_slice(&[0x1B, 0, 0, 0, 0x18, 0, 0, 0]);
        f.resize(32 + 0x38 + 0x18 + 0x18, 0);
        f
    }

    #[test]
    fn thin64_full_walk() {
        let f = thin64();
        let m = parse(&f).unwrap();
        assert!(m.is64 && !m.be);
        assert_eq!(m.cputype, 0x01000007);
        assert_eq!(m.filetype, 2);
        assert_eq!(m.ncmds, 3);
        assert_eq!(m.cmds.len(), 3);
        assert_eq!(m.cmds[0].ty, 0x19);
        assert_eq!(m.cmds[1].ty, 0x02);
        assert_eq!(m.cmds[2].ty, 0x1B);
        assert_eq!(seg_name(&f, &m.cmds[0]).unwrap(), "__TEXT");
        assert_eq!(cmd_name(0x19), "LC_SEGMENT_64");
        assert_eq!(cmd_name(0x1B), "LC_UUID");
        assert_eq!(cmd_name(0x80000028), "LC_MAIN");
        assert_eq!(cmd_name(0xDEAD), "LC_?");
    }

    #[test]
    fn thin32_be() {
        let mut f = vec![0xFE, 0xED, 0xFA, 0xCE]; // BE file bytes → CIGAM
        f.extend_from_slice(&[0, 0, 0, 7]); // cputype i386
        f.extend_from_slice(&[0, 0, 0, 3]);
        f.extend_from_slice(&[0, 0, 0, 1]); // MH_OBJECT
        f.extend_from_slice(&[0, 0, 0, 0]); // ncmds = 0
        f.extend_from_slice(&[0; 8]);
        let m = parse(&f).unwrap();
        assert!(!m.is64 && m.be && m.cputype == 7 && m.filetype == 1);
    }

    #[test]
    fn fat_table() {
        let mut f = vec![0xCA, 0xFE, 0xBA, 0xBE]; // FAT_MAGIC
        f.extend_from_slice(&[0, 0, 0, 2]); // nfat = 2
                                            // arch 1: x86_64
        f.extend_from_slice(&[0x01, 0, 0, 0x07]); // cputype
        f.extend_from_slice(&[0, 0, 0, 3]); // subtype
        f.extend_from_slice(&[0, 0, 0x10, 0]); // offset
        f.extend_from_slice(&[0, 0, 0x08, 0]); // size
        f.extend_from_slice(&[0, 0, 0, 0]); // align
                                            // arch 2: arm64
        f.extend_from_slice(&[0x01, 0, 0, 0x0C]);
        f.extend_from_slice(&[0, 0, 0, 0]);
        f.extend_from_slice(&[0, 0, 0x18, 0]);
        f.extend_from_slice(&[0, 0, 0x04, 0]);
        f.extend_from_slice(&[0, 0, 0, 0]);
        let a = parse_fat(&f).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].cputype, 0x01000007);
        assert_eq!(a[0].offset, 0x1000);
        assert_eq!(a[1].cputype, 0x0100000C);
        assert!(parse_fat(&thin64()).is_none());
    }

    #[test]
    fn malformed_inputs_degrade() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"MZ\x90\x00").is_none()); // DOS, not Mach-O
        let mut bad = thin64();
        bad.truncate(40); // cut inside load commands
        assert!(parse(&bad).is_none());
        let mut bad2 = thin64();
        // corrupt a cmdsize to be < 8
        bad2[32 + 4] = 4;
        bad2[32 + 5] = 0;
        assert!(parse(&bad2).is_none());
        assert!(parse_fat(&thin64()).is_none());
    }

    #[test]
    fn determinism_is_structural() {
        let f = thin64();
        let a = parse(&f).unwrap();
        let b = parse(&f).unwrap();
        assert_eq!(a, b);
    }
}

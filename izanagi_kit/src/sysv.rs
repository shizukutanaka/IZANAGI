//! System V / Xenix filesystem superblock scanner.
//!
//! The superblock occupies the second sector (byte offset `0x200`) of the
//! device. This parser implements the Linux **SysV4** layout:
//!
//! `s_isize u16` \@0, `s_pad0 u16` \@2, `s_fsize u32` \@4,
//! `s_nfree u16` \@8, `s_pad1 u16` \@10, `s_free[50] u32` \@12,
//! `s_ninode u16` \@0xDC, `s_pad2 u16` \@0xDE, `s_inode[50] u16` \@0xE0,
//! `s_flock u8` \@0x144, `s_ilock u8` \@0x145, `s_fmod u8` \@0x146,
//! `s_ronly u8` \@0x147, `s_time u32` \@0x148, `s_dinfo[4] u16` \@0x14C,
//! `s_tfree u32` \@0x154, `s_tinode u16` \@0x158, `s_pad3 u16` \@0x15A,
//! `s_fname[6]` \@0x15C, `s_fpack[6]` \@0x162, `s_clean u8` \@0x168,
//! `s_fill[13]` \@0x169, `s_magic u32` \@0x1F8, `s_type u32` \@0x1FC.
//!
//! `s_magic` must read `0xFD187E20` (SysV) or `0x2B5544` (Xenix) in
//! either byte order; the order is detected from it.
//!
//! ```
//! let mut f = vec![0u8; 0x200 + 512];
//! let s = 0x200;
//! f[s + 0x1F8..s + 0x1FC].copy_from_slice(&0xFD18_7E20u32.to_le_bytes()); // s_magic
//! f[s + 0x1FC..s + 0x200].copy_from_slice(&0xFD18_7E20u32.to_le_bytes()); // s_type
//! f[s + 4..s + 8].copy_from_slice(&40000u32.to_le_bytes());  // s_fsize
//! f[s + 0x154..s + 0x158].copy_from_slice(&1200u32.to_le_bytes()); // s_tfree
//! f[s + 0x15C..s + 0x160].copy_from_slice(b"root");           // s_fname
//! let v = izanagi_kit::sysv::parse(&f).unwrap();
//! assert_eq!(v.fsize, 40000);
//! assert_eq!(v.free_blocks, 1200);
//! assert_eq!(v.volume_name.as_deref(), Some("root"));
//! ```
//!
//! Reference: Linux `fs/sysv/sysv.h` (`SYSV_MAGIC = 0xfd187e20`,
//! `XENIX_SUPER_MAGIC = 0x2b5544`), `include/uapi/linux/xenix.h`.

/// Parsed System V / Xenix superblock.
#[derive(Debug, Clone, PartialEq)]
pub struct Sysv {
    /// `true` when the superblock was stored big-endian.
    pub big_endian: bool,
    /// Size of the inode area in blocks (`s_isize`).
    pub inode_size: u16,
    /// Total filesystem size in blocks (`s_fsize`).
    pub fsize: u32,
    /// Number of free-list entries currently cached (`s_nfree`).
    pub nfree: u16,
    /// Number of cached free inode numbers (`s_ninode`).
    pub ninode: u16,
    /// Total free blocks (`s_tfree`).
    pub free_blocks: u32,
    /// Total free inodes (`s_tinode`).
    pub free_inodes: u16,
    /// Volume label (`s_fname` + `s_fpack`, up to 12 bytes).
    pub volume_name: Option<String>,
    /// Filesystem type field (`s_type`); `0xFD187E20` for SysV4.
    pub fs_type: u32,
    /// Clean flag byte (`s_clean`).
    pub clean: u8,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

fn u16be(d: &[u8], off: usize) -> u16 {
    ((d[off] as u16) << 8) | d[off + 1] as u16
}

fn u32le(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

fn u32be(d: &[u8], off: usize) -> u32 {
    ((d[off] as u32) << 24)
        | ((d[off + 1] as u32) << 16)
        | ((d[off + 2] as u32) << 8)
        | d[off + 3] as u32
}

/// Parse a device image; `None` if no recognised magic is present.
pub fn parse(d: &[u8]) -> Option<Sysv> {
    const SB: usize = 0x200;
    if d.len() < SB + 0x200 {
        return None;
    }
    const MAGIC_SYSV: u32 = 0xFD18_7E20;
    const MAGIC_XENIX: u32 = 0x002B_5544;
    let le = u32le(d, SB + 0x1F8);
    let be = u32be(d, SB + 0x1F8);
    let (big, magic) = if le == MAGIC_SYSV || le == MAGIC_XENIX {
        (false, le)
    } else if be == MAGIC_SYSV || be == MAGIC_XENIX {
        (true, be)
    } else {
        return None;
    };
    let fs_type = if big {
        u32be(d, SB + 0x1FC)
    } else {
        u32le(d, SB + 0x1FC)
    };
    // SysV4 expects s_type == magic; Xenix leaves it implementation-defined.
    if magic == MAGIC_SYSV && fs_type != MAGIC_SYSV {
        return None;
    }
    let rd16: fn(&[u8], usize) -> u16 = if big { u16be } else { u16le };
    let rd32: fn(&[u8], usize) -> u32 = if big { u32be } else { u32le };
    // Volume label: s_fname[6] @ 0x15C + s_fpack[6] @ 0x162.
    let mut name = String::new();
    for i in 0..12 {
        let c = d[SB + 0x15C + i];
        if c == 0 {
            break;
        }
        if !(0x20..=0x7E).contains(&c) {
            return None;
        }
        name.push(c as char);
    }
    Some(Sysv {
        big_endian: big,
        inode_size: rd16(d, SB),
        fsize: rd32(d, SB + 4),
        nfree: rd16(d, SB + 8),
        ninode: rd16(d, SB + 0xDC),
        free_blocks: rd32(d, SB + 0x154),
        free_inodes: rd16(d, SB + 0x158),
        volume_name: if name.is_empty() { None } else { Some(name) },
        fs_type,
        clean: d[SB + 0x168],
    })
}

/// `true` if the image looks like a System V / Xenix filesystem.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sysv() -> Vec<u8> {
        let mut f = vec![0u8; 0x200 + 512];
        let s = 0x200;
        f[s + 0x1F8..s + 0x1FC].copy_from_slice(&0xFD18_7E20u32.to_le_bytes());
        f[s + 0x1FC..s + 0x200].copy_from_slice(&0xFD18_7E20u32.to_le_bytes());
        f[s..s + 2].copy_from_slice(&64u16.to_le_bytes());
        f[s + 4..s + 8].copy_from_slice(&40000u32.to_le_bytes());
        f[s + 8..s + 10].copy_from_slice(&42u16.to_le_bytes());
        f[s + 0xDC..s + 0xDE].copy_from_slice(&7u16.to_le_bytes());
        f[s + 0x154..s + 0x158].copy_from_slice(&1200u32.to_le_bytes());
        f[s + 0x158..s + 0x15A].copy_from_slice(&300u16.to_le_bytes());
        f[s + 0x15C..s + 0x160].copy_from_slice(b"root");
        f[s + 0x168] = 1;
        f
    }

    #[test]
    fn parses() {
        let v = parse(&sysv()).unwrap();
        assert!(!v.big_endian);
        assert_eq!(v.inode_size, 64);
        assert_eq!(v.fsize, 40000);
        assert_eq!(v.nfree, 42);
        assert_eq!(v.ninode, 7);
        assert_eq!(v.free_blocks, 1200);
        assert_eq!(v.free_inodes, 300);
        assert_eq!(v.volume_name.as_deref(), Some("root"));
        assert_eq!(v.fs_type, 0xFD18_7E20);
        assert_eq!(v.clean, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&vec![0u8; 0x800]).is_none());
        let mut f = sysv();
        f[0x200 + 0x1F8] = 0xFF;
        assert!(parse(&f).is_none());
        // s_type mismatch on the SysV magic
        let mut g = sysv();
        g[0x200 + 0x1FC] = 0xAA;
        assert!(parse(&g).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&sysv()));
        assert!(!detect(b"sysv"));
    }
}

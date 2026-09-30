//! HFS (classic Mac OS filesystem) Master Directory Block scanner.
//!
//! The MDB occupies the third sector of the volume (byte offset `0x400`)
//! and starts with the big-endian signature `0x4244` (`"BD"`). All fields
//! are big-endian. Key fields:
//!
//! `drSigWord u16` = `0x4244` \@0, `drCrDate u32` \@2, `drLsMod u32` \@6,
//! `drAtrb u16` \@10, `drNmFls u16` \@12, `drVBMSt u16` \@14,
//! `drAllocPtr u16` \@16, `drNmAlBlks u16` \@18, `drAlBlkSiz u32` \@20,
//! `drClpSiz u32` \@24, `drAlBlSt u16` \@28, `drNxtCNID u32` \@30,
//! `drFreeBks u16` \@34, `drVN[28]` \@36 (Pascal volume name),
//! `drNmRtDirs u16` \@82, `drFilCnt u32` \@84, `drDirCnt u32` \@88,
//! `drXTFlSize u32` \@130, `drCTFlSize u32` \@146.
//!
//! Total MDB size: 162 bytes.
//!
//! ```
//! let mut f = vec![0u8; 0x400 + 162];
//! let s = 0x400;
//! f[s] = 0x42;
//! f[s + 1] = 0x44;
//! f[s + 20] = 0; f[s + 21] = 0; f[s + 22] = 0x20; f[s + 23] = 0; // drAlBlkSiz = 0x2000
//! f[s + 36] = 4; // drVN: length 4 + "Test"
//! f[s + 37..s + 41].copy_from_slice(b"Test");
//! f[s + 82] = 0; f[s + 83] = 0x02; // drNmRtDirs = 2
//! let h = izanagi_kit::hfs::parse(&f).unwrap();
//! assert_eq!(h.volume_name.as_deref(), Some("Test"));
//! assert_eq!(h.alloc_block_size, 0x2000);
//! ```
//!
//! Reference: Apple Technical Note TN1150 (HFS Plus Volume Format; the
//! classic HFS MDB is documented in Inside Macintosh: Files).

/// Parsed HFS Master Directory Block.
#[derive(Debug, Clone, PartialEq)]
pub struct Hfs {
    /// Number of files on the volume (`drNmFls`).
    pub num_files: u16,
    /// Total allocation blocks (`drNmAlBlks`).
    pub num_alloc_blocks: u16,
    /// Allocation block size in bytes (`drAlBlkSiz`).
    pub alloc_block_size: u32,
    /// Clump size in bytes (`drClpSiz`).
    pub clump_size: u32,
    /// First allocation block start (`drAlBlSt`).
    pub alloc_start: u16,
    /// Next unused catalog node ID (`drNxtCNID`).
    pub next_cnid: u32,
    /// Number of free allocation blocks (`drFreeBks`).
    pub free_blocks: u16,
    /// Volume name from `drVN` (Pascal string, up to 27 bytes).
    pub volume_name: Option<String>,
    /// Number of directories in the root (`drNmRtDirs`).
    pub root_dirs: u16,
    /// Total file count (`drFilCnt`).
    pub file_count: u32,
    /// Total directory count (`drDirCnt`).
    pub dir_count: u32,
    /// Catalog file size (`drCTFlSize`).
    pub catalog_size: u32,
    /// Extents overflow file size (`drXTFlSize`).
    pub extents_size: u32,
}

fn u16be(d: &[u8], off: usize) -> u16 {
    ((d[off] as u16) << 8) | d[off + 1] as u16
}

fn u32be(d: &[u8], off: usize) -> u32 {
    ((d[off] as u32) << 24)
        | ((d[off + 1] as u32) << 16)
        | ((d[off + 2] as u32) << 8)
        | d[off + 3] as u32
}

/// Parse a volume image; `None` if the MDB signature is absent.
pub fn parse(d: &[u8]) -> Option<Hfs> {
    const SB: usize = 0x400;
    if d.len() < SB + 162 {
        return None;
    }
    if u16be(d, SB) != 0x4244 {
        return None;
    }
    let name_len = d[SB + 36] as usize;
    let volume_name = if name_len > 0 && name_len <= 27 {
        core::str::from_utf8(&d[SB + 37..SB + 37 + name_len])
            .ok()
            .map(|s| s.to_string())
    } else {
        None
    };
    Some(Hfs {
        num_files: u16be(d, SB + 12),
        num_alloc_blocks: u16be(d, SB + 18),
        alloc_block_size: u32be(d, SB + 20),
        clump_size: u32be(d, SB + 24),
        alloc_start: u16be(d, SB + 28),
        next_cnid: u32be(d, SB + 30),
        free_blocks: u16be(d, SB + 34),
        volume_name,
        root_dirs: u16be(d, SB + 82),
        file_count: u32be(d, SB + 84),
        dir_count: u32be(d, SB + 88),
        catalog_size: u32be(d, SB + 146),
        extents_size: u32be(d, SB + 130),
    })
}

/// `true` if the image looks like an HFS volume.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hfs() -> Vec<u8> {
        let mut f = vec![0u8; 0x400 + 162];
        let s = 0x400;
        f[s] = 0x42;
        f[s + 1] = 0x44;
        // drNmFls @12, drNmAlBlks @18
        f[s + 13] = 5;
        f[s + 18] = 0x01;
        // drAlBlkSiz @20 = 0x2000, drClpSiz @24 = 0x4000
        f[s + 22] = 0x20;
        f[s + 26] = 0x40;
        // drAlBlSt @28
        f[s + 29] = 0x10;
        // drNxtCNID @30 = 9, drFreeBks @34 = 7
        f[s + 33] = 9;
        f[s + 35] = 7;
        // drVN @36: len + bytes
        f[s + 36] = 4;
        f[s + 37..s + 41].copy_from_slice(b"Test");
        // drNmRtDirs @82 = 2, drFilCnt @84 = 9, drDirCnt @88 = 4
        f[s + 83] = 2;
        f[s + 87] = 9;
        f[s + 91] = 4;
        // drXTFlSize @130 = 0x6000, drCTFlSize @146 = 0x8000
        f[s + 132] = 0x60;
        f[s + 148] = 0x80;
        f
    }

    #[test]
    fn parses() {
        let h = parse(&hfs()).unwrap();
        assert_eq!(h.num_files, 5);
        assert_eq!(h.num_alloc_blocks, 0x0100);
        assert_eq!(h.alloc_block_size, 0x2000);
        assert_eq!(h.clump_size, 0x4000);
        assert_eq!(h.alloc_start, 0x10);
        assert_eq!(h.volume_name.as_deref(), Some("Test"));
        assert_eq!(h.next_cnid, 9);
        assert_eq!(h.free_blocks, 7);
        assert_eq!(h.root_dirs, 2);
        assert_eq!(h.file_count, 9);
        assert_eq!(h.dir_count, 4);
        assert_eq!(h.extents_size, 0x6000);
        assert_eq!(h.catalog_size, 0x8000);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&vec![0u8; 0x600]).is_none());
        let mut f = hfs();
        f[0x401] = 0;
        assert!(parse(&f).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&hfs()));
        assert!(!detect(b"BD"));
    }
}

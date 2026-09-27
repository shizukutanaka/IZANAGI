//! Microsoft PDB MSF 7.00 container parsing (Multi-Stream Format).
//!
//! 32-byte magic `Microsoft C/C++ MSF 7.00\r\n\x1ADS\0`, then
//! `page_size` u32LE (512/1024/2048/4096), `fpm_page` u32,
//! `page_count` u32, `dir_size` u32 (byte length of the stream
//! directory), `reserved` u32, `block_map_addr` u32 — the page whose
//! contents list the directory pages.
//!
//! ```
//! use izanagi_kit::msf;
//! let mut d = b"Microsoft C/C++ MSF 7.00\r\n\x1aDS\x00\x00\x00".to_vec();
//! d.extend_from_slice(&4096u32.to_le_bytes()); // page_size
//! d.extend_from_slice(&1u32.to_le_bytes()); // fpm
//! d.extend_from_slice(&4u32.to_le_bytes()); // page_count
//! d.extend_from_slice(&4u32.to_le_bytes()); // dir_size
//! d.extend_from_slice(&0u32.to_le_bytes()); // reserved
//! d.extend_from_slice(&3u32.to_le_bytes()); // block_map_addr
//! d.extend_from_slice(&[0u8; 4096 * 4 - 56]); // fill to page_count*page_size
//! let m = msf::parse(&d).unwrap();
//! assert_eq!(m.page_size, 4096);
//! ```

/// MSF 7.00 magic (32 bytes).
pub const MAGIC: &[u8; 32] = b"Microsoft C/C++ MSF 7\x2e00\r\n\x1aDS\x00\x00\x00";

/// A parsed MSF superblock.
#[derive(Clone, Debug, PartialEq)]
pub struct Msf {
    /// Page size in bytes (512/1024/2048/4096).
    pub page_size: u32,
    /// Free-page-map page index.
    pub fpm_page: u32,
    /// Total page count (file size / page_size).
    pub page_count: u32,
    /// Stream-directory byte size.
    pub dir_size: u32,
    /// Page containing the directory-page list.
    pub block_map_addr: u32,
    /// Number of pages the directory occupies.
    pub dir_pages: usize,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses an MSF 7.00 superblock: magic, legal page size, and
/// `page_count * page_size` must be ≤ input length.
pub fn parse(d: &[u8]) -> Option<Msf> {
    if d.len() < 56 {
        return None;
    }
    if d.get(..32)? != MAGIC {
        return None;
    }
    let page_size = u32le(d, 32)?;
    if !matches!(page_size, 512 | 1024 | 2048 | 4096) {
        return None;
    }
    let fpm_page = u32le(d, 36)?;
    let page_count = u32le(d, 40)?;
    let dir_size = u32le(d, 44)?;
    // reserved @48 skipped
    let block_map_addr = u32le(d, 52)?;
    let total = page_size as u64 * page_count as u64;
    if total > d.len() as u64 || page_count == 0 {
        return None;
    }
    if block_map_addr >= page_count {
        return None;
    }
    let dir_pages = (dir_size as usize).div_ceil(page_size as usize);
    Some(Msf {
        page_size,
        fpm_page,
        page_count,
        dir_size,
        block_map_addr,
        dir_pages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(page_size: u32, pages: u32) -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&page_size.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&pages.to_le_bytes());
        d.extend_from_slice(&100u32.to_le_bytes()); // dir_size
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes()); // block_map
        d.extend_from_slice(&std::vec![
            0u8;
            (page_size as usize) * (pages as usize) - 56
        ]);
        d
    }

    #[test]
    fn parses_superblock() {
        let m = parse(&fixture(512, 2)).unwrap();
        assert_eq!(m.page_size, 512);
        assert_eq!(m.page_count, 2);
        assert_eq!(m.dir_pages, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture(512, 2);
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture(512, 2);
        d[32..36].copy_from_slice(&999u32.to_le_bytes()); // bad page size
        assert!(parse(&d).is_none());
    }
}

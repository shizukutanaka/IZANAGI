//! Android `boot.img` header parsing (AOSP `boot_img_hdr` v0–v2).
//!
//! Little-endian header beginning `ANDROID!`: `kernel_size`/`kernel_addr`
//! @8/@12, `ramdisk_size`/`ramdisk_addr` @16/@20, `second_size`/
//! `second_addr` @24/@28, `tags_addr` @32, `page_size` @36, then either
//! `unused`(v0) or `header_version`(v1+) @40, `os_version` @44
//! (`(a<<25)|(b<<18)|(c<<11)|patch`), `name[16]` @48, `cmdline[512]` @64.
//!
//! ```
//! use izanagi_kit::bootimg;
//! let mut d = vec![0u8; 1096];
//! d[0..8].copy_from_slice(b"ANDROID!");
//! d[8..12].copy_from_slice(&4096u32.to_le_bytes()); // kernel_size
//! d[36..40].copy_from_slice(&2048u32.to_le_bytes()); // page_size
//! d[48..51].copy_from_slice(b"m9x");
//! let b = bootimg::parse(&d).unwrap();
//! assert_eq!(b.kernel_size, 4096);
//! assert_eq!(bootimg::os_version(b.os_version), (0, 0, 0, 0));
//! ```

use std::vec::Vec;

/// `ANDROID!` magic.
pub const MAGIC: &[u8; 8] = b"ANDROID!";

/// Header size through `extra_cmdline` (v0).
pub const HEADER_LEN: usize = 1096;

/// A parsed `boot.img` header.
#[derive(Clone, Debug, PartialEq)]
pub struct BootImg {
    /// Kernel image byte size.
    pub kernel_size: u32,
    /// Kernel load address.
    pub kernel_addr: u32,
    /// Ramdisk byte size (0 = none).
    pub ramdisk_size: u32,
    /// Ramdisk load address.
    pub ramdisk_addr: u32,
    /// Second-stage image size (0 = none).
    pub second_size: u32,
    /// Second-stage load address.
    pub second_addr: u32,
    /// Kernel tags address.
    pub tags_addr: u32,
    /// Flash page size (typically 2048).
    pub page_size: u32,
    /// Header version (v0 keeps the `unused` field, reported as is).
    pub header_version: u32,
    /// Packed `os_version` (see [`os_version`]).
    pub os_version: u32,
    /// Board/product name (NUL-trimmed, 16B).
    pub name: Vec<u8>,
    /// Kernel command line (NUL-trimmed, 512B).
    pub cmdline: Vec<u8>,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn trim0(d: &[u8]) -> Vec<u8> {
    let mut v = d.to_vec();
    while v.last() == Some(&0) {
        v.pop();
    }
    v
}

/// Parses a `boot.img` header (requires ≥ [`HEADER_LEN`] bytes).
pub fn parse(d: &[u8]) -> Option<BootImg> {
    if d.len() < HEADER_LEN {
        return None;
    }
    if d.get(0..8)? != MAGIC {
        return None;
    }
    Some(BootImg {
        kernel_size: u32le(d, 8)?,
        kernel_addr: u32le(d, 12)?,
        ramdisk_size: u32le(d, 16)?,
        ramdisk_addr: u32le(d, 20)?,
        second_size: u32le(d, 24)?,
        second_addr: u32le(d, 28)?,
        tags_addr: u32le(d, 32)?,
        page_size: u32le(d, 36)?,
        header_version: u32le(d, 40)?,
        os_version: u32le(d, 44)?,
        name: trim0(d.get(48..64)?),
        cmdline: trim0(d.get(64..576)?),
    })
}

/// Decodes packed `os_version` into `(major, minor, patch, patch_level)`.
pub fn os_version(v: u32) -> (u8, u8, u8, u16) {
    let a = (v >> 25) as u8;
    let b = ((v >> 18) & 0x7f) as u8;
    let c = ((v >> 11) & 0x7f) as u8;
    let p = (v & 0x7ff) as u16;
    (a, b, c, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header() {
        let mut d = vec![0u8; HEADER_LEN];
        d[0..8].copy_from_slice(MAGIC);
        d[8..12].copy_from_slice(&8192u32.to_le_bytes());
        d[16..20].copy_from_slice(&1024u32.to_le_bytes());
        d[36..40].copy_from_slice(&2048u32.to_le_bytes());
        // os_version: a=12, b=0, c=0, patch=34
        d[44..48].copy_from_slice(&(((12u32) << 25) | 34).to_le_bytes());
        d[48..57].copy_from_slice(b"angler\0\0\0");
        d[64..76].copy_from_slice(b"console=tty0");
        let b = parse(&d).unwrap();
        assert_eq!(b.kernel_size, 8192);
        assert_eq!(b.ramdisk_size, 1024);
        assert_eq!(b.page_size, 2048);
        assert_eq!(b.name, b"angler".to_vec());
        assert_eq!(b.cmdline, b"console=tty0".to_vec());
        assert_eq!(os_version(b.os_version), (12, 0, 0, 34));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 100]).is_none());
        let mut d = vec![0u8; HEADER_LEN];
        d[0..8].copy_from_slice(b"ANDROID?");
        assert!(parse(&d).is_none());
    }
}

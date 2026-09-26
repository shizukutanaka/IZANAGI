//! Atari 8-bit ATR disk image.
//!
//! `u16 0x0296 | paragraphs u16 | sector_size u16 | paragraphs_hi u16
//! | flags u8 | reserved[7]` then the sector data. `paragraphs` counts
//! the data area in 16-byte units, so the file is always
//! `16 + paragraphs * 16` bytes. Sectors are `sector_size` bytes
//! (128 single density, 256 double density) — but the three boot
//! sectors are always stored as 128 bytes each, which is why the
//! sector offset formula below splits at index 3.
//!
//! ```
//! use izanagi_kit::atr::{parse, SECTOR_SD};
//!
//! // 720-sector single-density image (90 KiB data area).
//! let pars = (720 * SECTOR_SD / 16) as u16;
//! let mut d = vec![0u8; 16 + usize::from(pars) * 16];
//! d[0] = 0x96; d[1] = 0x02;
//! d[2..4].copy_from_slice(&pars.to_le_bytes());
//! d[4..6].copy_from_slice(&(SECTOR_SD as u16).to_le_bytes());
//! let a = parse(&d).unwrap();
//! assert_eq!(a.sectors, 720);
//! assert_eq!(a.sector(&d, 3).unwrap().len(), SECTOR_SD);
//! assert_eq!(a.sector(&d, 719).unwrap().len(), SECTOR_SD);
//! assert!(a.sector(&d, 720).is_none());
//! ```

/// Bytes per sector, single density.
pub const SECTOR_SD: usize = 128;
/// Bytes per sector, double density.
pub const SECTOR_DD: usize = 256;
/// Header length.
pub const HEADER: usize = 16;

/// A parsed ATR image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atr {
    /// Total sector count.
    pub sectors: usize,
    /// Bytes per sector (128 or 256).
    pub sector_size: usize,
    /// Flags byte (bit 0 = read-only on some writers).
    pub flags: u8,
}

impl Atr {
    /// Slice of sector `i` (0-based). The first three sectors are
    /// stored as 128 bytes even in double-density images.
    pub fn sector<'a>(&self, d: &'a [u8], i: usize) -> Option<&'a [u8]> {
        if i >= self.sectors {
            return None;
        }
        let (at, len) = if self.sector_size == SECTOR_DD {
            if i < 3 {
                (HEADER + i * SECTOR_SD, SECTOR_SD)
            } else {
                (HEADER + 3 * SECTOR_SD + (i - 3) * SECTOR_DD, SECTOR_DD)
            }
        } else {
            (HEADER + i * SECTOR_SD, SECTOR_SD)
        };
        d.get(at..at + len)
    }
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

/// Parse an ATR image header, or `None` on bad magic, unsupported
/// sector size, or a size that does not match the paragraph count.
pub fn parse(d: &[u8]) -> Option<Atr> {
    if d.first() != Some(&0x96) || d.get(1) != Some(&0x02) {
        return None;
    }
    let pars = usize::from(u16le(d, 2)?) | (usize::from(u16le(d, 6)?) << 16);
    let sec_size = usize::from(u16le(d, 4)?);
    if sec_size != SECTOR_SD && sec_size != SECTOR_DD {
        return None;
    }
    if pars == 0 || d.len() != HEADER + pars * 16 {
        return None;
    }
    let data = pars * 16;
    let sectors = if sec_size == SECTOR_DD {
        if data < 3 * SECTOR_SD || (data - 3 * SECTOR_SD) % SECTOR_DD != 0 {
            return None;
        }
        3 + (data - 3 * SECTOR_SD) / SECTOR_DD
    } else {
        if data % SECTOR_SD != 0 {
            return None;
        }
        data / SECTOR_SD
    };
    if sectors == 0 {
        return None;
    }
    Some(Atr {
        sectors,
        sector_size: sec_size,
        flags: *d.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image(nsec: usize, sec: usize) -> Vec<u8> {
        let data = if sec == SECTOR_DD {
            3 * SECTOR_SD + (nsec - 3) * SECTOR_DD
        } else {
            nsec * SECTOR_SD
        };
        let mut d = vec![0u8; HEADER + data];
        d[0] = 0x96;
        d[1] = 0x02;
        d[2..4].copy_from_slice(&((data / 16) as u16).to_le_bytes());
        d[4..6].copy_from_slice(&(sec as u16).to_le_bytes());
        d
    }

    #[test]
    fn sd_90k() {
        let d = image(720, SECTOR_SD);
        let a = parse(&d).unwrap();
        assert_eq!(a.sectors, 720);
        assert_eq!(a.sector_size, SECTOR_SD);
        assert_eq!(a.flags, 0);
        assert_eq!(a.sector(&d, 0).unwrap().len(), SECTOR_SD);
        assert!(a.sector(&d, 720).is_none());
        assert!(parse(&[]).is_none());
    }

    #[test]
    fn dd_180k_boot_sectors_128() {
        // 720 sectors: 3x128 boot + 717x256.
        let d = image(720, SECTOR_DD);
        let a = parse(&d).unwrap();
        assert_eq!(a.sectors, 720);
        assert_eq!(a.sector_size, SECTOR_DD);
        assert_eq!(a.sector(&d, 2).unwrap().len(), SECTOR_SD);
        assert_eq!(a.sector(&d, 3).unwrap().len(), SECTOR_DD);
        assert_eq!(a.sector(&d, 719).unwrap().len(), SECTOR_DD);
    }

    #[test]
    fn dd_offsets() {
        let mut d = image(10, SECTOR_DD);
        let a = parse(&d).unwrap();
        let at = HEADER + 3 * SECTOR_SD;
        d[at] = 0xAB;
        assert_eq!(a.sector(&d, 3).unwrap()[0], 0xAB);
    }

    #[test]
    fn rejects() {
        let mut d = image(720, SECTOR_SD);
        d[0] = 0;
        assert_eq!(parse(&d), None);
        let mut d = image(720, SECTOR_SD);
        d[4] = 64; // sector size 64 unsupported
        assert_eq!(parse(&d), None);
        let mut d = image(720, SECTOR_SD);
        d.truncate(HEADER); // paragraph count says data exists
        assert_eq!(parse(&d), None);
        let mut d = image(720, SECTOR_SD);
        d[2] = 0; // pars = 0
        assert_eq!(parse(&d), None);
    }
}

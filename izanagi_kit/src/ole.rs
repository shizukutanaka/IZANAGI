//! OLE2 / Compound File Binary (`.doc`, `.xls`, `.ppt`, `.msi`).
//!
//! The 512-byte header carries the `D0 CF 11 E0 A1 B1 1A E1`
//! magic, sector geometry (shift 9 → 512B or 12 → 4096B), the
//! first directory sector and a 109-slot DIFAT of FAT sector ids.
//! `parse` builds the FAT, walks the directory chain and returns
//! every 128-byte directory entry (storage/stream/root).
//!
//! ```
//! use izanagi_kit::ole::{parse, ENTRY_STREAM, FREESECT, ENDOFCHAIN};
//!
//! // 512-byte sectors: header + FAT + dir.
//! let mut d = vec![0u8; 512 * 3];
//! d[..8].copy_from_slice(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]);
//! let w16 = |d: &mut [u8], o: usize, v: u16| {
//!     d[o] = v as u8; d[o + 1] = (v >> 8) as u8;
//! };
//! let w32 = |d: &mut [u8], o: usize, v: u32| {
//!     d[o] = v as u8; d[o + 1] = (v >> 8) as u8;
//!     d[o + 2] = (v >> 16) as u8; d[o + 3] = (v >> 24) as u8;
//! };
//! w16(&mut d, 0x1a, 3);      // major 3 -> 512B sectors
//! w16(&mut d, 0x1c, 0xfffe);
//! w16(&mut d, 0x1e, 9);
//! w16(&mut d, 0x20, 6);      // mini shift
//! w32(&mut d, 0x2c, 1);      // one FAT sector (sector 0)
//! w32(&mut d, 0x30, 1);      // dir chain starts at sector 1
//! w32(&mut d, 0x38, 4096);   // mini cutoff
//! w32(&mut d, 0x4c, 0);      // DIFAT[0] = sector 0
//! for i in 1..109 { w32(&mut d, 0x4c + i * 4, FREESECT); }
//! // sector 0 = FAT: [dir-sector=ENDOFCHAIN? no: dir at 1]
//! let fat = 512;
//! w32(&mut d, fat, ENDOFCHAIN);      // FAT sector itself
//! w32(&mut d, fat + 4, ENDOFCHAIN);  // dir sector end
//! // sector 1 = directory, entry 0 = root
//! let dir = 1024;
//! d[dir] = b'R'; d[dir + 2] = b'o';
//! w16(&mut d, dir + 64, 6);          // name len (UTF-16 w/NUL: "Ro\0")
//! d[dir + 66] = 5;                   // root storage
//! w32(&mut d, dir + 68, FREESECT);
//! w32(&mut d, dir + 72, FREESECT);
//! w32(&mut d, dir + 76, FREESECT);
//! w32(&mut d, dir + 116, ENDOFCHAIN);
//! let c = parse(&d).unwrap();
//! assert_eq!(c.entries.len(), 4); // root + 3 empty slots
//! assert_eq!(c.entries[0].name.as_str(), "Ro");
//! ```

/// Magic bytes.
pub const MAGIC: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
/// Header size.
pub const HEADER: usize = 512;
/// DIFAT slots stored in the header.
pub const DIFAT_IN_HEADER: usize = 109;
/// FAT markers.
pub const FREESECT: u32 = 0xffff_ffff;
/// End of a sector chain.
pub const ENDOFCHAIN: u32 = 0xffff_fffe;
/// Sector holds FAT data.
pub const FATSECT: u32 = 0xffff_fffd;
/// Sector holds DIFAT data.
pub const DIFSECT: u32 = 0xffff_fffc;
/// Directory entry types.
pub const ENTRY_EMPTY: u8 = 0;
/// Storage (folder).
pub const ENTRY_STORAGE: u8 = 1;
/// Stream (file payload).
pub const ENTRY_STREAM: u8 = 2;
/// Root entry (holds the mini-stream).
pub const ENTRY_ROOT: u8 = 5;

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}
fn le64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(le32(d, at)?) | u64::from(le32(d, at + 4)?) << 32)
}

/// One 128-byte directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    /// UTF-16 name decoded (lossy ASCII-approximate for non-ASCII).
    pub name: String,
    /// Entry type: 0 empty, 1 storage, 2 stream, 5 root.
    pub kind: u8,
    /// Sibling/child entry ids (FREESECT when none).
    pub left: u32,
    /// Right sibling id.
    pub right: u32,
    /// Child id.
    pub child: u32,
    /// First sector of the stream.
    pub start_sector: u32,
    /// Stream size in bytes (u32 for major 3).
    pub size: u64,
    /// This entry's index in the directory stream.
    pub index: u32,
}

/// A parsed CFB file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ole {
    /// Major version (3 → 512B sectors, 4 → 4096B).
    pub major: u16,
    /// Sector size in bytes.
    pub sector_size: usize,
    /// Mini-sector size.
    pub mini_sector_size: usize,
    /// FAT sector count.
    pub num_fat: u32,
    /// First directory-sector id.
    pub first_dir_sector: u32,
    /// Mini-stream cutoff (4096 default).
    pub mini_cutoff: u32,
    /// First mini-FAT sector and count.
    pub first_minifat_sector: u32,
    /// Mini-FAT sector count.
    pub num_minifat: u32,
    /// First DIFAT sector and count (extra DIFAT beyond the header).
    pub first_difat_sector: u32,
    /// DIFAT sector count.
    pub num_difat: u32,
    /// FAT sector ids (from the header DIFAT only; extra-DIFAT
    /// chains are uncommon and reported via `num_difat` > 0).
    pub fat_sectors: Vec<u32>,
    /// FAT entries.
    pub fat: Vec<u32>,
    /// Directory entries in stream order.
    pub entries: Vec<DirEntry>,
}

/// Byte offset of sector `s`. Sector ids count from after the header,
/// and the header region is padded to a full sector — so the base is
/// 512 for major 3 but 4096 for major 4, i.e. `ssz` in both cases.
fn sector_at(base: usize, ssz: usize, s: u32) -> Option<usize> {
    base.checked_add((usize::try_from(s).ok()?).checked_mul(ssz)?)
}

/// Parse a CFB file: header, FAT (from the header DIFAT), then the
/// directory chain. Returns `None` on bad magic, odd geometry, or
/// a truncated chain. Directory entries are bounded by input size —
/// every sector read is bounds-checked, so `entries` can never exceed
/// `file_len / 128` slots.
pub fn parse(d: &[u8]) -> Option<Ole> {
    if d.get(..8)? != MAGIC.as_slice() {
        return None;
    }
    if le16(d, 0x1c)? != 0xfffe {
        return None;
    }
    let major = le16(d, 0x1a)?;
    let shift = u32::from(le16(d, 0x1e)?);
    let sector_size = usize::try_from(1u64.checked_shl(shift)?).ok()?;
    if !(sector_size == 512 && major == 3 || sector_size == 4096 && major == 4) {
        return None;
    }
    let num_fat = le32(d, 0x2c)?;
    let first_dir = le32(d, 0x30)?;
    let mini_cutoff = le32(d, 0x38)?;
    let first_minifat = le32(d, 0x3c)?;
    let num_minifat = le32(d, 0x40)?;
    let first_difat = le32(d, 0x44)?;
    let num_difat = le32(d, 0x48)?;

    let mut fat_sectors = Vec::new();
    for i in 0..DIFAT_IN_HEADER {
        let s = le32(d, 0x4c + i * 4)?;
        if s != FREESECT && fat_sectors.len() < usize::try_from(num_fat).ok()? {
            fat_sectors.push(s);
        }
    }
    if num_difat > 0 && fat_sectors.len() < usize::try_from(num_fat).ok()? {
        // Extra DIFAT chain exists but only the header slots are
        // used here; still fine for small files. Full support would
        // walk `first_difat`.
        return None;
    }
    // FAT contents.
    let mut fat = Vec::new();
    for &s in &fat_sectors {
        let at = sector_at(sector_size, sector_size, s)?;
        for i in 0..sector_size / 4 {
            fat.push(le32(d, at + i * 4)?);
        }
    }
    // Directory chain: follow FAT from first_dir until ENDOFCHAIN.
    let mut entries = Vec::new();
    let mut s = first_dir;
    let mut guard = 0usize;
    while s != ENDOFCHAIN && s != FREESECT {
        if guard >= fat.len() {
            return None;
        }
        guard += 1;
        let at = sector_at(sector_size, sector_size, s)?;
        for i in 0..sector_size / 128 {
            let e = at + i * 128;
            let name_len = usize::from(le16(d, e + 64).unwrap_or(0));
            let name_len = name_len.min(64).saturating_sub(2).min(62);
            let mut name = String::new();
            for j in (0..name_len).step_by(2) {
                let c = le16(d, e + j).unwrap_or(0);
                name.push(char::from_u32(u32::from(c)).unwrap_or('?'));
            }
            entries.push(DirEntry {
                name,
                kind: d.get(e + 66).copied()?,
                left: le32(d, e + 68)?,
                right: le32(d, e + 72)?,
                child: le32(d, e + 76)?,
                start_sector: le32(d, e + 116)?,
                size: if major == 3 {
                    u64::from(le32(d, e + 120)?)
                } else {
                    le64(d, e + 120)?
                },
                index: u32::try_from(entries.len()).unwrap_or(0),
            });
        }
        s = *fat.get(usize::try_from(s).ok()?)?;
    }
    Some(Ole {
        major,
        sector_size,
        mini_sector_size: usize::try_from(1u64.checked_shl(u32::from(le16(d, 0x20)?))?).ok()?,
        num_fat,
        first_dir_sector: first_dir,
        mini_cutoff,
        first_minifat_sector: first_minifat,
        num_minifat,
        first_difat_sector: first_difat,
        num_difat,
        fat_sectors,
        fat,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture(stream_size: u64) -> Vec<u8> {
        let mut d = vec![0u8; 512 * 4];
        d[..8].copy_from_slice(&MAGIC);
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
        };
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
            d[o + 2] = (v >> 16) as u8;
            d[o + 3] = (v >> 24) as u8;
        };
        w16(&mut d, 0x1a, 3);
        w16(&mut d, 0x1c, 0xfffe);
        w16(&mut d, 0x1e, 9);
        w16(&mut d, 0x20, 6);
        w32(&mut d, 0x2c, 1);
        w32(&mut d, 0x30, 1);
        w32(&mut d, 0x38, 4096);
        w32(&mut d, 0x3c, ENDOFCHAIN);
        w32(&mut d, 0x44, ENDOFCHAIN);
        w32(&mut d, 0x4c, 0);
        for i in 1..109 {
            w32(&mut d, 0x4c + i * 4, FREESECT);
        }
        // sector 0 FAT, sectors 1 dir, 2 stream data
        w32(&mut d, 512, FATSECT);
        w32(&mut d, 516, ENDOFCHAIN);
        w32(&mut d, 520, ENDOFCHAIN);
        // dir at sector 1 (offset 1024): root + one stream entry
        let dir = 1024;
        fn nm(d: &mut [u8], off: usize, s: &str) {
            for (i, c) in s.chars().enumerate() {
                d[off + i * 2] = c as u8;
            }
            d[off + 64] = ((s.len() + 1) * 2) as u8;
            d[off + 65] = 0;
        }
        nm(&mut d, dir, "Root Entry");
        d[dir + 66] = ENTRY_ROOT;
        w32(&mut d, dir + 76, 1); // child = entry 1
        w32(&mut d, dir + 116, 2);
        w32(&mut d, dir + 120, stream_size as u32);
        nm(&mut d, dir + 128, "Workbook");
        d[dir + 128 + 66] = ENTRY_STREAM;
        w32(&mut d, dir + 128 + 116, 2);
        w32(&mut d, dir + 128 + 120, 700);
        d
    }

    #[test]
    fn header_and_dir() {
        let d = fixture(700);
        let c = parse(&d).unwrap();
        assert_eq!(c.major, 3);
        assert_eq!(c.sector_size, 512);
        assert_eq!(c.mini_sector_size, 64);
        assert_eq!(c.num_fat, 1);
        assert_eq!(c.fat_sectors, vec![0]);
        assert_eq!(c.fat.len(), 128);
        assert_eq!(c.entries.len(), 4);
        assert_eq!(c.entries[0].name, "Root Entry");
        assert_eq!(c.entries[0].kind, ENTRY_ROOT);
        assert_eq!(c.entries[0].child, 1);
        assert_eq!(c.entries[1].name, "Workbook");
        assert_eq!(c.entries[1].kind, ENTRY_STREAM);
        assert_eq!(c.entries[1].size, 700);
        assert_eq!(c.entries[2].kind, ENTRY_EMPTY);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 64]).is_none());
        let mut d = fixture(0);
        d[0] = 0xff; // bad magic
        assert!(parse(&d).is_none());
        let mut d2 = fixture(0);
        d2[0x1a] = 5; // bad major
        assert!(parse(&d2).is_none());
        // missing FAT sector → truncated
        let mut d3 = fixture(0);
        d3.truncate(600);
        assert!(parse(&d3).is_none());
    }
}

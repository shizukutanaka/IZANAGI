//! ISO 9660 (ECMA-119) CD image surface: the Primary Volume
//! Descriptor lives at sector 16, and multi-byte numerics are
//! stored *both endian* (little then big — the "733" form), so
//! the whole layer is byte-order agnostic by construction.
//! Directory records give you extents and names; [`entries`]
//! walks a directory's blocks.
//!
//! ```
//! use izanagi_kit::iso9660 as iso;
//! let mut img = vec![0u8; 2048 * 17];
//! let pvd = 2048 * 16;
//! img[pvd] = 1; // PVD
//! img[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
//! img[pvd + 6] = 1; // version
//! img[pvd + 8..pvd + 13].copy_from_slice(b"SYSID");
//! img[pvd + 40..pvd + 46].copy_from_slice(b"VOL ID");
//! // vol space size: 17 sectors, both-endian
//! img[pvd + 80..pvd + 84].copy_from_slice(&[17, 0, 0, 0]);
//! img[pvd + 84..pvd + 88].copy_from_slice(&[0, 0, 0, 17]);
//! // logical block size 2048 both-endian u16
//! img[pvd + 128..pvd + 130].copy_from_slice(&[0, 8]);
//! img[pvd + 130..pvd + 132].copy_from_slice(&[8, 0]);
//! // root dir record (34B) at pvd+156: extent 20, size 2048, dir flag
//! let mut rr = [0u8; 34];
//! rr[0] = 34;
//! rr[2..6].copy_from_slice(&20u32.to_le_bytes());
//! rr[6..10].copy_from_slice(&[0, 0, 0, 20]);
//! rr[10..14].copy_from_slice(&2048u32.to_le_bytes());
//! rr[14..18].copy_from_slice(&[0, 0, 8, 0]);
//! rr[25] = 2;
//! rr[32] = 1;
//! img[pvd + 156..pvd + 190].copy_from_slice(&rr);
//! let isoimg = iso::parse(&img).unwrap();
//! assert_eq!(isoimg.volume_id(), "VOL ID");
//! assert_eq!(isoimg.logical_block_size, 2048);
//! ```

use std::vec::Vec;

/// Sector size the spec fixes at 2048.
pub const SECTOR: usize = 2048;
/// Sector number of the Primary Volume Descriptor.
pub const PVD_SECTOR: usize = 16;

fn le16(d: &[u8], at: usize) -> Option<u32> {
    Some(*d.get(at)? as u32 | (*d.get(at + 1)? as u32) << 8)
}
fn be16(d: &[u8], at: usize) -> Option<u32> {
    Some((*d.get(at)? as u32) << 8 | *d.get(at + 1)? as u32)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        *d.get(at)? as u32
            | (*d.get(at + 1)? as u32) << 8
            | (*d.get(at + 2)? as u32) << 16
            | (*d.get(at + 3)? as u32) << 24,
    )
}
fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (*d.get(at)? as u32) << 24
            | (*d.get(at + 1)? as u32) << 16
            | (*d.get(at + 2)? as u32) << 8
            | *d.get(at + 3)? as u32,
    )
}

/// ISO "both-endian" u16: LE word then BE word; they must agree.
pub fn both16(d: &[u8], at: usize) -> Option<u32> {
    let a = le16(d, at)?;
    let b = be16(d, at + 2)?;
    if a == b {
        Some(a)
    } else {
        None
    }
}

/// ISO "both-endian" u32 ("733").
pub fn both32(d: &[u8], at: usize) -> Option<u32> {
    let a = le32(d, at)?;
    let b = be32(d, at + 4)?;
    if a == b {
        Some(a)
    } else {
        None
    }
}

/// One directory record (file or directory entry).
#[derive(Clone, Debug)]
pub struct DirRec {
    /// Record's own byte length.
    pub rec_len: usize,
    /// Extent (starting sector, 2048-byte units).
    pub extent: u32,
    /// Data length in bytes.
    pub size: u32,
    /// Recording date, 7 raw bytes (yy, mm, dd, hh, mm, ss, tz/15min).
    pub date: [u8; 7],
    /// Flag bits: bit1 = directory, bit2 = associated, bit4 = record…
    pub flags: u8,
    /// File unit size (interleave).
    pub unit_size: u8,
    /// Interleave gap size.
    pub gap: u8,
    /// Volume sequence number.
    pub vol_seq: u32,
    /// Name bytes (`[0]` = `.`, `[1]` = `..`; d1-encoded otherwise).
    pub name: Vec<u8>,
}

impl DirRec {
    /// `true` when the entry is a directory (flag bit 1).
    pub fn is_dir(&self) -> bool {
        self.flags & 0x02 != 0
    }
}

/// Parses one directory record at `at`. Returns `(rec, end)` —
/// `end = at + rec_len` (the record length already includes its
/// padding). `None` when malformed or `d[at] == 0` (record-less
/// padding: callers stop at it).
pub fn dir_rec(d: &[u8], at: usize) -> Option<(DirRec, usize)> {
    let len = *d.get(at)? as usize;
    if len < 33 || at.checked_add(len)? > d.len() {
        return None;
    }
    let r = DirRec {
        rec_len: len,
        extent: le32(d, at + 2)?,
        size: le32(d, at + 10)?,
        date: [
            *d.get(at + 18)?,
            *d.get(at + 19)?,
            *d.get(at + 20)?,
            *d.get(at + 21)?,
            *d.get(at + 22)?,
            *d.get(at + 23)?,
            *d.get(at + 24)?,
        ],
        flags: *d.get(at + 25)?,
        unit_size: *d.get(at + 26)?,
        gap: *d.get(at + 27)?,
        vol_seq: le16(d, at + 28)?,
        name: d
            .get(at + 33..at + 33 + *d.get(at + 32)? as usize)?
            .to_vec(),
    };
    if r.name.is_empty() {
        return None; // name len 0 invalid
    }
    Some((r, at + len))
}

/// Parsed image surface — just the PVD; directories are walked
/// lazily.
#[derive(Clone, Debug)]
pub struct Iso {
    /// `CD001` descriptor type seen (1 = primary).
    pub ty: u8,
    /// System identifier, trimmed.
    pub system_id: std::string::String,
    /// Volume identifier, trimmed.
    pub volume_id: std::string::String,
    /// Volume size in logical blocks.
    pub vol_space_size: u32,
    /// Bytes per logical block (normally 2048).
    pub logical_block_size: u32,
    /// Path-table bytes.
    pub path_table_size: u32,
    /// Root directory record.
    pub root: DirRec,
}

fn trim_field(b: &[u8]) -> std::string::String {
    let s: std::string::String = b
        .iter()
        .map(|&c| {
            if c.is_ascii_graphic() || c == b' ' {
                c as char
            } else {
                ' '
            }
        })
        .collect();
    s.trim().to_string()
}

impl Iso {
    /// Volume identifier as a trimmed string.
    pub fn volume_id(&self) -> &str {
        &self.volume_id
    }
    /// System identifier as a trimmed string.
    pub fn system_id(&self) -> &str {
        &self.system_id
    }
}

/// Parses the PVD at sector 16 (type 1, `CD001`, version 1) and
/// the root directory record inside it.
pub fn parse(d: &[u8]) -> Option<Iso> {
    let pvd = PVD_SECTOR.checked_mul(SECTOR)?;
    if *d.get(pvd)? != 1 {
        return None; // not the primary descriptor
    }
    if d.get(pvd + 1..pvd + 6)? != b"CD001" {
        return None;
    }
    if *d.get(pvd + 6)? != 1 {
        return None;
    }
    let (root, _) = dir_rec(d, pvd + 156)?;
    Some(Iso {
        ty: 1,
        system_id: trim_field(d.get(pvd + 8..pvd + 40)?),
        volume_id: trim_field(d.get(pvd + 40..pvd + 72)?),
        vol_space_size: both32(d, pvd + 80)?,
        logical_block_size: both16(d, pvd + 128)?,
        path_table_size: both32(d, pvd + 132)?,
        root,
    })
}

/// Iterates the directory records of `dir`'s extent — returns the
/// entries in on-disc order (`.`, `..`, then files).
/// Stops at the first zero-length record byte per block (padding).
pub fn entries(d: &[u8], dir: &DirRec) -> Option<Vec<DirRec>> {
    let mut out = Vec::new();
    let mut at = dir.extent as usize * SECTOR;
    let end = at.checked_add(dir.size as usize)?;
    while at < end && at < d.len() {
        // zero length byte = rest of this sector is padding
        if *d.get(at)? == 0 {
            // skip to next sector boundary
            at = (at / SECTOR + 1) * SECTOR;
            continue;
        }
        let (r, next) = dir_rec(d, at)?;
        out.push(r);
        at = next;
    }
    Some(out)
}

/// Finds a direct child of `dir` by name — ISO names are
/// uppercase-d1 like `FOO.TXT;1`; the match compares the
/// name-part before `;`.
pub fn find(d: &[u8], dir: &DirRec, name: &str) -> Option<DirRec> {
    let want = name.to_ascii_uppercase();
    for e in entries(d, dir)? {
        let base: std::string::String = e
            .name
            .iter()
            .take_while(|&&c| c != b';')
            .map(|&c| c as char)
            .collect();
        if base == want || base.trim_end_matches('.') == want {
            return Some(e);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be16(v: u16) -> [u8; 2] {
        [(v >> 8) as u8, v as u8]
    }
    fn be32(v: u32) -> [u8; 4] {
        [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]
    }

    fn dir_bytes(files: &[(&str, u32, u32)]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, extent, size) in files {
            let name_b = name.as_bytes();
            let len = 33 + name_b.len() + (name_b.len() & 1 ^ 1); // pad to even
            let mut r = vec![0u8; len];
            r[0] = len as u8;
            r[2..6].copy_from_slice(&extent.to_le_bytes());
            r[6..10].copy_from_slice(&be32(*extent));
            r[10..14].copy_from_slice(&size.to_le_bytes());
            r[14..18].copy_from_slice(&be32(*size));
            r[18..25].copy_from_slice(&[125, 9, 1, 0, 0, 0, 0]);
            r[25] = if name.ends_with('/') { 2 } else { 0 };
            r[32] = name_b.len() as u8;
            r[33..33 + name_b.len()].copy_from_slice(name_b);
            out.extend_from_slice(&r);
        }
        out
    }

    fn image() -> Vec<u8> {
        let mut img = vec![0u8; 2048 * 30];
        let pvd = 2048 * 16;
        img[pvd] = 1;
        img[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        img[pvd + 6] = 1;
        img[pvd + 8..pvd + 15].copy_from_slice(b"SYSTEM ");
        img[pvd + 40..pvd + 48].copy_from_slice(b"MYDISC  ");
        img[pvd + 80..pvd + 84].copy_from_slice(&30u32.to_le_bytes());
        img[pvd + 84..pvd + 88].copy_from_slice(&be32(30));
        img[pvd + 120..pvd + 122].copy_from_slice(&1u16.to_le_bytes());
        img[pvd + 122..pvd + 124].copy_from_slice(&be16(1));
        img[pvd + 124..pvd + 126].copy_from_slice(&1u16.to_le_bytes());
        img[pvd + 126..pvd + 128].copy_from_slice(&be16(1));
        img[pvd + 128..pvd + 130].copy_from_slice(&2048u16.to_le_bytes());
        img[pvd + 130..pvd + 132].copy_from_slice(&be16(2048));
        img[pvd + 132..pvd + 136].copy_from_slice(&10u32.to_le_bytes());
        img[pvd + 136..pvd + 140].copy_from_slice(&be32(10));
        // root record @ pvd+156: name len 1 byte 0, extent 20, size 2048
        let mut rr = vec![0u8; 34];
        rr[0] = 34;
        rr[2..6].copy_from_slice(&20u32.to_le_bytes());
        rr[6..10].copy_from_slice(&be32(20));
        rr[10..14].copy_from_slice(&2048u32.to_le_bytes());
        rr[14..18].copy_from_slice(&be32(2048));
        rr[18..25].copy_from_slice(&[125, 9, 1, 0, 0, 0, 0]);
        rr[25] = 2;
        rr[32] = 1;
        rr[33] = 0;
        img[pvd + 156..pvd + 190].copy_from_slice(&rr);
        // root dir contents at sector 20
        let contents = dir_bytes(&[
            ("\x00", 20, 2048),
            ("\x01", 20, 2048),
            ("FOO.TXT;1", 21, 12),
        ]);
        img[20 * 2048..20 * 2048 + contents.len()].copy_from_slice(&contents);
        img
    }

    #[test]
    fn parses_pvd_and_walks_root() {
        let img = image();
        let i = parse(&img).unwrap();
        assert_eq!(i.volume_id(), "MYDISC");
        assert_eq!(i.system_id(), "SYSTEM");
        assert_eq!(i.vol_space_size, 30);
        assert_eq!(i.logical_block_size, 2048);
        assert!(i.root.is_dir());
        let es = entries(&img, &i.root).unwrap();
        assert_eq!(es.len(), 3);
        let f = find(&img, &i.root, "FOO.TXT").unwrap();
        assert_eq!(f.size, 12);
        assert!(find(&img, &i.root, "NOPE").is_none());
    }

    #[test]
    fn both_endian_agreement_enforced() {
        let mut img = image();
        img[16 * 2048 + 80] ^= 0xFF; // LE side corrupt → mismatch
        assert!(parse(&img).is_none());
    }

    #[test]
    fn both_endian_helpers_and_dir_rec_direct() {
        // both16/both32 reject disagreement
        let b = [8, 0, 0, 8, 0, 0, 0, 0];
        assert_eq!(both16(&b, 0), Some(8));
        assert_eq!(both16(&b, 4), Some(0));
        let m = [1, 0, 0, 0, 0, 0, 0, 1];
        assert_eq!(both32(&m, 0), Some(1));
        let bad = [1, 0, 0, 0, 0, 0, 0, 2];
        assert_eq!(both32(&bad, 0), None);
        // dir_rec on a lone record
        let mut rr = [0u8; 34];
        rr[0] = 34;
        rr[2..6].copy_from_slice(&7u32.to_le_bytes());
        rr[6..10].copy_from_slice(&be32(7));
        rr[10..14].copy_from_slice(&5u32.to_le_bytes());
        rr[14..18].copy_from_slice(&be32(5));
        rr[25] = 2;
        rr[32] = 1;
        let (r, end) = dir_rec(&rr, 0).unwrap();
        assert_eq!(end, 34);
        assert_eq!(r.extent, 7);
        assert_eq!(r.size, 5);
        assert!(r.is_dir());
        assert!(dir_rec(&rr[..33], 0).is_none()); // len says 34, data short
        assert!(dir_rec(&[0; 34], 0).is_none()); // len byte 0
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        let mut img = image();
        img[16 * 2048] = 2; // not primary
        assert!(parse(&img).is_none());
        let mut img2 = image();
        img2[16 * 2048 + 2] = b'X'; // bad magic
        assert!(parse(&img2).is_none());
    }
}

//! FAT12/16/32 filesystem reading (Microsoft FAT spec).
//!
//! Parses the BPB, computes the region map, walks FAT cluster chains
//! and 32-byte directory entries (8.3 short names; LFN entries are
//! skipped). Type is decided by cluster count per spec: <4085 FAT12,
//! <65525 FAT16, else FAT32.
//!
//! ```
//! let mut img = vec![0u8; 512 * 20];
//! // BPB: 512 B/sector, 1 sector/cluster, 1 reserved, 1 FAT of 1
//! img[11] = 0; img[12] = 2; // bps = 512
//! img[13] = 1; // spc
//! img[14] = 1; img[15] = 0; // reserved = 1
//! img[16] = 1; // nfats
//! img[17] = 16; img[18] = 0; // root ents = 16 → 1 sector
//! img[19] = 20; img[20] = 0; // total16 = 20
//! img[22] = 1; img[23] = 0; // fatsz16 = 1
//! // FAT12: bytes 0..2 = media+reserved entries, entry 2 = EOC
//! img[512] = 0xF0; img[512 + 1] = 0xFF; img[512 + 2] = 0xFF;
//! img[512 + 3] = 0xFF; img[512 + 4] = 0x0F; // entry 2 → 0xFFF (EOC)
//! // dir entry at root region = reserved(1) + fat(1) = sector 2
//! let dir = 512 * 2;
//! img[dir..dir + 8].copy_from_slice(b"HELLO   ");
//! img[dir + 8..dir + 11].copy_from_slice(b"TXT");
//! img[dir + 11] = 0x20; // archive
//! img[dir + 26] = 2; img[dir + 27] = 0; // clus = 2
//! img[dir + 28..dir + 32].copy_from_slice(&3u32.to_le_bytes());
//! img[512 * 3] = b'a'; img[512 * 3 + 1] = b'b'; img[512 * 3 + 2] = b'c';
//! let f = izanagi_kit::fat::parse(&img).unwrap();
//! let es = izanagi_kit::fat::entries(&img, &f, None).unwrap();
//! assert_eq!(izanagi_kit::fat::entry_name(&es[0]), "HELLO   .TXT");
//! assert_eq!(izanagi_kit::fat::file_bytes(&img, &f, &es[0]).unwrap(), b"abc");
//! ```

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | ((*d.get(at + 1)? as u16) << 8))
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (*d.get(at)? as u32)
            | ((*d.get(at + 1)? as u32) << 8)
            | ((*d.get(at + 2)? as u32) << 16)
            | ((*d.get(at + 3)? as u32) << 24),
    )
}

/// Filesystem variant decided by cluster count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    /// FAT12 — 12-bit entries, packed two per 3 bytes.
    Fat12,
    /// FAT16.
    Fat16,
    /// FAT32 — root directory is a normal cluster chain.
    Fat32,
}

/// A 32-byte 8.3 directory entry.
#[derive(Debug, Clone)]
pub struct DirEnt {
    /// Raw 8-byte base name (space-padded).
    pub name: [u8; 8],
    /// Raw 3-byte extension.
    pub ext: [u8; 3],
    /// Attribute byte (0x0F = LFN, 0x10 = dir, 0x08 = volume label).
    pub attr: u8,
    /// First cluster.
    pub clus: u32,
    /// File size in bytes (0 for directories).
    pub size: u32,
    /// Byte offset of this entry inside the image.
    pub at: usize,
}

/// Parsed volume geometry.
#[derive(Debug)]
pub struct Fat {
    /// FAT variant.
    pub ty: Ty,
    /// Bytes per sector.
    pub bps: usize,
    /// Sectors per cluster.
    pub spc: usize,
    /// Sector where the first FAT begins.
    pub fat_at: usize,
    /// FAT length in sectors (each of `nfats` copies — first is used).
    pub fatsz: usize,
    /// Sector where the root directory region begins (FAT12/16 only).
    pub root_at: usize,
    /// Number of root directory sectors (FAT12/16 only; 0 on FAT32).
    pub root_secs: usize,
    /// First data sector (cluster 2's sector).
    pub data_at: usize,
    /// Total clusters in the data region.
    pub clusters: u32,
    /// FAT32 root's first cluster (meaningful only on `Ty::Fat32`).
    pub root_clus: u32,
}

/// Sector of cluster `n` (clusters are numbered from 2).
pub fn cluster_at(f: &Fat, n: u32) -> usize {
    f.data_at + ((n - 2) as usize) * f.spc
}

/// Parses the BPB and computes the region map.
/// `None` on truncation, zero bps/spc, or a zero total sector count.
pub fn parse(d: &[u8]) -> Option<Fat> {
    let bps = le16(d, 11)? as usize;
    let spc = *d.get(13)? as usize;
    let reserved = le16(d, 14)? as usize;
    let nfats = *d.get(16)? as usize;
    let root_ents = le16(d, 17)? as usize;
    let tot16 = le16(d, 19)? as usize;
    let fatsz16 = le16(d, 22)? as usize;
    let tot32 = le32(d, 32)? as usize;
    let fatsz32 = le32(d, 36)? as usize;

    if bps == 0 || spc == 0 || nfats == 0 || reserved == 0 {
        return None;
    }
    let total = if tot16 != 0 { tot16 } else { tot32 };
    let fatsz = if fatsz16 != 0 { fatsz16 } else { fatsz32 };
    if total == 0 || fatsz == 0 {
        return None;
    }
    let root_secs = (root_ents * 32).div_ceil(bps);
    let fat_at = reserved;
    let root_at = fat_at + nfats * fatsz;
    let data_at = root_at + root_secs;
    if data_at >= total || d.len() / bps < data_at + 1 {
        return None;
    }
    let clusters = ((total - data_at) / spc) as u32;
    let ty = if clusters < 4085 {
        Ty::Fat12
    } else if clusters < 65525 {
        Ty::Fat16
    } else {
        Ty::Fat32
    };
    let root_clus = if ty == Ty::Fat32 { le32(d, 44)? } else { 0 };
    Some(Fat {
        ty,
        bps,
        spc,
        fat_at,
        fatsz,
        root_at,
        root_secs,
        data_at,
        clusters,
        root_clus,
    })
}

/// Reads FAT entry `n` (the value stored for cluster `n`).
/// FAT12 reads the packed 12-bit pair; FAT16/32 read 2/4 bytes and
/// mask the reserved high nibble on FAT32.
pub fn fat_entry(d: &[u8], f: &Fat, n: u32) -> Option<u32> {
    match f.ty {
        Ty::Fat12 => {
            let off = (n as usize) + (n as usize) / 2;
            let b0 = *d.get(f.fat_at * f.bps + off)? as u32;
            let b1 = *d.get(f.fat_at * f.bps + off + 1)? as u32;
            let v = b0 | (b1 << 8);
            Some(if n & 1 == 0 { v & 0xFFF } else { v >> 4 })
        }
        Ty::Fat16 => Some(le16(d, f.fat_at * f.bps + (n as usize) * 2)? as u32),
        Ty::Fat32 => Some(le32(d, f.fat_at * f.bps + (n as usize) * 4)? & 0x0FFF_FFFF),
    }
}

/// End-of-chain marker: `>=` this means "last cluster" (0xFF8 FAT12,
/// 0xFFF8 FAT16, 0x0FFFFFF8 FAT32).
pub fn eoc(f: &Fat) -> u32 {
    match f.ty {
        Ty::Fat12 => 0xFF8,
        Ty::Fat16 => 0xFFF8,
        Ty::Fat32 => 0x0FFF_FFF8,
    }
}

/// Follows the chain from `start`; caps at `f.clusters + 2` entries so
/// a cyclic FAT can't loop forever.
pub fn chain(d: &[u8], f: &Fat, start: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut c = start;
    while c >= 2 && c < eoc(f) && out.len() <= f.clusters as usize + 2 {
        out.push(c);
        match fat_entry(d, f, c) {
            Some(n) => c = n,
            None => break,
        }
    }
    out
}

/// Reads `size` bytes starting at `clus` following the chain.
pub fn read_chain(d: &[u8], f: &Fat, clus: u32, size: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(size);
    for c in chain(d, f, clus) {
        let at = cluster_at(f, c) * f.bps;
        let bytes = f.bps * f.spc;
        if at + bytes > d.len() {
            return None;
        }
        out.extend_from_slice(&d[at..at + bytes]);
        if out.len() >= size {
            out.truncate(size);
            return Some(out);
        }
    }
    if out.len() >= size {
        out.truncate(size);
        Some(out)
    } else {
        None
    }
}

/// Decodes one 32-byte directory slot. `None` skips LFN entries
/// (attr 0x0F), volume labels (attr 0x08), and free/deleted slots
/// (first byte 0x00 = end of directory, 0xE5 = deleted). Returns
/// `Some(None)` for a free entry, `Some(Some(..))` for a live one,
/// `None` at the end-of-directory marker.
fn dir_slot(d: &[u8], at: usize) -> Option<Option<DirEnt>> {
    let b0 = *d.get(at)?;
    if b0 == 0x00 {
        return None; // end of directory
    }
    let mut name = [0u8; 8];
    let mut ext = [0u8; 3];
    if at + 32 > d.len() {
        return None;
    }
    name.copy_from_slice(&d[at..at + 8]);
    ext.copy_from_slice(&d[at + 8..at + 11]);
    let attr = d[at + 11];
    if b0 == 0xE5 || attr == 0x0F || attr & 0x08 != 0 {
        return Some(None);
    }
    let clus = (le16(d, at + 26)? as u32) | ((le16(d, at + 20)? as u32) << 16);
    let size = le32(d, at + 28)?;
    Some(Some(DirEnt {
        name,
        ext,
        attr,
        clus,
        size,
        at,
    }))
}

/// Lists live entries of a directory: `clus = None` reads the
/// FAT12/16 root region; `Some(c)` follows cluster chain `c` (required
/// on FAT32, whose root lives at `f.root_clus`).
pub fn entries(d: &[u8], f: &Fat, clus: Option<u32>) -> Option<Vec<DirEnt>> {
    let mut out = Vec::new();
    let mut push = |at: usize| -> bool {
        match dir_slot(d, at) {
            None => false, // end of dir or truncated
            Some(None) => true,
            Some(Some(e)) => {
                out.push(e);
                true
            }
        }
    };
    match clus {
        None => {
            if f.ty == Ty::Fat32 {
                return entries(d, f, Some(f.root_clus));
            }
            let base = f.root_at * f.bps;
            for i in 0..(f.root_secs * f.bps / 32) {
                if !push(base + i * 32) {
                    break;
                }
            }
        }
        Some(c) => {
            for c in chain(d, f, c) {
                let base = cluster_at(f, c) * f.bps;
                for i in 0..(f.bps * f.spc / 32) {
                    if !push(base + i * 32) {
                        break;
                    }
                }
            }
        }
    }
    Some(out)
}

/// `"NAME    .EXT"` rendering of an 8.3 entry (keeps the padding for
/// round-trip fidelity — trim at the call site if unwanted).
pub fn entry_name(e: &DirEnt) -> String {
    let mut s = String::new();
    for b in e.name {
        s.push(b as char);
    }
    s.push('.');
    for b in e.ext {
        s.push(b as char);
    }
    s
}

/// Reads a file's full contents (size-limited chain read).
/// `None` for directories (use [`entries`]) or truncated chains.
pub fn file_bytes(d: &[u8], f: &Fat, e: &DirEnt) -> Option<Vec<u8>> {
    if e.attr & 0x10 != 0 {
        return None;
    }
    if e.size == 0 {
        return Some(Vec::new());
    }
    read_chain(d, f, e.clus, e.size as usize)
}

/// Finds a live entry by exact 8.3 name (11 padded bytes, e.g.
/// `b"HELLO   TXT"`) inside a directory listing.
pub fn find<'a>(ents: &'a [DirEnt], name11: &[u8; 11]) -> Option<&'a DirEnt> {
    ents.iter().find(|e| {
        let mut n = [0u8; 11];
        n[..8].copy_from_slice(&e.name);
        n[8..].copy_from_slice(&e.ext);
        &n == name11
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the doctest's 20-sector FAT12 image with one file.
    fn fat12() -> Vec<u8> {
        let mut img = vec![0u8; 512 * 20];
        img[11] = 0;
        img[12] = 2;
        img[13] = 1;
        img[14] = 1;
        img[16] = 1;
        img[17] = 16;
        img[19] = 20;
        img[22] = 1;
        // FAT12 entries 0,1 = reserved; entry 2 = EOC
        img[512] = 0xF0;
        img[512 + 1] = 0xFF;
        img[512 + 2] = 0xFF;
        img[512 + 3] = 0xFF;
        img[512 + 4] = 0x0F;
        let dir = 512 * 2;
        img[dir..dir + 8].copy_from_slice(b"HELLO   ");
        img[dir + 8..dir + 11].copy_from_slice(b"TXT");
        img[dir + 11] = 0x20;
        img[dir + 26] = 2;
        img[dir + 28] = 3; // size = 3
        img[512 * 3] = b'a';
        img[512 * 3 + 1] = b'b';
        img[512 * 3 + 2] = b'c';
        img
    }

    #[test]
    fn parses_fat12_geometry() {
        let f = parse(&fat12()).unwrap();
        assert_eq!(f.ty, Ty::Fat12);
        assert_eq!(f.bps, 512);
        assert_eq!(f.fat_at, 1);
        assert_eq!(f.root_at, 2);
        assert_eq!(f.data_at, 3);
        assert_eq!(f.clusters, 17);
    }

    #[test]
    fn reads_dir_and_file() {
        let img = fat12();
        let f = parse(&img).unwrap();
        let es = entries(&img, &f, None).unwrap();
        assert_eq!(es.len(), 1);
        assert_eq!(entry_name(&es[0]), "HELLO   .TXT");
        assert_eq!(find(&es, b"HELLO   TXT").unwrap().clus, 2);
        assert_eq!(file_bytes(&img, &f, &es[0]).unwrap(), b"abc");
    }

    #[test]
    fn fat12_packed_entry() {
        let img = fat12();
        let f = parse(&img).unwrap();
        assert_eq!(fat_entry(&img, &f, 0), Some(0xFF0));
        assert_eq!(fat_entry(&img, &f, 1), Some(0xFFF));
        assert_eq!(fat_entry(&img, &f, 2), Some(0xFFF)); // EOC
        assert_eq!(chain(&img, &f, 2), vec![2]);
    }

    #[test]
    fn follows_multi_cluster_chain() {
        let mut img = fat12();
        // entry 2 → 3 → EOC: cluster2 = 0x003, cluster3 = 0xFFF
        img[512 + 3] = 0x03;
        img[512 + 4] = 0xF0;
        img[512 + 5] = 0xFF;
        let f = parse(&img).unwrap();
        assert_eq!(chain(&img, &f, 2), vec![2, 3]);
        // file spans 2 clusters
        img[512 * 4] = b'd';
        img[512 * 4 + 1] = b'e';
        let es = entries(&img, &f, None).unwrap();
        let mut e2 = es[0].clone();
        e2.size = 512 + 2;
        e2.clus = 2;
        assert_eq!(cluster_at(&f, 2), f.data_at); // first data cluster
        assert_eq!(fat_entry(&img, &f, 2), Some(3));
        assert_eq!(eoc(&f), 0xFF8); // FAT12 end-of-chain mark
        assert_eq!(chain(&img, &f, 2), vec![2, 3]);
        assert_eq!(read_chain(&img, &f, 2, 3).unwrap()[..3], *b"abc");
        let bytes = file_bytes(&img, &f, &e2).unwrap();
        assert_eq!(&bytes[..3], b"abc");
        assert_eq!(&bytes[512..], b"de"); // 'd','e' live in cluster 3
    }

    #[test]
    fn skips_lfn_and_deleted() {
        let mut img = fat12();
        let f = parse(&img).unwrap();
        let dir = 512 * 2;
        // insert an LFN entry before HELLO
        img.copy_within(dir..dir + 32, dir + 32);
        img[dir..dir + 11].copy_from_slice(b"\x01l\x00o\x00n\x00g\x00n\x00");
        img[dir + 11] = 0x0F;
        // a deleted entry after
        img[dir + 64] = 0xE5;
        let es = entries(&img, &f, None).unwrap();
        assert_eq!(es.len(), 1);
        assert_eq!(entry_name(&es[0]), "HELLO   .TXT");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 64]).is_none()); // bps = 0
        let mut bad = fat12();
        bad.truncate(100);
        assert!(parse(&bad).is_none()); // data region missing
        let mut bad2 = fat12();
        bad2[13] = 0; // spc = 0
        assert!(parse(&bad2).is_none());
        let img = fat12();
        let f = parse(&img).unwrap();
        let es = entries(&img, &f, None).unwrap();
        let mut dir_e = es[0].clone();
        dir_e.attr = 0x10;
        assert!(file_bytes(&img, &f, &dir_e).is_none());
    }
}

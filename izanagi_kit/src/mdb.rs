//! LMDB `data.mdb` — pages 0 and 1 are meta pages; each carries an
//! `MDB_meta` with magic `0xBEEFC0DE`, a version, the map size, the two
//! sub-databases (free, main) and a transaction id. The meta with the
//! higher txnid is the live one. 64-bit little-endian layout only.
//!
//! ```
//! use izanagi_kit::mdb::parse;
//!
//! let mut f = vec![0u8; 8192];
//! f[0..8].copy_from_slice(&0u64.to_le_bytes());      // pgno 0
//! f[10..12].copy_from_slice(&16u16.to_le_bytes());   // P_META flag
//! f[16..20].copy_from_slice(&0xBEEFC0DEu32.to_le_bytes());
//! f[20..24].copy_from_slice(&1u32.to_le_bytes());    // version
//! let m = parse(&f).unwrap();
//! assert_eq!(m.active.version, 1);
//! ```

use std::vec::Vec;

/// Meta magic.
pub const MAGIC: u32 = 0xBEEF_C0DE;
/// `P_META` page flag.
const P_META: u16 = 0x10;

/// One `MDB_db` sub-database snapshot.
#[derive(Clone, Copy, Debug)]
pub struct DbInfo {
    /// B-tree depth.
    pub depth: u16,
    /// Branch page count.
    pub branch_pages: u64,
    /// Leaf page count.
    pub leaf_pages: u64,
    /// Overflow page count.
    pub overflow_pages: u64,
    /// Entry count.
    pub entries: u64,
    /// Root page number (`u64::MAX` = empty tree).
    pub root: u64,
}

/// One meta page's contents.
#[derive(Clone, Copy, Debug)]
pub struct Meta {
    /// Page number (`mp_pgno` — 0 or 1).
    pub pgno: u64,
    /// `mm_version`.
    pub version: u32,
    /// `mm_address`.
    pub address: u64,
    /// `mm_mapsize`.
    pub mapsize: u64,
    /// `mm_psize` (database page size).
    pub psize: u32,
    /// `mm_flags` (e.g. MDB_INTEGERKEY for the main db).
    pub flags: u32,
    /// Free list info.
    pub free: DbInfo,
    /// Main tree info.
    pub main: DbInfo,
    /// Last allocated page.
    pub last_pg: u64,
    /// Transaction id.
    pub txnid: u64,
}

/// Parsed LMDB file.
#[derive(Clone, Debug)]
pub struct Mdb {
    /// Both meta pages (may be only one if the file is tiny).
    pub metas: Vec<Meta>,
    /// The live meta (highest txnid).
    pub active: Meta,
}

fn u16le(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(o)?, *d.get(o + 1)?]))
}
fn u32le(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(o)?,
        *d.get(o + 1)?,
        *d.get(o + 2)?,
        *d.get(o + 3)?,
    ]))
}
fn u64le(d: &[u8], o: usize) -> Option<u64> {
    let s = d.get(o..o + 8)?;
    Some(u64::from_le_bytes([
        s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
    ]))
}

fn db(d: &[u8], o: usize) -> Option<DbInfo> {
    Some(DbInfo {
        depth: u16le(d, o + 6)?, // after pad u32 + flags u16
        branch_pages: u64le(d, o + 8)?,
        leaf_pages: u64le(d, o + 16)?,
        overflow_pages: u64le(d, o + 24)?,
        entries: u64le(d, o + 32)?,
        root: u64le(d, o + 40)?,
    })
}

fn meta(d: &[u8], page: usize) -> Option<Meta> {
    let o = page.checked_mul(4096)?; // meta pages sit on OS-page boundaries
                                     // MDB_page header: pgno u64, pad u16, flags u16, lower u16, upper u16 (16B)
    let pgno = u64le(d, o)?;
    let flags = u16le(d, o + 10)?;
    if flags & P_META == 0 || pgno != page as u64 {
        return None;
    }
    let m = o + 16;
    if u32le(d, m)? != MAGIC {
        return None;
    }
    Some(Meta {
        pgno,
        version: u32le(d, m + 4)?,
        address: u64le(d, m + 8)?,
        mapsize: u64le(d, m + 16)?,
        psize: u32le(d, m + 24)?,
        flags: u32le(d, m + 28)?,
        // mm_psize u32, mm_flags u32, then two MDB_db (48B each).
        free: db(d, m + 32)?,
        main: db(d, m + 80)?,
        last_pg: u64le(d, m + 128)?,
        txnid: u64le(d, m + 136)?,
    })
}

/// Parse `data.mdb`: locate up to two meta pages, choose the newer.
pub fn parse(d: &[u8]) -> Option<Mdb> {
    let mut metas = Vec::new();
    for page in 0..2usize {
        if let Some(m) = meta(d, page) {
            metas.push(m);
        }
    }
    let active = *metas.iter().max_by_key(|m| m.txnid)?;
    Some(Mdb { metas, active })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tx0: u64, tx1: u64) -> Vec<u8> {
        let mut f = vec![0u8; 8192];
        for (page, tx) in [(0usize, tx0), (1, tx1)] {
            let o = page * 4096;
            f[o..o + 8].copy_from_slice(&(page as u64).to_le_bytes());
            f[o + 10..o + 12].copy_from_slice(&P_META.to_le_bytes());
            f[o + 16..o + 20].copy_from_slice(&MAGIC.to_le_bytes());
            f[o + 20..o + 24].copy_from_slice(&1u32.to_le_bytes());
            f[o + 32..o + 40].copy_from_slice(&(1u64 << 20).to_le_bytes()); // mapsize
            f[o + 40..o + 44].copy_from_slice(&4096u32.to_le_bytes()); // psize @m+24
            f[o + 152..o + 160].copy_from_slice(&tx.to_le_bytes()); // txnid
        }
        f
    }

    #[test]
    fn picks_higher_txnid() {
        let m = parse(&fixture(7, 9)).unwrap();
        assert_eq!(m.metas.len(), 2);
        assert_eq!(m.active.txnid, 9);
        assert_eq!(m.active.version, 1);
        assert_eq!(m.active.mapsize, 1 << 20);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut f = fixture(1, 2);
        f[16] ^= 1; // corrupt magic
        f[4096 + 16] ^= 1;
        assert!(parse(&f).is_none());
        let mut g = fixture(1, 2);
        g[10] = 0; // clear P_META on page 0 — page 1 still valid
        g[4096 + 10] = 0;
        assert!(parse(&g).is_none());
    }
}

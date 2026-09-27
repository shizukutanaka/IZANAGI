//! Berkeley DB database files — page 0 is a meta page beginning with an
//! LSN, page number, magic `0x00053162`, version, page size and the
//! database type byte (`Btree`/`Hash`/`Queue`/`Recno`).
//!
//! ```
//! use izanagi_kit::bdb::parse;
//!
//! let mut f = vec![0u8; 512];
//! f[12..16].copy_from_slice(&0x00053162u32.to_le_bytes()); // magic
//! f[16..20].copy_from_slice(&9u32.to_le_bytes());          // version
//! f[20..24].copy_from_slice(&4096u32.to_le_bytes());       // pagesize
//! f[25] = 1;                                              // Btree
//! let b = parse(&f).unwrap();
//! assert_eq!(b.pagesize, 4096);
//! ```

/// Berkeley DB meta magic.
pub const MAGIC: u32 = 0x0005_3162;

/// Database type byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// B+tree.
    Btree,
    /// Extended linear hash.
    Hash,
    /// Fixed-length record queue.
    Queue,
    /// Variable-length record recno.
    Recno,
    /// Unknown/other type byte.
    Other(u8),
}

/// Parsed meta page.
#[derive(Clone, Debug)]
pub struct Bdb {
    /// Log sequence number `(file, offset)`.
    pub lsn: (u32, u32),
    /// Page number (0).
    pub pgno: u32,
    /// Format version.
    pub version: u32,
    /// Page size (power of two, 512..=65536).
    pub pagesize: u32,
    /// Database kind.
    pub kind: Kind,
    /// Free-list page number.
    pub free: u32,
    /// 20-byte unique file id.
    pub uid: [u8; 20],
}

fn u32le(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(o)?,
        *d.get(o + 1)?,
        *d.get(o + 2)?,
        *d.get(o + 3)?,
    ]))
}

/// Parse the meta page of a `.db` file.
pub fn parse(d: &[u8]) -> Option<Bdb> {
    if d.len() < 64 {
        return None;
    }
    if u32le(d, 12)? != MAGIC {
        return None;
    }
    let pagesize = u32le(d, 20)?;
    if !(512..=65536).contains(&pagesize) || !pagesize.is_power_of_two() {
        return None;
    }
    let mut uid = [0u8; 20];
    uid.copy_from_slice(d.get(32..52)?);
    Some(Bdb {
        lsn: (u32le(d, 0)?, u32le(d, 4)?),
        pgno: u32le(d, 8)?,
        version: u32le(d, 16)?,
        pagesize,
        kind: match *d.get(25)? {
            1 => Kind::Btree,
            2 => Kind::Hash,
            3 => Kind::Queue,
            4 => Kind::Recno,
            k => Kind::Other(k),
        },
        free: u32le(d, 28)?,
        uid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut f = vec![0u8; 512];
        f[12..16].copy_from_slice(&MAGIC.to_le_bytes());
        f[16..20].copy_from_slice(&9u32.to_le_bytes());
        f[20..24].copy_from_slice(&4096u32.to_le_bytes());
        f[25] = 1;
        f[28..32].copy_from_slice(&3u32.to_le_bytes());
        for (i, b) in f[32..52].iter_mut().enumerate() {
            *b = i as u8;
        }
        f
    }

    #[test]
    fn parses() {
        let b = parse(&fixture()).unwrap();
        assert_eq!(b.kind, Kind::Btree);
        assert_eq!(b.free, 3);
        assert_eq!(b.uid[19], 19);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut f = fixture();
        f[12] ^= 1;
        assert!(parse(&f).is_none());
        let mut g = fixture();
        g[20] = 1; // pagesize = 4097
        assert!(parse(&g).is_none());
        let mut h = fixture();
        h[25] = 2;
        assert_eq!(parse(&h).unwrap().kind, Kind::Hash);
    }
}

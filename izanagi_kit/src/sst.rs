//! LevelDB/RocksDB `*.sst` / `*.ldb` tables — the last 48 bytes are a
//! footer holding two varint block handles (metaindex, index) padded to
//! 40 bytes plus the LE u64 magic `0xdb4775248b80fb57`. Blocks carry a
//! 5-byte trailer `(type u8, crc u32)`; entries use the shared-prefix
//! restart encoding.
//!
//! ```
//! use izanagi_kit::sst::parse;
//!
//! let mut f = vec![0u8; 64];
//! // footer: metaindex handle (0,0), index handle (0,0), pad, magic
//! f[56..64].copy_from_slice(&0xdb4775248b80fb57u64.to_le_bytes());
//! let s = parse(&f).unwrap();
//! assert!(s.index.is_some());
//! ```

use std::vec::Vec;

/// SSTable magic (`table_magic_number`), little-endian on disk.
pub const MAGIC: u64 = 0xdb47_7524_8b80_fb57;
/// Footer length: two handles padded to 40 + 8-byte magic.
pub const FOOTER: usize = 48;

/// A `(offset, size)` block handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    /// Byte offset of the block payload.
    pub offset: u64,
    /// Payload length (before the 5-byte trailer).
    pub size: u64,
}

/// One index entry → the data block it points at.
#[derive(Clone, Copy, Debug)]
pub struct IndexEntry {
    /// Data-block handle decoded from the entry value.
    pub handle: Handle,
}

/// Parsed sstable metadata.
#[derive(Clone, Debug)]
pub struct Sst {
    /// Metaindex block handle.
    pub metaindex: Handle,
    /// Index block handle (always present; `Option` only mirrors the
    /// field-level API in docs).
    pub index: Option<Handle>,
    /// Data-block handles decoded from index entries.
    pub data: Vec<IndexEntry>,
}

fn varint64(d: &[u8], at: usize) -> Option<(u64, usize)> {
    let mut v = 0u64;
    let mut i = at;
    for _ in 0..10 {
        let b = *d.get(i)?;
        i += 1;
        v = (v << 7) | (b & 0x7f) as u64;
        if b < 0x80 {
            return Some((v, i));
        }
    }
    None
}

fn handle(d: &[u8], at: usize) -> Option<(Handle, usize)> {
    let (offset, at) = varint64(d, at)?;
    let (size, at) = varint64(d, at)?;
    Some((Handle { offset, size }, at))
}

/// Walk an index/metaindex block's shared-prefix entries.
/// Format: `(shared u32, non_shared u32, vlen u32, key_suffix, value)`
/// records up to `restarts_off`; restarts array + count trail it.
fn entries(d: &[u8], h: Handle) -> Option<Vec<IndexEntry>> {
    let off = usize::try_from(h.offset).ok()?;
    let end = off.checked_add(usize::try_from(h.size).ok()?)?;
    if end > d.len() {
        return None;
    }
    let blk = &d[off..end];
    if blk.len() < 4 {
        return Some(Vec::new());
    }
    let nrestarts = u32::from_le_bytes([
        blk[blk.len() - 4],
        blk[blk.len() - 3],
        blk[blk.len() - 2],
        blk[blk.len() - 1],
    ]) as usize;
    let restarts_end = blk.len() - 4;
    let restarts_off = restarts_end.checked_sub(nrestarts.checked_mul(4)?)?;
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < restarts_off {
        if i + 12 > restarts_off {
            return None;
        }
        let shared = u32::from_le_bytes([blk[i], blk[i + 1], blk[i + 2], blk[i + 3]]) as usize;
        let nsh = u32::from_le_bytes([blk[i + 4], blk[i + 5], blk[i + 6], blk[i + 7]]) as usize;
        let vlen = u32::from_le_bytes([blk[i + 8], blk[i + 9], blk[i + 10], blk[i + 11]]) as usize;
        i += 12;
        if i.checked_add(nsh + vlen)? > restarts_off {
            return None;
        }
        let _ = shared; // prefix reconstruction not needed for handles
        let v = &blk[i + nsh..i + nsh + vlen];
        let (handle, _) = handle(v, 0)?;
        out.push(IndexEntry { handle });
        i += nsh + vlen;
    }
    if i != restarts_off {
        return None;
    }
    Some(out)
}

/// Parse a `.sst` file: footer + index entries.
pub fn parse(d: &[u8]) -> Option<Sst> {
    if d.len() < FOOTER {
        return None;
    }
    let f = &d[d.len() - FOOTER..];
    let magic = u64::from_le_bytes([f[40], f[41], f[42], f[43], f[44], f[45], f[46], f[47]]);
    if magic != MAGIC {
        return None;
    }
    let (metaindex, at) = handle(f, 0)?;
    let (index, _) = handle(f, at)?;
    for h in [metaindex, index] {
        let end = h.offset.checked_add(h.size)?.checked_add(5)?; // trailer
        if end > (d.len() - FOOTER) as u64 {
            return None;
        }
    }
    let data = entries(d, index)?;
    for e in &data {
        let end = e.handle.offset.checked_add(e.handle.size)?.checked_add(5)?;
        if end > (d.len() - FOOTER) as u64 {
            return None;
        }
    }
    Some(Sst {
        metaindex,
        index: Some(index),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // index block: one entry (shared=0, nsh=1, key "k", vlen=2, handle 0,0) + restarts
        let mut blk = Vec::new();
        blk.extend_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
        blk.push(b'k');
        blk.extend_from_slice(&[0, 0]); // handle offset=0 size=0
        blk.extend_from_slice(&[0, 0, 0, 0]); // restarts[0]=0
        blk.extend_from_slice(&[1, 0, 0, 0]); // nrestarts=1
        blk.extend_from_slice(&[0; 5]); // trailer
        let mut f = vec![0u8; 16];
        f.extend_from_slice(&blk);
        let off = 16u8;
        let size = (blk.len() - 5) as u8;
        let mut foot = vec![0, 0, off, size]; // metaindex(0,0), index(16,size)
        foot.resize(40, 0);
        foot.extend_from_slice(&MAGIC.to_le_bytes());
        f.extend_from_slice(&foot);
        f
    }

    #[test]
    fn parses_footer_and_index() {
        let s = parse(&fixture()).unwrap();
        assert_eq!(s.metaindex.offset, 0);
        assert_eq!(s.index.unwrap().offset, 16);
        assert_eq!(s.data.len(), 1);
        assert_eq!(s.data[0].handle, Handle { offset: 0, size: 0 });
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut f = fixture();
        *f.last_mut().unwrap() ^= 1; // corrupt magic
        assert!(parse(&f).is_none());
        let mut g = fixture();
        let n = g.len();
        g[n - 48] = 0xFF; // metaindex offset varint garbage → out of range
        g[n - 47] = 0xFF;
        assert!(parse(&g).is_none());
    }
}

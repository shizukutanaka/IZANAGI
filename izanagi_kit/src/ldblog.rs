//! LevelDB/RocksDB write-ahead log / MANIFEST (`.log`) — the file is a
//! sequence of 32 KiB blocks, each a run of records `{crc u32, len u16,
//! type u8}` where a record's payload may be fragmented as
//! FULL/FIRST/MIDDLE/LAST. Type-0 means block padding. CRCs are
//! structural-only here (the format uses a masked CRC32C); lengths and
//! fragment order are fully validated.
//!
//! ```
//! use izanagi_kit::ldblog::parse;
//!
//! let mut b = vec![0u8; 4 + 2 + 1];
//! b[4] = 3; b[5] = 0; b[6] = 1; // len=3, FULL
//! b.extend_from_slice(b"abc");
//! let l = parse(&b).unwrap();
//! assert_eq!(l.records, vec![b"abc".to_vec()]);
//! ```

use std::vec::Vec;

/// Record fragment types.
pub const ZERO: u8 = 0;
/// Full single-fragment record.
pub const FULL: u8 = 1;
/// First fragment of a multi-block record.
pub const FIRST: u8 = 2;
/// Middle fragment.
pub const MIDDLE: u8 = 3;
/// Last fragment.
pub const LAST: u8 = 4;
/// Physical record header size.
const HEADER: usize = 7;
/// Physical block size.
const BLOCK: usize = 32768;

/// A parsed log.
#[derive(Clone, Debug)]
pub struct Log {
    /// Reassembled logical records in order.
    pub records: Vec<Vec<u8>>,
    /// Number of physical fragments consumed.
    pub fragments: usize,
}

/// Parse a LevelDB/RocksDB log.
pub fn parse(d: &[u8]) -> Option<Log> {
    let mut records = Vec::new();
    let mut fragments = 0usize;
    let mut cur: Option<Vec<u8>> = None;
    let mut off = 0usize;
    while off + HEADER <= d.len() {
        // tail of a block shorter than a header is padding
        let block_end = (off / BLOCK + 1) * BLOCK;
        if block_end - off < HEADER {
            let pad = block_end - off;
            if pad > d.len() - off {
                break;
            }
            if d[off..off + pad].iter().any(|c| *c != 0) {
                return None;
            }
            off += pad;
            continue;
        }
        let _crc = u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]]);
        let len = u16::from_le_bytes([d[off + 4], d[off + 5]]) as usize;
        let ty = d[off + 6];
        let avail = block_end - off - HEADER;
        if len > avail || off + HEADER + len > d.len() {
            return None;
        }
        let payload = &d[off + HEADER..off + HEADER + len];
        match ty {
            ZERO => {
                if len != 0 || payload.iter().any(|c| *c != 0) || cur.is_some() {
                    return None;
                }
            }
            FULL => {
                if cur.is_some() {
                    return None;
                }
                records.push(payload.to_vec());
            }
            FIRST => {
                if cur.is_some() {
                    return None;
                }
                cur = Some(payload.to_vec());
            }
            MIDDLE => {
                cur.as_mut()?.extend_from_slice(payload);
            }
            LAST => {
                cur.as_mut()?.extend_from_slice(payload);
                records.push(cur.take().unwrap_or_default());
            }
            _ => return None,
        }
        if ty != ZERO {
            fragments += 1;
        }
        off += HEADER + len;
    }
    if cur.is_some() || off != d.len() {
        return None;
    }
    Some(Log { records, fragments })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(ty: u8, payload: &[u8]) -> Vec<u8> {
        let mut r = vec![0; HEADER];
        r[4..6].copy_from_slice(&(payload.len() as u16).to_le_bytes());
        r[6] = ty;
        r.extend_from_slice(payload);
        r
    }

    #[test]
    fn simple_and_fragmented() {
        let mut d = rec(FULL, b"hello");
        // force a second block: pad to 32768 then FIRST..LAST
        d.resize(BLOCK, 0);
        d.extend_from_slice(&rec(FIRST, b"big"));
        d.extend_from_slice(&rec(MIDDLE, b"-"));
        d.extend_from_slice(&rec(LAST, b"data"));
        let l = parse(&d).unwrap();
        assert_eq!(l.records, vec![b"hello".to_vec(), b"big-data".to_vec()]);
        assert_eq!(l.fragments, 4);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").unwrap().records.is_empty());
        assert!(parse(&rec(MIDDLE, b"x")).is_none()); // MIDDLE without FIRST
        assert!(parse(&rec(9, b"x")).is_none()); // unknown type
        let mut d = rec(FULL, b"hi");
        d.resize(BLOCK + 1, 0); // stray byte into next block
        assert!(parse(&d).is_none());
    }
}

//! Avro Object Container File (OCF) envelope — Apache Avro spec.
//!
//! `Obj\x01` then a metadata **map** encoded with zig-zag varints:
//! `long count` (negative `-N` means N entries preceded by a `long`
//! byte size), then `count` × `(long len + UTF-8 key, long len +
//! bytes)` pairs, then the 16-byte sync marker. Data blocks are
//! `long count, long size, payload, sync` until EOF.
//!
//! ```
//! use izanagi_kit::avro::{parse, blocks};
//! let mut d = b"Obj\x01".to_vec();
//! d.push(2); // count=1 (zigzag: 2 = +1)
//! d.push(10); d.extend_from_slice(b"codec"); // len 5 → zigzag 10
//! d.push(4); d.extend_from_slice(b"sn");     // len 2 → zigzag 4
//! d.push(0); // end of map
//! d.extend_from_slice(&[0xAA; 16]);          // sync marker
//! d.extend_from_slice(&[2, 6]);              // block: 1 record, 3B
//! d.extend_from_slice(b"xyz");
//! d.extend_from_slice(&[0xAA; 16]);          // block sync
//! let a = parse(&d).unwrap();
//! assert_eq!(a.sync, [0xAA; 16]);
//! let bs = blocks(&d, &a);
//! assert_eq!(bs.len(), 1);
//! assert_eq!(bs[0].size, 3);
//! ```

/// A decoded metadata entry (e.g. `avro.schema`, `avro.codec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta {
    /// Key range.
    pub key_at: usize,
    /// Key length.
    pub key_len: usize,
    /// Value range.
    pub val_at: usize,
    /// Value length.
    pub val_len: usize,
}

/// Parsed OCF header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avro {
    /// The 16-byte sync marker written after every data block.
    pub sync: [u8; 16],
    /// Metadata map entries (`avro.schema`, `avro.codec`, …).
    pub meta: Vec<Meta>,
    /// Offset of the first data block.
    pub data_at: usize,
}

/// One data block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Record count in the block.
    pub count: u64,
    /// Payload byte size.
    pub size: u64,
    /// Payload offset.
    pub at: usize,
}

/// LEB128 varint → `(unsigned value, bytes consumed)`.
fn varint(d: &[u8], at: usize) -> Option<(u64, usize)> {
    let mut v: u64 = 0;
    let mut shift = 0u32;
    let mut i = at;
    loop {
        let b = *d.get(i)?;
        v |= ((b & 0x7F) as u64) << shift;
        i += 1;
        if b & 0x80 == 0 {
            return Some((v, i - at));
        }
        shift += 7;
        if shift > 63 || i - at > 10 {
            return None;
        }
    }
}

/// Zig-zag varint → signed value mapped back to non-negative u64
/// (`None` when the decoded value is negative in a count context…
/// callers use `zigzag` when they need sign).
fn zigzag(v: u64) -> i64 {
    ((v >> 1) as i64) ^ -((v & 1) as i64)
}

/// Parse the `Obj\x01` magic, metadata map, and sync marker.
/// `None` on malformed input.
pub fn parse(d: &[u8]) -> Option<Avro> {
    if d.get(0..4)? != b"Obj\x01" {
        return None;
    }
    let mut at = 4usize;
    let mut meta = Vec::new();
    loop {
        let (raw, n) = varint(d, at)?;
        at += n;
        let count = zigzag(raw);
        if count == 0 {
            break;
        }
        let n_entries = if count < 0 {
            count.checked_neg()?
        } else {
            count
        };
        if count < 0 {
            // negative count → byte-size long before the entries
            let (_sz, n) = varint(d, at)?;
            at += n;
        }
        for _ in 0..n_entries {
            let (klen, n) = varint(d, at)?;
            at += n;
            let kl = zigzag(klen);
            if kl < 0 {
                return None;
            }
            let key_at = at;
            at = at.checked_add(kl as usize)?;
            if at > d.len() {
                return None;
            }
            let (vlen, n) = varint(d, at)?;
            at += n;
            let vl = zigzag(vlen);
            if vl < 0 {
                return None;
            }
            let val_at = at;
            at = at.checked_add(vl as usize)?;
            if at > d.len() {
                return None;
            }
            meta.push(Meta {
                key_at,
                key_len: kl as usize,
                val_at,
                val_len: vl as usize,
            });
        }
    }
    let mut sync = [0u8; 16];
    sync.copy_from_slice(d.get(at..at + 16)?);
    Some(Avro {
        sync,
        meta,
        data_at: at + 16,
    })
}

/// Metadata value bytes for `key` (e.g. `b"avro.codec"`).
pub fn meta<'a>(d: &'a [u8], a: &Avro, key: &[u8]) -> Option<&'a [u8]> {
    a.meta
        .iter()
        .find(|m| &d[m.key_at..m.key_at + m.key_len] == key)
        .map(|m| &d[m.val_at..m.val_at + m.val_len])
}

/// Walk data blocks until EOF or a malformed block/sync marker.
pub fn blocks(d: &[u8], a: &Avro) -> Vec<Block> {
    let mut out = Vec::new();
    let mut at = a.data_at;
    while at < d.len() {
        let (rc, n) = match varint(d, at) {
            Some(v) => v,
            None => break,
        };
        at += n;
        let count = match zigzag(rc) {
            c if c <= 0 => break,
            c => c as u64,
        };
        let (rs, n) = match varint(d, at) {
            Some(v) => v,
            None => break,
        };
        at += n;
        let size = match zigzag(rs) {
            s if s < 0 => break,
            s => s as u64,
        };
        let payload = match at.checked_add(size as usize) {
            Some(e) if e <= d.len() => {
                let p = at;
                at = e;
                p
            }
            _ => break,
        };
        // sync marker follows every block
        if d.get(at..at + 16) != Some(&a.sync) {
            break;
        }
        at += 16;
        out.push(Block {
            count,
            size,
            at: payload,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"Obj\x01".to_vec();
        d.push(4); // count +2 (zigzag 4)
        d.push(20);
        d.extend_from_slice(b"avro.codec"); // len 10 → zz 20
        d.push(8);
        d.extend_from_slice(b"null"); // len 4 → zz 8
        d.push(24);
        d.extend_from_slice(b"avro.schema2"); // len 12 → zz 24
        d.push(4);
        d.extend_from_slice(b"{}"); // len 2 → zz 4
        d.push(0);
        d.extend_from_slice(&[0x5A; 16]);
        d.extend_from_slice(&[4, 10]); // 2 records, 5 bytes
        d.extend_from_slice(b"abcde");
        d.extend_from_slice(&[0x5A; 16]);
        d.extend_from_slice(&[2, 6]); // 1 record, 3 bytes
        d.extend_from_slice(b"xyz");
        d.extend_from_slice(&[0x5A; 16]);
        d
    }

    #[test]
    fn header_meta_blocks() {
        let d = fixture();
        let a = parse(&d).unwrap();
        assert_eq!(a.sync, [0x5A; 16]);
        assert_eq!(a.meta.len(), 2);
        assert_eq!(meta(&d, &a, b"avro.codec"), Some(&b"null"[..]));
        assert_eq!(meta(&d, &a, b"avro.schema2"), Some(&b"{}"[..]));
        assert_eq!(meta(&d, &a, b"nope"), None);
        let bs = blocks(&d, &a);
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0].count, 2);
        assert_eq!(bs[0].size, 5);
        assert_eq!(&d[bs[0].at..bs[0].at + 5], b"abcde");
        assert_eq!(bs[1].count, 1);
        // whole file consumed
        assert_eq!(bs[1].at + 3 + 16, d.len());
    }

    #[test]
    fn negative_count_map() {
        let mut d = b"Obj\x01".to_vec();
        d.push(3); // zigzag 3 → -2 → 2 entries, size follows
        d.push(6); // block byte size 3 (zz 6) — content unchecked
        d.push(4);
        d.extend_from_slice(b"ky");
        d.push(2);
        d.extend_from_slice(b"v");
        d.push(4);
        d.extend_from_slice(b"k2");
        d.push(2);
        d.extend_from_slice(b"w");
        d.push(0);
        d.extend_from_slice(&[0x11; 16]);
        let a = parse(&d).unwrap();
        assert_eq!(a.meta.len(), 2);
        assert_eq!(meta(&d, &a, b"ky"), Some(&b"v"[..]));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"Obj\x02").is_none());
        let mut d = fixture();
        d.truncate(6); // mid-map
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        // corrupt last block's sync
        let n = d2.len();
        d2[n - 1] = 0x00;
        let a = parse(&d2).unwrap();
        assert_eq!(blocks(&d2, &a).len(), 1);
    }
}

//! Redis RDB snapshot files — `REDIS` + 4-digit version, then opcode
//! records: `0xFA` aux, `0xFE`/`0xFB` select/resize-db, `0xFC`/`0xFD`
//! expiry, then `type key value`. String-encoded lengths (6/14/32/64
//! bit + `0xC0` int8/16/32 + LZF) are honoured; LZF payloads are
//! skipped by length, never decompressed.
//!
//! ```
//! use izanagi_kit::rdb::parse;
//!
//! let r = parse(b"REDIS0009\xFE\x00\x00\x03key\x03val\xFF\x00\x00\x00\x00\x00\x00\x00\x00").unwrap();
//! assert_eq!(r.version, 9);
//! assert_eq!(r.dbs[0].entries[0].key, "key");
//! ```

use std::string::String;
use std::vec::Vec;

/// One key/value record (value is summarised, not decoded).
#[derive(Clone, Debug)]
pub struct Entry {
    /// Key bytes, lossy UTF-8.
    pub key: String,
    /// RDB object type byte.
    pub kind: u8,
    /// Expiry in ms since epoch when `0xFC`/`0xFD` was present.
    pub expire_ms: Option<u64>,
    /// Element count of the value (1 for plain strings).
    pub items: usize,
}

/// One logical database (`0xFE` section).
#[derive(Clone, Debug)]
pub struct Db {
    /// Database index.
    pub index: u64,
    /// `0xFB` resize hint `(db_size, expires_size)` when present.
    pub resize: Option<(u64, u64)>,
    /// Key/value records.
    pub entries: Vec<Entry>,
}

/// A parsed RDB file.
#[derive(Clone, Debug)]
pub struct Rdb {
    /// File format version (1..=11 observed).
    pub version: u32,
    /// `0xFA` aux fields `(key, value)`.
    pub aux: Vec<(String, String)>,
    /// Databases in file order.
    pub dbs: Vec<Db>,
    /// Trailing 8-byte checksum was present.
    pub checksum: bool,
}

struct Rd<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> Rd<'a> {
    fn u8(&mut self) -> Option<u8> {
        let b = *self.d.get(self.at)?;
        self.at += 1;
        Some(b)
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.at.checked_add(n)? > self.d.len() {
            return None;
        }
        let s = &self.d[self.at..self.at + n];
        self.at += n;
        Some(s)
    }
    fn be32(&mut self) -> Option<u32> {
        // RDB 32-bit lengths are big-endian on disk; fold manually so
        // no target-dependent or BE conversion is used.
        let s = self.take(4)?;
        Some(((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | s[3] as u32)
    }
    fn le32(&mut self) -> Option<u32> {
        let s = self.take(4)?;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn le64(&mut self) -> Option<u64> {
        let s = self.take(8)?;
        Some(u64::from_le_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }
    /// RDB length encoding: 00xxxxxx=6bit, 01xxxxxx=14bit BE,
    /// 10xxxxxx=32bit BE, byte 0x81=64bit LE.
    fn len(&mut self) -> Option<u64> {
        let b = self.u8()?;
        if b == 0x81 {
            return self.le64();
        }
        match b >> 6 {
            0 => Some((b & 0x3f) as u64),
            1 => Some((((b & 0x3f) as u64) << 8) | self.u8()? as u64),
            2 => Some(self.be32()? as u64),
            _ => None, // 0xC0 family is encoded, not a length
        }
    }
    /// Encoded string → its bytes. LZF content is reported as the raw
    /// compressed payload (caller decides).
    fn string(&mut self) -> Option<String> {
        let b = self.u8()?;
        let n = if b >= 0xC0 {
            match b & 0x3f {
                0 => return Some(self.u8()?.to_string()),
                1 => {
                    let s = self.take(2)?;
                    return Some(u16::from_le_bytes([s[0], s[1]]).to_string());
                }
                2 => return Some(self.le32()?.to_string()),
                3 => {
                    // LZF: clen, ulen, then clen bytes we skip.
                    let clen = self.len()? as usize;
                    let _ulen = self.len()?;
                    self.take(clen)?;
                    return Some(String::new());
                }
                _ => return None,
            }
        } else if b == 0x81 {
            let n = self.le64()?;
            usize::try_from(n).ok()?
        } else {
            match b >> 6 {
                0 => (b & 0x3f) as usize,
                1 => (((b & 0x3f) as usize) << 8) | self.u8()? as usize,
                _ => self.be32()? as usize,
            }
        };
        Some(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
    /// Skip one value given its RDB type byte; returns element count.
    fn value(&mut self, t: u8) -> Option<usize> {
        match t {
            0 => {
                self.string()?;
                Some(1)
            }
            1 | 2 => {
                let n = self.len()?;
                for _ in 0..n {
                    self.string()?;
                }
                Some(n as usize)
            }
            3 => {
                // zset: member + ASCII score (legacy len-prefixed)
                let n = self.len()?;
                for _ in 0..n {
                    self.string()?;
                    let l = self.u8()?;
                    match l {
                        253..=255 => {}
                        _ => {
                            self.take(l as usize)?;
                        }
                    }
                }
                Some(n as usize)
            }
            4 => {
                // zset2: member + f64 score (kept opaque)
                let n = self.len()?;
                for _ in 0..n {
                    self.string()?;
                    self.take(8)?;
                }
                Some(n as usize)
            }
            5 => {
                // hash: field/value pairs
                let n = self.len()?;
                for _ in 0..n {
                    self.string()?;
                    self.string()?;
                }
                Some(n as usize)
            }
            // blob-typed containers (ziplist/intset/quicklist/listpack
            // families and module data are all length-prefixed strings)
            _ => {
                self.string()?;
                Some(1)
            }
        }
    }
}

/// Parse a Redis RDB snapshot.
pub fn parse(d: &[u8]) -> Option<Rdb> {
    if !d.starts_with(b"REDIS") {
        return None;
    }
    let v: u32 = std::str::from_utf8(d.get(5..9)?).ok()?.parse().ok()?;
    let mut r = Rd { d, at: 9 };
    let mut rdb = Rdb {
        version: v,
        aux: Vec::new(),
        dbs: Vec::new(),
        checksum: false,
    };
    let mut pending_expire: Option<u64> = None;
    let mut seen_db = false;
    loop {
        match r.u8()? {
            0xFF => {
                // EOF: optional 8-byte checksum tail
                if r.at + 8 == d.len() {
                    rdb.checksum = true;
                    r.at += 8;
                }
                if r.at != d.len() {
                    return None;
                }
                return Some(rdb);
            }
            0xFA => {
                let k = r.string()?;
                let v = r.string()?;
                rdb.aux.push((k, v));
            }
            0xFE => {
                let index = r.len()?;
                rdb.dbs.push(Db {
                    index,
                    resize: None,
                    entries: Vec::new(),
                });
                seen_db = true;
            }
            0xFB => {
                let a = r.len()?;
                let b = r.len()?;
                rdb.dbs.last_mut()?.resize = Some((a, b));
            }
            0xFC => pending_expire = Some(r.le64()?),
            0xFD => pending_expire = Some(r.le32()? as u64 * 1000),
            kind => {
                if !seen_db {
                    return None;
                }
                let key = r.string()?;
                let items = r.value(kind)?;
                rdb.dbs.last_mut()?.entries.push(Entry {
                    key,
                    kind,
                    expire_ms: pending_expire.take(),
                    items,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_db() {
        let r = parse(b"REDIS0009\xFE\x00\x00\x03key\x03val\xFF\x00\x00\x00\x00\x00\x00\x00\x00")
            .unwrap();
        assert_eq!(r.dbs[0].index, 0);
        assert_eq!(r.dbs[0].entries[0].kind, 0);
        assert_eq!(r.dbs[0].entries[0].items, 1);
        assert!(r.checksum);
    }

    #[test]
    fn aux_expire_and_containers() {
        let mut b = Vec::new();
        b.extend_from_slice(
            b"REDIS0010\xFA\x03aof\x03yes\xFE\x02\xFB\x03\x01\xFD\x00\x00\x00\x01\x00\x03abc\x03def",
        );
        b.extend_from_slice(b"\x02\x01s\x01\x02aa\xFF");
        let r = parse(&b).unwrap();
        assert_eq!(r.aux[0], ("aof".to_string(), "yes".to_string()));
        assert_eq!(r.dbs[0].resize, Some((3, 1)));
        assert_eq!(r.dbs[0].entries[0].expire_ms, Some(16777216000));
        assert_eq!(r.dbs[0].entries[1].items, 1);
        assert_eq!(r.dbs[0].entries[1].key, "s");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"NOREDIS").is_none());
        assert!(parse(b"REDIS0009\x00").is_none()); // entry before any db
        assert!(parse(b"REDIS0009\xFE\x00").is_none()); // missing EOF
        assert!(parse(b"REDISxxxx").is_none());
    }
}

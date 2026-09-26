//! MAME CHD (Compressed Hunks of Data) headers, versions 1–5.
//!
//! Every version starts with the 8-byte `"MComprHD"` magic, a u32
//! big-endian header length and a u32 version. The rest of the header
//! differs per version: v1–v4 carry `compression`, `hunk_size`,
//! `total_hunks`, CHS geometry and SHA-1 digests; v5 replaces that
//! with a compressor list, logical size and map/meta offsets.
//!
//! ```
//! use izanagi_kit::chd::{parse, MAGIC};
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(MAGIC);
//! d.extend_from_slice(&108u32.to_be_bytes()); // header length
//! d.extend_from_slice(&3u32.to_be_bytes());   // version 3
//! d.extend_from_slice(&0u32.to_be_bytes());   // flags
//! d.extend_from_slice(&1u32.to_be_bytes());   // compression
//! d.extend_from_slice(&4128u32.to_be_bytes());// hunk size
//! d.extend_from_slice(&16u32.to_be_bytes());  // total hunks
//! d.extend_from_slice(&[0u8; 12]);            // cylinders/heads/sectors
//! d.extend_from_slice(&[0u8; 36]);            // unused
//! d.extend_from_slice(&[0xAB; 20]);           // sha1
//! d.extend_from_slice(&[0; 4]);               // pad to len 108
//! let c = parse(&d).unwrap();
//! assert_eq!(c.version, 3);
//! assert_eq!(c.total_hunks, Some(16));
//! assert_eq!(c.hunk_size, Some(4128));
//! ```

/// Magic bytes.
pub const MAGIC: &[u8; 8] = b"MComprHD";

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

fn be64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(be32(d, at)?) << 32 | u64::from(be32(d, at + 4)?))
}

/// A parsed CHD header. Fields present only in some versions are
/// `Option`/`[u8; 20]`-zeroed.
#[derive(Clone, Debug, PartialEq)]
pub struct Chd {
    /// Header byte length as stored.
    pub length: u32,
    /// Format version (1..=5).
    pub version: u32,
    /// Flags (v3+).
    pub flags: u32,
    /// v3–v4: compression method id. v5: up to four compressor ids.
    pub compressors: [u8; 4],
    /// v3–v4: bytes per uncompressed hunk.
    pub hunk_size: Option<u32>,
    /// v3–v4: total hunk count.
    pub total_hunks: Option<u32>,
    /// v3–v4: cylinders/heads/sectors.
    pub chs: Option<(u32, u32, u32)>,
    /// v1–v5: uncompressed data size in bytes (v1/v2 derive it from
    /// hunk_size × total_hunks; v5 stores it directly).
    pub logical_bytes: Option<u64>,
    /// v5: byte offset of the hunk map.
    pub map_at: Option<u64>,
    /// v5: byte offset of the metadata region.
    pub meta_at: Option<u64>,
    /// SHA-1 of the uncompressed data (all-zero when absent).
    pub sha1: [u8; 20],
    /// v4+: SHA-1 of the parent's data (all-zero when absent).
    pub parent_sha1: [u8; 20],
}

/// Parse a CHD header. Returns `None` on a bad magic, an unknown
/// version, or a truncated header.
pub fn parse(d: &[u8]) -> Option<Chd> {
    if d.get(..8)? != MAGIC {
        return None;
    }
    let length = be32(d, 8)?;
    let version = be32(d, 12)?;
    let mut c = Chd {
        length,
        version,
        flags: 0,
        compressors: [0; 4],
        hunk_size: None,
        total_hunks: None,
        chs: None,
        logical_bytes: None,
        map_at: None,
        meta_at: None,
        sha1: [0; 20],
        parent_sha1: [0; 20],
    };
    match version {
        1 | 2 => {
            // v1/v2: flags@16, compression@20, hunksize@24, totalhunks@28
            // (v2 adds sha1@44); header = 44 / 80 bytes
            c.flags = be32(d, 16)?;
            c.compressors[0] = be32(d, 20)? as u8;
            let hs = be32(d, 24)?;
            let th = be32(d, 28)?;
            c.hunk_size = Some(hs);
            c.total_hunks = Some(th);
            c.logical_bytes = Some(u64::from(hs) * u64::from(th));
            if version == 2 {
                c.sha1.copy_from_slice(d.get(44..64)?);
            }
        }
        3 | 4 => {
            // flags@16 compression@20 hunksize@24 totalhunks@28
            // cyl@32 heads@36 sect@40 sha1@44 parentsha1@64 (v4)
            c.flags = be32(d, 16)?;
            c.compressors[0] = be32(d, 20)? as u8;
            let hs = be32(d, 24)?;
            let th = be32(d, 28)?;
            c.hunk_size = Some(hs);
            c.total_hunks = Some(th);
            c.chs = Some((be32(d, 32)?, be32(d, 36)?, be32(d, 40)?));
            c.logical_bytes = Some(u64::from(hs) * u64::from(th));
            c.sha1.copy_from_slice(d.get(44..64)?);
            if version == 4 {
                c.parent_sha1.copy_from_slice(d.get(64..84)?);
            }
        }
        5 => {
            // compressors[4]@16, logicalbytes@20, mapoffset@28,
            // metaoffset@36, sha1@44, parentsha1@64, rawsha1@84
            c.compressors.copy_from_slice(d.get(16..20)?);
            c.logical_bytes = Some(be64(d, 20)?);
            c.map_at = Some(be64(d, 28)?);
            c.meta_at = Some(be64(d, 36)?);
            c.sha1.copy_from_slice(d.get(44..64)?);
            c.parent_sha1.copy_from_slice(d.get(64..84)?);
        }
        _ => return None,
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn be(d: &mut Vec<u8>, v: u32) {
        d.extend_from_slice(&v.to_be_bytes());
    }
    fn be64v(d: &mut Vec<u8>, v: u64) {
        d.extend_from_slice(&v.to_be_bytes());
    }

    fn v3() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(MAGIC);
        be(&mut d, 108);
        be(&mut d, 3);
        be(&mut d, 0); // flags
        be(&mut d, 1); // zlib
        be(&mut d, 4128); // hunksize
        be(&mut d, 16); // total
        be(&mut d, 100); // cyl
        be(&mut d, 16); // heads
        be(&mut d, 63); // sect
        d.extend_from_slice(&[0xAB; 20]); // sha1
        d.resize(108, 0);
        d
    }

    #[test]
    fn v3_fields() {
        let c = parse(&v3()).unwrap();
        assert_eq!(c.version, 3);
        assert_eq!(c.length, 108);
        assert_eq!(c.compressors[0], 1);
        assert_eq!(c.hunk_size, Some(4128));
        assert_eq!(c.total_hunks, Some(16));
        assert_eq!(c.chs, Some((100, 16, 63)));
        assert_eq!(c.logical_bytes, Some(4128 * 16));
        assert_eq!(c.sha1[0], 0xAB);
        assert_eq!(c.parent_sha1, [0; 20]);
        assert_eq!(c.map_at, None);
    }

    #[test]
    fn v5_fields() {
        let mut d = Vec::new();
        d.extend_from_slice(MAGIC);
        be(&mut d, 124);
        be(&mut d, 5);
        d.extend_from_slice(&[1, 2, 0, 0]); // zlib+lzma
        be64v(&mut d, 0x40_0000); // logical
        be64v(&mut d, 124); // map offset
        be64v(&mut d, 4096); // meta offset
        d.extend_from_slice(&[0xCD; 20]); // sha1
        d.extend_from_slice(&[0xEF; 20]); // parent
        d.extend_from_slice(&[0x99; 20]); // raw sha1
        let c = parse(&d).unwrap();
        assert_eq!(c.version, 5);
        assert_eq!(c.compressors, [1, 2, 0, 0]);
        assert_eq!(c.logical_bytes, Some(0x40_0000));
        assert_eq!(c.map_at, Some(124));
        assert_eq!(c.meta_at, Some(4096));
        assert_eq!(c.sha1[0], 0xCD);
        assert_eq!(c.parent_sha1[0], 0xEF);
        assert_eq!(c.hunk_size, None);
    }

    #[test]
    fn v1_has_no_sha() {
        let mut d = Vec::new();
        d.extend_from_slice(MAGIC);
        be(&mut d, 44);
        be(&mut d, 1);
        be(&mut d, 0);
        be(&mut d, 1);
        be(&mut d, 512);
        be(&mut d, 10);
        let c = parse(&d).unwrap();
        assert_eq!(c.logical_bytes, Some(5120));
        assert_eq!(c.sha1, [0; 20]);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(b"XXXX\x00\x00\x00\x10\x00\x00\x00\x05"), None);
        let mut bad = v3();
        bad[12..16].copy_from_slice(&6u32.to_be_bytes()); // unknown version
        assert_eq!(parse(&bad), None);
        let mut short = v3();
        short.truncate(60);
        assert_eq!(parse(&short), None);
    }

    #[test]
    fn constants() {
        assert_eq!(MAGIC, b"MComprHD");
    }
}

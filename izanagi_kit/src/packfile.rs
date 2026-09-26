//! Git packfile + pack index (`.pack`/`.idx`) — the packed
//! counterpart of `git`'s loose-object layer. Entries carry a
//! `(type, size)` varint header followed by a zlib stream
//! (`inflate_zlib_count` supplies the member boundary). OFS_DELTA
//! and REF_DELTA entries keep their delta text; resolving deltas is
//! out of scope.
//!
//! ```
//! use izanagi_kit::{deflate, packfile};
//! let body = b"blob body";
//! let z = deflate::deflate_zlib(body);
//! let mut p = b"PACK".to_vec();
//! p.extend_from_slice(&[0, 0, 0, 2]); // version 2
//! p.extend_from_slice(&[0, 0, 0, 1]); // one object
//! // hdr: type 1 (commit=1… blob=3), size 9 → 0x39 with size bits
//! p.push((3 << 4) | (body.len() as u8 & 0x0F));
//! p.extend_from_slice(&z);
//! p.extend_from_slice(&[0; 20]); // trailer sha1 (not validated)
//! let pk = packfile::parse(&p).unwrap();
//! assert_eq!(packfile::type_name(pk.entries[0].ty), "blob");
//! assert_eq!(pk.entries[0].data, body.to_vec());
//! ```

use std::vec::Vec;

fn r32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (*d.get(at)? as u32) << 24
            | (*d.get(at + 1)? as u32) << 16
            | (*d.get(at + 2)? as u32) << 8
            | *d.get(at + 3)? as u32,
    )
}

/// One packed object.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Object type: 1 commit, 2 tree, 3 blob, 4 tag,
    /// 6 OFS_DELTA, 7 REF_DELTA.
    pub ty: u8,
    /// Inflated content (for deltas: the delta instructions).
    pub data: Vec<u8>,
    /// File offset of the object header.
    pub offset: usize,
    /// For OFS_DELTA: base object's offset. `None` otherwise.
    pub base_offset: Option<u64>,
    /// For REF_DELTA: base object's sha1 name. `None` otherwise.
    pub base_sha: Option<[u8; 20]>,
}

/// A parsed packfile.
#[derive(Clone, Debug)]
pub struct Pack {
    /// Pack version (2 or 3).
    pub version: u32,
    /// Objects in file order.
    pub entries: Vec<Entry>,
    /// The 20-byte SHA-1 trailer.
    pub trailer: [u8; 20],
}

/// Type number to git's name.
pub fn type_name(ty: u8) -> &'static str {
    match ty {
        1 => "commit",
        2 => "tree",
        3 => "blob",
        4 => "tag",
        6 => "ofs-delta",
        7 => "ref-delta",
        _ => "unknown",
    }
}

/// Parses a `.pack`: `PACK` magic, version 2/3, count, then zlib
/// members, then a 20-byte trailer.
pub fn parse(d: &[u8]) -> Option<Pack> {
    if d.get(0..4)? != b"PACK" {
        return None;
    }
    let version = r32(d, 4)?;
    if version != 2 && version != 3 {
        return None;
    }
    let count = r32(d, 8)? as usize;
    let mut i = 12usize;
    let mut entries = Vec::with_capacity(count.min(1 << 22));
    for _ in 0..count {
        let offset = i;
        // (type|size) varint: first byte = [msb|type3|size4],
        // continuation bytes carry 7 more size bits each.
        let mut b = *d.get(i)?;
        i += 1;
        let ty = (b >> 4) & 7;
        let mut size = (b & 0x0F) as u64;
        let mut shift = 4usize;
        while b & 0x80 != 0 {
            b = *d.get(i)?;
            i += 1;
            size |= ((b & 0x7F) as u64) << shift;
            shift += 7;
            if shift > 63 {
                return None;
            }
        }
        if ty == 0 || ty == 5 {
            return None; // reserved types
        }
        let mut base_offset = None;
        let mut base_sha = None;
        if ty == 6 {
            // OFS_DELTA: base offset as an n-byte varint —
            // each byte adds 7 bits but the msb adds 1 first.
            let mut b2 = *d.get(i)?;
            i += 1;
            let mut off = (b2 & 0x7F) as u64;
            while b2 & 0x80 != 0 {
                b2 = *d.get(i)?;
                i += 1;
                off = ((off + 1) << 7) | (b2 & 0x7F) as u64;
            }
            base_offset = Some(offset as u64 - off);
        } else if ty == 7 {
            let mut sha = [0u8; 20];
            sha.copy_from_slice(d.get(i..i + 20)?);
            i += 20;
            base_sha = Some(sha);
        }
        let (data, used) = crate::inflate::inflate_zlib_count(d.get(i..)?)?;
        if data.len() as u64 != size {
            return None; // declared size mismatch
        }
        i += used;
        entries.push(Entry {
            ty,
            data,
            offset,
            base_offset,
            base_sha,
        });
    }
    let trailer_end = i.checked_add(20)?;
    if trailer_end != d.len() {
        return None;
    }
    let mut trailer = [0u8; 20];
    trailer.copy_from_slice(&d[i..trailer_end]);
    Some(Pack {
        version,
        entries,
        trailer,
    })
}

/// Parsed `.idx` v2: fanout table, sorted sha1 names, and the
/// (crc32, offset) pairs — the lookup index shipped beside a pack.
#[derive(Clone, Debug)]
pub struct Idx {
    /// Sorted sha1 names.
    pub names: Vec<[u8; 20]>,
    /// `crc32s[i]` is `names[i]`'s zlib-member crc.
    pub crc32s: Vec<u32>,
    /// `offsets[i]` is `names[i]`'s pack offset — MSB set means a
    /// pointer into the 8-byte large-offset table (resolved by
    /// [`offset_of`]).
    pub offsets: Vec<u32>,
    /// Raw large-offset table bytes (8-byte BE entries).
    pub large: Vec<u8>,
    /// The pack trailer's own sha1.
    pub pack_sha: [u8; 20],
}

/// Parses a `.idx` v2 file (`\xFFtOc` magic + fanout + names +
/// crcs + offsets + pack sha + idx sha).
pub fn parse_idx(d: &[u8]) -> Option<Idx> {
    if d.get(0..4)? != [0xFF, b't', b'O', b'c'] {
        return None;
    }
    if r32(d, 4)? != 2 {
        return None;
    }
    let mut fanout = [0u32; 256];
    for (j, f) in fanout.iter_mut().enumerate() {
        *f = r32(d, 8 + j * 4)?;
    }
    for w in fanout.windows(2) {
        if w[0] > w[1] {
            return None; // fanout must be non-decreasing
        }
    }
    let n = fanout[255] as usize;
    let names_at: usize = 8 + 256 * 4;
    let crc_at = names_at.checked_add(n.checked_mul(20)?)?;
    let off_at = crc_at.checked_add(n.checked_mul(4)?)?;
    let large_at = off_at.checked_add(n.checked_mul(4)?)?;
    let mut names = Vec::with_capacity(n.min(1 << 22));
    let mut offsets = Vec::with_capacity(names.capacity());
    let mut crc32s = Vec::with_capacity(names.capacity());
    for i in 0..n {
        let mut name = [0u8; 20];
        name.copy_from_slice(d.get(names_at + i * 20..names_at + i * 20 + 20)?);
        names.push(name);
        crc32s.push(r32(d, crc_at + i * 4)?);
        offsets.push(r32(d, off_at + i * 4)?);
    }
    // large-offset table: sized by the highest referenced slot
    let large_n = offsets
        .iter()
        .filter(|&&o| o & 0x8000_0000 != 0)
        .map(|&o| (o & 0x7FFF_FFFF) as usize)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);
    let large_end = large_at.checked_add(large_n.checked_mul(8)?)?;
    let pack_sha_at = large_end;
    let mut pack_sha = [0u8; 20];
    pack_sha.copy_from_slice(d.get(pack_sha_at..pack_sha_at + 20)?);
    Some(Idx {
        names,
        crc32s,
        offsets,
        large: d.get(large_at..large_end)?.to_vec(),
        pack_sha,
    })
}

/// Resolves `offsets[i]` — MSB set indexes the 8-byte large-offset
/// table, otherwise it's the direct pack offset.
pub fn offset_of(idx: &Idx, i: usize) -> Option<u64> {
    let raw = *idx.offsets.get(i)?;
    if raw & 0x8000_0000 == 0 {
        return Some(raw as u64);
    }
    let j = (raw & 0x7FFF_FFFF) as usize;
    let hi = r32(&idx.large, j.checked_mul(8)?)? as u64;
    let lo = r32(&idx.large, j * 8 + 4)? as u64;
    Some(hi << 32 | lo)
}

/// Binary-search an index for a sha1 → its pack offset.
pub fn find(idx: &Idx, sha: &[u8; 20]) -> Option<u64> {
    idx.names
        .binary_search(sha)
        .ok()
        .and_then(|i| offset_of(idx, i))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deflate::deflate_zlib;

    fn pack_one(body: &[u8], ty: u8) -> Vec<u8> {
        let z = deflate_zlib(body);
        let mut p = b"PACK".to_vec();
        p.extend_from_slice(&[0, 0, 0, 2]);
        p.extend_from_slice(&[0, 0, 0, 1]);
        let mut size = body.len() as u64;
        let mut first = (ty << 4) | (size as u8 & 0x0F);
        size >>= 4;
        if size != 0 {
            first |= 0x80;
        }
        p.push(first);
        while size != 0 {
            let mut b = (size & 0x7F) as u8;
            size >>= 7;
            if size != 0 {
                b |= 0x80;
            }
            p.push(b);
        }
        p.extend_from_slice(&z);
        p.extend_from_slice(&[0; 20]);
        p
    }

    #[test]
    fn parses_blob() {
        let p = pack_one(b"hello git", 3);
        let pk = parse(&p).unwrap();
        assert_eq!(pk.version, 2);
        assert_eq!(pk.entries.len(), 1);
        assert_eq!(pk.entries[0].ty, 3);
        assert_eq!(type_name(pk.entries[0].ty), "blob");
        assert_eq!(pk.entries[0].data, b"hello git".to_vec());
    }

    #[test]
    fn large_size_varint() {
        let body = vec![0xABu8; 2000]; // needs 2nd size byte
        let p = pack_one(&body, 3);
        let pk = parse(&p).unwrap();
        assert_eq!(pk.entries[0].data, body);
    }

    #[test]
    fn two_entries_walk() {
        let z1 = deflate_zlib(b"one");
        let z2 = deflate_zlib(b"two");
        let mut p = b"PACK".to_vec();
        p.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 2]);
        p.push((3 << 4) | 3);
        p.extend_from_slice(&z1);
        p.push((2 << 4) | 3);
        p.extend_from_slice(&z2);
        p.extend_from_slice(&[0; 20]);
        let pk = parse(&p).unwrap();
        assert_eq!(pk.entries.len(), 2);
        assert_eq!(type_name(pk.entries[1].ty), "tree");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"PACK").is_none());
        let mut bad = pack_one(b"x", 3);
        bad[5] = 9; // version 9
        assert!(parse(&bad).is_none());
        let mut bad2 = pack_one(b"x", 3);
        bad2.truncate(bad2.len() - 21); // missing trailer
        assert!(parse(&bad2).is_none());
        let mut bad3 = pack_one(b"xxxx", 3);
        bad3[12] = (5 << 4) | 4; // reserved type 5
        assert!(parse(&bad3).is_none());
    }

    #[test]
    fn idx_v2_lookup() {
        let mut i = vec![0xFF, b't', b'O', b'c', 0, 0, 0, 2];
        // fanout[k] = #names with first byte ≤ k → all 1 for a byte-0 name
        i.extend_from_slice(&[0, 0, 0, 1]);
        for _ in 0..255 {
            i.extend_from_slice(&[0, 0, 0, 1]);
        }
        let mut name = [0u8; 20];
        name[19] = 0x42;
        i.extend_from_slice(&name);
        i.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // crc
        i.extend_from_slice(&[0, 0, 0x05, 0x00]); // offset 1280
        i.extend_from_slice(&[0xAA; 20]); // pack sha
        i.extend_from_slice(&[0xBB; 20]); // idx sha
        let idx = parse_idx(&i).unwrap();
        assert_eq!(idx.names.len(), 1);
        assert_eq!(find(&idx, &name), Some(1280));
        assert_eq!(offset_of(&idx, 0), Some(1280));
        assert_eq!(offset_of(&idx, 1), None); // out of range
        let mut miss = name;
        miss[19] = 0x41;
        assert_eq!(find(&idx, &miss), None);
        assert_eq!(idx.pack_sha, [0xAA; 20]);
    }
}

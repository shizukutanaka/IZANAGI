//! Windows Event Log `.evtx` envelope — the `ElfFile`/`ElfChnk`
//! layout documented by the libyal `evtx-kb` spec.
//!
//! File header (4096 bytes): `ElfFile\0` magic, u64 first/last
//! chunk numbers, u64 next record identifier, u32 header size
//! (128), u16 major/minor, u16 header block size (4096), u16 chunk
//! count, and a CRC32 over the first 120 bytes at +124.
//! Chunks live at `0x1000 + n * 0x10000`, each starting `ElfChnk\0`.
//!
//! ```
//! use izanagi_kit::evtx::{parse, chunks, CHUNK_SIZE};
//! let mut d = vec![0u8; 0x1000 + CHUNK_SIZE];
//! d[0..8].copy_from_slice(b"ElfFile\0");
//! d[8..16].copy_from_slice(&0u64.to_le_bytes()); // first chunk
//! d[16..24].copy_from_slice(&0u64.to_le_bytes()); // last chunk
//! d[24..32].copy_from_slice(&1u64.to_le_bytes()); // next record
//! d[32..36].copy_from_slice(&128u32.to_le_bytes()); // hdr size
//! d[36..38].copy_from_slice(&1u16.to_le_bytes()); // minor
//! d[38..40].copy_from_slice(&3u16.to_le_bytes()); // major
//! d[40..42].copy_from_slice(&4096u16.to_le_bytes()); // block
//! d[42..44].copy_from_slice(&1u16.to_le_bytes()); // chunks
//! d[0x1000..0x1008].copy_from_slice(b"ElfChnk\0");
//! let e = parse(&d).unwrap();
//! assert_eq!(e.chunks, 1);
//! assert_eq!(chunks(&d, &e).len(), 1);
//! ```

/// Header block size; chunk 0 starts here.
pub const HEADER_SIZE: usize = 0x1000;
/// Every chunk is exactly 64 KiB.
pub const CHUNK_SIZE: usize = 0x10000;
/// File magic.
pub const MAGIC: &[u8; 8] = b"ElfFile\0";
/// Chunk magic.
pub const CHUNK_MAGIC: &[u8; 8] = b"ElfChnk\0";

/// Parsed file header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evtx {
    /// First chunk number.
    pub first_chunk: u64,
    /// Last chunk number.
    pub last_chunk: u64,
    /// Next record identifier.
    pub next_record: u64,
    /// `u16` major version at +38 (expected 3).
    pub major: u16,
    /// `u16` minor version at +36 (expected 1).
    pub minor: u16,
    /// Declared chunk count at +42.
    pub chunks: u16,
}

/// A chunk's position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// File offset (`HEADER_SIZE + i * CHUNK_SIZE`).
    pub at: usize,
}

fn u16s(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32s(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64s(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Parse the `ElfFile` header. `None` on wrong magic, truncated
/// header, non-128 header size, or a non-4096 block size.
pub fn parse(d: &[u8]) -> Option<Evtx> {
    if d.get(0..8)? != MAGIC {
        return None;
    }
    if d.len() < HEADER_SIZE {
        return None;
    }
    if u32s(d, 32)? != 128 || u16s(d, 40)? != HEADER_SIZE as u16 {
        return None;
    }
    Some(Evtx {
        first_chunk: u64s(d, 8)?,
        last_chunk: u64s(d, 16)?,
        next_record: u64s(d, 24)?,
        minor: u16s(d, 36)?,
        major: u16s(d, 38)?,
        chunks: u16s(d, 42)?,
    })
}

/// Walk chunk headers — `ElfChnk\0` at each 64 KiB boundary —
/// stopping at the first hole. Never returns more than
/// `e.chunks` entries.
pub fn chunks(d: &[u8], e: &Evtx) -> Vec<Chunk> {
    let mut out = Vec::new();
    for i in 0..e.chunks as usize {
        let at = HEADER_SIZE + i * CHUNK_SIZE;
        if d.get(at..at + 8) != Some(CHUNK_MAGIC) {
            break;
        }
        out.push(Chunk { at });
    }
    out
}

/// CRC32 (IEEE, reflected) — EVTx headers/chunks use it; computed
/// table-free to stay dependency-free.
pub fn crc32(d: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
        }
    }
    !c
}

/// The stored header CRC32 (bytes 124..128 cover bytes 0..120).
pub fn header_crc(d: &[u8]) -> Option<u32> {
    u32s(d, 124)
}

/// Verify the header CRC.
pub fn crc_ok(d: &[u8]) -> Option<bool> {
    Some(header_crc(d)? == crc32(d.get(0..120)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; HEADER_SIZE + 2 * CHUNK_SIZE];
        d[0..8].copy_from_slice(MAGIC);
        d[8..16].copy_from_slice(&0u64.to_le_bytes());
        d[16..24].copy_from_slice(&1u64.to_le_bytes());
        d[24..32].copy_from_slice(&42u64.to_le_bytes());
        d[32..36].copy_from_slice(&128u32.to_le_bytes());
        d[36..38].copy_from_slice(&1u16.to_le_bytes());
        d[38..40].copy_from_slice(&3u16.to_le_bytes());
        d[40..42].copy_from_slice(&4096u16.to_le_bytes());
        d[42..44].copy_from_slice(&2u16.to_le_bytes());
        let c = crc32(&d[0..120]);
        d[124..128].copy_from_slice(&c.to_le_bytes());
        d[0x1000..0x1008].copy_from_slice(CHUNK_MAGIC);
        d[0x11000..0x11008].copy_from_slice(CHUNK_MAGIC);
        d
    }

    #[test]
    fn header_and_chunks() {
        let d = fixture();
        let e = parse(&d).unwrap();
        assert_eq!(e.major, 3);
        assert_eq!(e.chunks, 2);
        assert_eq!(e.next_record, 42);
        assert_eq!(header_crc(&d), Some(crc32(d.get(0..120).unwrap())));
        assert_eq!(crc_ok(&d), Some(true));
        let cs = chunks(&d, &e);
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].at, 0x1000);
        assert_eq!(cs[1].at, 0x11000);
    }

    #[test]
    fn crc_detects_corruption() {
        let mut d = fixture();
        d[8] ^= 1;
        assert_eq!(crc_ok(&d), Some(false));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[32] = 0; // header size != 128
        assert!(parse(&d2).is_none());
        // chunk hole truncates the walk
        let mut d3 = fixture();
        d3[0x11000] = 0;
        assert_eq!(chunks(&d3, &parse(&d3).unwrap()).len(), 1);
    }
}

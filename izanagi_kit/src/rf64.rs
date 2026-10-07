//! RF64 (EBU Tech 3306) / BW64 RIFF-family header parsing.
//!
//! Layout: `RF64`(or `BW64`) + u32 `0xFFFFFFFF` + `WAVE` + `ds64` chunk
//! (riff_size u64, data_size u64, sample_count u64, table_len u32).
//!
//! ```
//! use izanagi_kit::rf64::parse;
//!
//! let mut f = vec![0u8; 48];
//! f[..4].copy_from_slice(b"RF64");
//! f[4..8].copy_from_slice(&[0xFF; 4]);
//! f[8..12].copy_from_slice(b"WAVE");
//! f[12..16].copy_from_slice(b"ds64");
//! f[16..20].copy_from_slice(&[28, 0, 0, 0]);   // ds64 chunk len 28
//! f[20..28].copy_from_slice(&[0x80, 0, 0, 0, 0, 0, 0, 0]); // riff size
//! let r = parse(&f).unwrap();
//! assert_eq!(r.riff_size, 128);
//! ```

/// Parsed RF64/BW64 + ds64 header.
#[derive(Debug)]
pub struct Rf64 {
    /// True when the magic is `BW64` rather than `RF64`.
    pub bw64: bool,
    /// RIFF payload size (u64 from ds64).
    pub riff_size: u64,
    /// `data` chunk size.
    pub data_size: u64,
    /// Total sample count.
    pub sample_count: u64,
    /// Chunk-table entry count.
    pub table_len: u32,
    /// Offset of the first chunk after the ds64 header (`ds64` + 28 + table).
    pub chunks_at: usize,
}

/// Parses an RF64/BW64 file. Requires the `ds64` chunk as the first WAVE
/// chunk (EBU mandates it).
pub fn parse(d: &[u8]) -> Option<Rf64> {
    if d.len() < 40 || &d[8..12] != b"WAVE" {
        return None;
    }
    let bw64 = match &d[..4] {
        b"RF64" => false,
        b"BW64" => true,
        _ => return None,
    };
    if d[4..8] != [0xFF; 4] {
        return None;
    }
    if &d[12..16] != b"ds64" {
        return None;
    }
    let ds64_len = (d[16] as usize)
        | ((d[17] as usize) << 8)
        | ((d[18] as usize) << 16)
        | ((d[19] as usize) << 24);
    if ds64_len < 28 || 20 + ds64_len > d.len() {
        return None;
    }
    let u64le = |o: usize| {
        let mut v: u64 = 0;
        for i in 0..8 {
            v |= (d[o + i] as u64) << (i * 8);
        }
        v
    };
    let table_len =
        (d[40] as u32) | ((d[41] as u32) << 8) | ((d[42] as u32) << 16) | ((d[43] as u32) << 24);
    Some(Rf64 {
        bw64,
        riff_size: u64le(20),
        data_size: u64le(28),
        sample_count: u64le(36),
        table_len,
        chunks_at: 20 + ds64_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(magic: &[u8; 4], riff: u64) -> Vec<u8> {
        let mut f = vec![0u8; 48];
        f[..4].copy_from_slice(magic);
        f[4..8].copy_from_slice(&[0xFF; 4]);
        f[8..12].copy_from_slice(b"WAVE");
        f[12..16].copy_from_slice(b"ds64");
        f[16..20].copy_from_slice(&[28, 0, 0, 0]);
        f[20..28].copy_from_slice(&[
            (riff & 0xFF) as u8,
            (riff >> 8) as u8,
            (riff >> 16) as u8,
            (riff >> 24) as u8,
            (riff >> 32) as u8,
            (riff >> 40) as u8,
            (riff >> 48) as u8,
            (riff >> 56) as u8,
        ]);
        f[28..36].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 64]); // data size 2^62
        f
    }

    #[test]
    fn parses() {
        let r = parse(&file(b"RF64", 0x1_0000_0005)).unwrap();
        assert!(!r.bw64);
        assert_eq!(r.riff_size, 0x1_0000_0005);
        assert_eq!(r.data_size, 1 << 62);
        assert_eq!(r.chunks_at, 48);
        let r = parse(&file(b"BW64", 7)).unwrap();
        assert!(r.bw64);
    }

    #[test]
    fn rejects() {
        assert!(parse(&file(b"RIFX", 8)).is_none());
        let mut f = file(b"RF64", 8);
        f[4] = 0x10; // not FFFFFFFF
        assert!(parse(&f).is_none());
        let mut f = file(b"RF64", 8);
        f[12] = b'x';
        assert!(parse(&f).is_none());
    }
}

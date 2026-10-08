//! SAC (Seismic Analysis Code) binary file header sniffing.
//!
//! Layout: 70 × f32 (280 B), then i32s starting with the nzyear block;
//! npts at byte 316, nvhdr (header version, 6) at byte 304, data at 632.
//! Both endiannesses are detected via the version field.
//!
//! ```
//! use izanagi_kit::sac::parse;
//!
//! // little-endian SAC: nvhdr=6 @304, npts=100 @316, data 632+100*4
//! let mut h = vec![0u8; 632 + 400];
//! h[304..308].copy_from_slice(&[6, 0, 0, 0]);
//! h[316..320].copy_from_slice(&[100, 0, 0, 0]);
//! h[280..284].copy_from_slice(&[0xE9, 0x07, 0, 0]); // nzyear 2025
//! let s = parse(&h).unwrap();
//! assert_eq!(s.npts, 100);
//! assert!(s.little_endian);
//! ```

/// Parsed SAC header summary.
#[derive(Debug)]
pub struct Sac {
    /// True when the file is little-endian (x86 style; many SACs are).
    pub little_endian: bool,
    /// Header version field (npts region); canonical = 6.
    pub nvhdr: i32,
    /// Reference year.
    pub nzyear: i32,
    /// Reference day of year.
    pub nzjday: i32,
    /// Number of data points.
    pub npts: i32,
    /// Sampling interval (`delta`), kept as raw f32 bits — no floats in kit.
    pub delta_bits: u32,
    /// Begin time (`b`), raw f32 bits.
    pub b_bits: u32,
    /// Offset of the sample data (632).
    pub data_at: usize,
}

/// Parses a SAC file: header must be ≥632 B, `nvhdr` must be 1..=7
/// read consistently in one endianness, and `npts` must fit the file.
pub fn parse(d: &[u8]) -> Option<Sac> {
    if d.len() < 632 {
        return None;
    }
    fn le32(d: &[u8], o: usize) -> u32 {
        (d[o] as u32)
            | ((d[o + 1] as u32) << 8)
            | ((d[o + 2] as u32) << 16)
            | ((d[o + 3] as u32) << 24)
    }
    fn be32(d: &[u8], o: usize) -> u32 {
        ((d[o] as u32) << 24)
            | ((d[o + 1] as u32) << 16)
            | ((d[o + 2] as u32) << 8)
            | (d[o + 3] as u32)
    }
    // nvhdr @304 decides endianness
    let le_hdr = le32(d, 304) as i32;
    let be_hdr = be32(d, 304) as i32;
    let little = if (1..=7).contains(&le_hdr) {
        true
    } else if (1..=7).contains(&be_hdr) {
        false
    } else {
        return None;
    };
    let g = |o: usize| {
        if little {
            le32(d, o)
        } else {
            be32(d, o)
        }
    };
    let g32 = |o: usize| g(o) as i32;
    let npts = g32(316);
    if npts <= 0 {
        return None;
    }
    // data follows the 632-byte header; each sample is 4 bytes
    let need = 632usize.checked_add(npts as usize * 4)?;
    if need > d.len() {
        return None;
    }
    Some(Sac {
        little_endian: little,
        nvhdr: g32(304),
        nzyear: g32(280),
        nzjday: g32(284),
        npts,
        delta_bits: g(0),
        b_bits: g(20),
        data_at: 632,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr_le() -> Vec<u8> {
        let mut h = vec![0u8; 632 + 400];
        h[0..4].copy_from_slice(&[0, 0, 0x80, 0x3F]); // delta = 1.0 bits (LE)
        h[280..284].copy_from_slice(&[0xE9, 0x07, 0, 0]); // 2025
        h[284..288].copy_from_slice(&[0x64, 0, 0, 0]); // doy 100
        h[304..308].copy_from_slice(&[6, 0, 0, 0]);
        h[316..320].copy_from_slice(&[100, 0, 0, 0]);
        h
    }

    #[test]
    fn parses_le() {
        let s = parse(&hdr_le()).unwrap();
        assert!(s.little_endian);
        assert_eq!(s.nvhdr, 6);
        assert_eq!(s.nzyear, 2025);
        assert_eq!(s.nzjday, 100);
        assert_eq!(s.npts, 100);
        assert_eq!(s.delta_bits, 0x3F800000);
        assert_eq!(s.data_at, 632);
    }

    #[test]
    fn parses_be() {
        let mut h = vec![0u8; 632 + 8];
        h[304..308].copy_from_slice(&[0, 0, 0, 6]);
        h[316..320].copy_from_slice(&[0, 0, 0, 2]);
        let s = parse(&h).unwrap();
        assert!(!s.little_endian);
        assert_eq!(s.npts, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0; 631]).is_none());
        let mut h = hdr_le();
        h[304..308].copy_from_slice(&[9, 0, 0, 0]); // version 9 unknown
        assert!(parse(&h).is_none());
        let mut h = hdr_le();
        h[316..320].copy_from_slice(&[0xFF, 0xFF, 0x7F, 0x7F]); // huge npts
        assert!(parse(&h).is_none());
    }
}

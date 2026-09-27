//! MRC / CCP4 electron-microscopy map — 1024-byte header.
//!
//! Grid dims `nx/ny/nz` @0, `mode` @12, start indices, `mapc/r/s` axis
//! order @16..28, cell geometry as raw u32 bit patterns @40..64 (float
//! text is not interpreted), `ispg`, `nsymbt`, `MAP `/`DA` magic.
//!
//! ```
//! use izanagi_kit::mrc::parse;
//!
//! let mut h = vec![0u8; 1024];
//! h[0..4].copy_from_slice(&64u32.to_le_bytes());
//! h[4..8].copy_from_slice(&64u32.to_le_bytes());
//! h[8..12].copy_from_slice(&32u32.to_le_bytes());
//! h[16..20].copy_from_slice(&1u32.to_le_bytes());
//! h[20..24].copy_from_slice(&2u32.to_le_bytes());
//! h[24..28].copy_from_slice(&3u32.to_le_bytes());
//! h[208..212].copy_from_slice(b"MAP ");
//! let m = parse(&h).unwrap();
//! assert_eq!((m.nx, m.ny, m.nz), (64, 64, 32));
//! assert!(m.data_at(&h) == Some(1024));
//! ```

fn u32le(d: &[u8], o: usize) -> Option<u32> {
    let b = d.get(o..o + 4)?;
    Some((b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16) | ((b[3] as u32) << 24))
}

/// A parsed MRC header.
#[derive(Clone, Debug)]
pub struct Mrc {
    /// Columns (fastest axis).
    pub nx: u32,
    /// Rows.
    pub ny: u32,
    /// Sections.
    pub nz: u32,
    /// Data mode (0=i8,1=i16,2=f32,6=u16...).
    pub mode: u32,
    /// Axis order `mapc/mapr/maps` (1=x,2=y,3=z).
    pub map_axes: [u32; 3],
    /// Cell dimensions `xlen/ylen/zlen` — raw IEEE-754 bits.
    pub cell_bits: [u32; 3],
    /// Cell angles `alpha/beta/gamma` — raw bits.
    pub cell_angle_bits: [u32; 3],
    /// Space group number.
    pub ispg: u32,
    /// Extended-header bytes following the 1024-byte header.
    pub nsymbt: u32,
}

impl Mrc {
    /// Offset of voxel data (header + extended header).
    pub fn data_at(&self, d: &[u8]) -> Option<u64> {
        let at = 1024u64 + self.nsymbt as u64;
        if at <= d.len() as u64 {
            Some(at)
        } else {
            None
        }
    }
}

/// Parse the 1024-byte MRC header. `None` when too short, dims/mode are
/// zero, or `MAP ` magic is absent.
pub fn parse(d: &[u8]) -> Option<Mrc> {
    if d.len() < 1024 {
        return None;
    }
    let nx = u32le(d, 0)?;
    let ny = u32le(d, 4)?;
    let nz = u32le(d, 8)?;
    let mode = u32le(d, 12)?;
    if nx == 0 || ny == 0 || nz == 0 {
        return None;
    }
    if d.get(208..212) != Some(b"MAP ") {
        return None;
    }
    Some(Mrc {
        nx,
        ny,
        nz,
        mode,
        map_axes: [u32le(d, 16)?, u32le(d, 20)?, u32le(d, 24)?],
        cell_bits: [u32le(d, 40)?, u32le(d, 44)?, u32le(d, 48)?],
        cell_angle_bits: [u32le(d, 52)?, u32le(d, 56)?, u32le(d, 60)?],
        ispg: u32le(d, 88)?,
        nsymbt: u32le(d, 92)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> Vec<u8> {
        let mut h = vec![0u8; 1024];
        h[0..4].copy_from_slice(&4u32.to_le_bytes());
        h[4..8].copy_from_slice(&5u32.to_le_bytes());
        h[8..12].copy_from_slice(&6u32.to_le_bytes());
        h[208..212].copy_from_slice(b"MAP ");
        h
    }

    #[test]
    fn parses() {
        let mut h = hdr();
        h[92..96].copy_from_slice(&8u32.to_le_bytes());
        h.extend_from_slice(&[0u8; 8]);
        let m = parse(&h).unwrap();
        assert_eq!((m.nx, m.ny, m.nz, m.mode), (4, 5, 6, 0));
        assert_eq!(m.data_at(&h), Some(1032));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 100]).is_none());
        let mut h = hdr();
        h[208..212].copy_from_slice(b"XXXX");
        assert!(parse(&h).is_none());
        let mut h2 = hdr();
        h2[0..4].copy_from_slice(&0u32.to_le_bytes());
        assert!(parse(&h2).is_none());
    }
}

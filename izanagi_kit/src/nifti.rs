//! NIfTI-1 header (neuroimaging `.nii` / `.nii.gz`).
//!
//! The 348-byte header opens with `sizeof_hdr` = 348 — read
//! little-endian normally, or byte-swapped for big-endian files.
//! `dim[0]` is the number of dimensions with `dim[1..]` the sizes;
//! `datatype` and `bitpix` give the voxel encoding. Float fields
//! (pixdim, scl, cal, quaternion, qoffset, srow) are exposed as
//! their raw u32 bit patterns — the kit never interprets floats.
//!
//! ```
//! use izanagi_kit::nifti::{parse, HEADER, Kind};
//!
//! let mut d = vec![0u8; HEADER + 4];
//! let put = |d: &mut [u8], at: usize, v: u16| {
//!     d[at] = v as u8; d[at + 1] = (v >> 8) as u8;
//! };
//! let put32 = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = v as u8; d[at + 1] = (v >> 8) as u8;
//!     d[at + 2] = (v >> 16) as u8; d[at + 3] = (v >> 24) as u8;
//! };
//! put32(&mut d, 0, HEADER as u32);
//! put(&mut d, 40, 3);              // 3 dimensions
//! put(&mut d, 42, 256); put(&mut d, 44, 256); put(&mut d, 46, 64);
//! put(&mut d, 70, 8);              // int32
//! put(&mut d, 72, 32);
//! d[344..348].copy_from_slice(b"n+1\0");
//! let n = parse(&d).unwrap();
//! assert_eq!(n.ndim(), 3);
//! assert_eq!(n.dims(), &[256, 256, 64]);
//! assert_eq!(n.kind, Kind::Single);
//! ```

/// Header size in bytes.
pub const HEADER: usize = 348;

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}
fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}
fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}

/// File layout indicated by the magic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `n+1` — single .nii file, data follows the header.
    Single,
    /// `ni1` — header+data split across .hdr/.img pair.
    Pair,
    /// `n+2`/other — NIfTI-2 or unrecognized.
    Other,
}

/// A parsed NIfTI-1 header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nifti {
    /// True when the file is byte-swapped (big-endian).
    pub swapped: bool,
    /// Dimension sizes, `dim[0]` = number of used dims (1-7).
    pub dim: [u16; 8],
    /// Datatype code (4 i16, 8 i32, 16 f32, 64 f64, 256 i8, ...).
    pub datatype: u16,
    /// Bits per voxel.
    pub bitpix: u16,
    /// `pixdim` raw f32 bit patterns.
    pub pixdim_raw: [u32; 8],
    /// `vox_offset` raw f32 bits (352.0 = data right after header).
    pub vox_offset_raw: u32,
    /// `scl_slope` raw f32 bits.
    pub scl_slope_raw: u32,
    /// `scl_inter` raw f32 bits.
    pub scl_inter_raw: u32,
    /// `cal_max`/`cal_min` raw f32 bits.
    pub cal_raw: (u32, u32),
    /// `qform_code` — transform method (0 none, 1 scanner, 2 aligned, ...).
    pub qform_code: u16,
    /// `sform_code` — affine transform method.
    pub sform_code: u16,
    /// File layout from the magic.
    pub kind: Kind,
    /// Free-form description string (descrip, NUL-trimmed).
    pub description: String,
}

impl Nifti {
    /// Number of spatial/time dimensions (`dim[0]`, clamped to 7).
    pub fn ndim(&self) -> usize {
        usize::from(self.dim[0]).min(7)
    }
    /// Used dimension sizes.
    pub fn dims(&self) -> &[u16] {
        &self.dim[1..=self.ndim()]
    }
}

/// Parse a header. Returns `None` when shorter than 348 bytes or
/// `sizeof_hdr` is neither 348 (LE) nor its byte swap (BE).
pub fn parse(d: &[u8]) -> Option<Nifti> {
    if d.len() < HEADER {
        return None;
    }
    let swapped = if le32(d, 0)? == HEADER as u32 {
        false
    } else if be32(d, 0)? == HEADER as u32 {
        true
    } else {
        return None;
    };
    let r16 = if swapped { be16 } else { le16 };
    let r32 = if swapped { be32 } else { le32 };
    let mut dim = [0u16; 8];
    for (i, s) in dim.iter_mut().enumerate() {
        *s = r16(d, 40 + i * 2)?;
    }
    let mut pixdim_raw = [0u32; 8];
    for (i, s) in pixdim_raw.iter_mut().enumerate() {
        *s = r32(d, 76 + i * 4)?;
    }
    let kind = match d.get(344..348)? {
        b"n+1\0" => Kind::Single,
        b"ni1\0" => Kind::Pair,
        _ => Kind::Other,
    };
    let desc_bytes = d.get(148..228)?;
    let end = desc_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(desc_bytes.len());
    Some(Nifti {
        swapped,
        dim,
        datatype: r16(d, 70)?,
        bitpix: r16(d, 72)?,
        pixdim_raw,
        vox_offset_raw: r32(d, 108)?,
        scl_slope_raw: r32(d, 112)?,
        scl_inter_raw: r32(d, 116)?,
        cal_raw: (r32(d, 124)?, r32(d, 128)?),
        qform_code: r16(d, 252)?,
        sform_code: r16(d, 254)?,
        kind,
        description: String::from_utf8_lossy(&desc_bytes[..end]).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 512];
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
        };
        let w32 = |d: &mut [u8], o: usize, v: u32| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
            d[o + 2] = (v >> 16) as u8;
            d[o + 3] = (v >> 24) as u8;
        };
        w32(&mut d, 0, 348);
        w16(&mut d, 40, 4);
        w16(&mut d, 42, 64);
        w16(&mut d, 44, 64);
        w16(&mut d, 46, 30);
        w16(&mut d, 48, 100); // 4th dim: time frames
        w16(&mut d, 70, 16); // f32
        w16(&mut d, 72, 32);
        w32(&mut d, 76, 0); // pixdim[0]
        w32(&mut d, 80, 0x4000_0000); // 2.0f
        w32(&mut d, 84, 0x4000_0000);
        w32(&mut d, 88, 0x4040_0000); // 3.0f
        w32(&mut d, 108, 0x43b0_0000); // vox_offset = 352.0
        w16(&mut d, 252, 1);
        w16(&mut d, 254, 4);
        d[148..152].copy_from_slice(b"TEST");
        d[344..348].copy_from_slice(b"n+1\0");
        d
    }

    #[test]
    fn fields_decode() {
        let d = fixture();
        let n = parse(&d).unwrap();
        assert!(!n.swapped);
        assert_eq!(n.ndim(), 4);
        assert_eq!(n.dims(), &[64, 64, 30, 100]);
        assert_eq!(n.datatype, 16);
        assert_eq!(n.bitpix, 32);
        assert_eq!(n.pixdim_raw[1], 0x4000_0000);
        assert_eq!(n.vox_offset_raw, 0x43b0_0000);
        assert_eq!(n.qform_code, 1);
        assert_eq!(n.sform_code, 4);
        assert_eq!(n.kind, Kind::Single);
        assert_eq!(n.description, "TEST");
    }

    #[test]
    fn big_endian_header() {
        let mut d = fixture();
        // swap the byte order of the fields we wrote
        d.swap(0, 3);
        d.swap(1, 2); // sizeof_hdr now BE
        d.swap(40, 41);
        d.swap(42, 43);
        d.swap(44, 45);
        d.swap(46, 47);
        d.swap(48, 49);
        d.swap(70, 71);
        d.swap(72, 73);
        for i in 0..8 {
            let o = 76 + i * 4;
            d.swap(o, o + 3);
            d.swap(o + 1, o + 2);
        }
        d.swap(108, 111);
        d.swap(109, 110);
        d.swap(252, 253);
        d.swap(254, 255);
        let n = parse(&d).unwrap();
        assert!(n.swapped);
        assert_eq!(n.dims(), &[64, 64, 30, 100]);
        assert_eq!(n.pixdim_raw[3], 0x4040_0000);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 100]).is_none());
        let mut d = fixture();
        d[0] = 9; // wrong sizeof_hdr both ways
        assert!(parse(&d).is_none());
    }
}

//! Analyze 7.5 `.hdr` header (Mayo Clinic Analyze, AVW): exactly 348
//! bytes — `sizeof_hdr` u32 = 348 (read both ways to pick the file's
//! byte order), `data_type`/`db_name`/`descrip` text, `dim` i16\[8\],
//! `datatype` i16 (2=u8 4=i16 8=i32 64=f64), `bitpix` i16,
//! `glmax`/`glmin` i32, `orient`/`regular` bytes. Float fields
//! (`pixdim`, `vox_offset`, …) are exposed as raw bits — the crate
//! never materializes `f32`/`f64`.
//!
//! ```
//! let mut h = vec![0u8; 348];
//! h[0..4].copy_from_slice(&348u32.to_le_bytes());
//! h[70..72].copy_from_slice(&4i16.to_le_bytes()); // datatype i16
//! h[72..74].copy_from_slice(&16i16.to_le_bytes()); // bitpix
//! h[40..42].copy_from_slice(&3i16.to_le_bytes()); // dim[0] = 3
//! let a = izanagi_kit::analyze::parse(&h).unwrap();
//! assert_eq!(a.datatype, 4);
//! assert_eq!(a.dims[0], 3);
//! ```

use std::string::String;

/// Byte order the header was written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    /// Little-endian header.
    Little,
    /// Big-endian header.
    Big,
}

/// A parsed Analyze 7.5 header.
#[derive(Clone, Debug)]
pub struct Analyze {
    /// Detected byte order.
    pub endian: Endian,
    /// `data_type` string (usually a date).
    pub data_type: String,
    /// `db_name` string.
    pub db_name: String,
    /// `dim` array — element 0 is the dimension count, the rest are sizes.
    pub dims: [i16; 8],
    /// `datatype` code (2=u8, 4=i16, 8=i32, 16=f32 bits, 64=f64 bits).
    pub datatype: i16,
    /// Bits per voxel.
    pub bitpix: i16,
    /// Raw bits of `pixdim[8]` (f32 grid spacing — kept as bits).
    pub pixdim_bits: [u32; 8],
    /// Raw bits of `vox_offset` (f32).
    pub vox_offset_bits: u32,
    /// `glmax` global max.
    pub glmax: i32,
    /// `glmin` global min.
    pub glmin: i32,
    /// `descrip` free text.
    pub descrip: String,
    /// `orient` byte (0 transverse, 1 coronal, 2 sagittal).
    pub orient: u8,
}

fn trim(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim_end().to_string()
}

fn i16s(d: &[u8], e: Endian) -> i16 {
    let (a, b) = (u16::from(d[0]), u16::from(d[1]));
    let v = match e {
        Endian::Little => a | (b << 8),
        Endian::Big => (a << 8) | b,
    };
    v as i16
}
fn i32s(d: &[u8], e: Endian) -> i32 {
    let mut v = 0u32;
    match e {
        Endian::Little => {
            for i in (0..4).rev() {
                v = v << 8 | u32::from(d[i]);
            }
        }
        Endian::Big => {
            for b in &d[..4] {
                v = v << 8 | u32::from(*b);
            }
        }
    }
    v as i32
}

/// Parse an Analyze 7.5 header; `None` unless exactly 348 bytes and
/// `sizeof_hdr` == 348 in one byte order.
pub fn parse(d: &[u8]) -> Option<Analyze> {
    if d.len() != 348 {
        return None;
    }
    let le = u32::from_le_bytes(d[..4].try_into().ok()?);
    let be = {
        let mut v = 0u32;
        for &b in &d[..4] {
            v = v << 8 | u32::from(b);
        }
        v
    };
    let endian = if le == 348 {
        Endian::Little
    } else if be == 348 {
        Endian::Big
    } else {
        return None;
    };
    let mut dims = [0i16; 8];
    for i in 0..8 {
        dims[i] = i16s(&d[40 + i * 2..], endian);
    }
    if dims[0] < 0 || dims[0] > 7 {
        return None;
    }
    let mut pixdim_bits = [0u32; 8];
    for i in 0..8 {
        let raw = i32s(&d[76 + i * 4..], endian);
        pixdim_bits[i] = raw as u32;
    }
    Some(Analyze {
        endian,
        data_type: trim(&d[4..14]),
        db_name: trim(&d[14..32]),
        dims,
        datatype: i16s(&d[70..], endian),
        bitpix: i16s(&d[72..], endian),
        pixdim_bits,
        vox_offset_bits: i32s(&d[108..], endian) as u32,
        glmax: i32s(&d[140..], endian),
        glmin: i32s(&d[144..], endian),
        descrip: trim(&d[148..228]),
        orient: d[252],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(e: Endian) -> Vec<u8> {
        let mut h = vec![0u8; 348];
        let put32 = |h: &mut Vec<u8>, off: usize, v: u32| match e {
            Endian::Little => h[off..off + 4].copy_from_slice(&v.to_le_bytes()),
            Endian::Big => {
                h[off] = (v >> 24) as u8;
                h[off + 1] = (v >> 16) as u8;
                h[off + 2] = (v >> 8) as u8;
                h[off + 3] = v as u8;
            }
        };
        put32(&mut h, 0, 348);
        h
    }

    #[test]
    fn le_header() {
        let mut h = hdr(Endian::Little);
        h[4..12].copy_from_slice(b"2024-01-");
        h[70..72].copy_from_slice(&[8, 0]); // datatype i32
        h[72..74].copy_from_slice(&[32, 0]); // bitpix
        h[40..42].copy_from_slice(&[4, 0]); // dim[0]
        h[44..46].copy_from_slice(&[64, 0]); // dim[2]=64
        h[140..144].copy_from_slice(&[0, 0, 1, 0]); // glmax 65536
        h[148..153].copy_from_slice(b"brain");
        h[252] = 2;
        let a = parse(&h).unwrap();
        assert_eq!(a.endian, Endian::Little);
        assert_eq!(a.data_type, "2024-01-");
        assert_eq!(a.datatype, 8);
        assert_eq!(a.bitpix, 32);
        assert_eq!(a.dims[0], 4);
        assert_eq!(a.dims[2], 64);
        assert_eq!(a.glmax, 65536);
        assert_eq!(a.descrip, "brain");
        assert_eq!(a.orient, 2);
    }

    #[test]
    fn be_header() {
        let mut h = hdr(Endian::Big);
        h[70] = 0;
        h[71] = 2;
        h[40] = 0;
        h[41] = 3;
        let a = parse(&h).unwrap();
        assert_eq!(a.endian, Endian::Big);
        assert_eq!(a.datatype, 2);
        assert_eq!(a.dims[0], 3);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&vec![0u8; 347]).is_none());
        let mut h = vec![0u8; 348]; // sizeof_hdr = 0
        assert!(parse(&h).is_none());
        h[0..4].copy_from_slice(&348u32.to_le_bytes());
        h[40..42].copy_from_slice(&[9, 0]); // dims[0] = 9 > 7
        assert!(parse(&h).is_none());
    }
}

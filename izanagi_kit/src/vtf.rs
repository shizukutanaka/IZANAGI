//! Valve Texture Format (VTF) header.
//!
//! A VTF opens with `VTF\0`, two u32 version numbers (7.0–7.5)
//! and an 80-byte header: texture size, flags, frames,
//! reflectivity (kept as raw u32 bits), bump-map scale, high-res
//! image format + mipmap count, low-res thumbnail format + size,
//! and (7.2+) depth. `parse` requires `header_size` ≥ 80.
//!
//! ```
//! use izanagi_kit::vtf::{parse, MAGIC};
//!
//! let mut d = vec![0u8; 80];
//! d[..4].copy_from_slice(&MAGIC);
//! let w = |d: &mut [u8], o: usize, v: u32| {
//!     for i in 0..4 { d[o + i] = (v >> (i * 8)) as u8; }
//! };
//! w(&mut d, 4, 7);            // major
//! w(&mut d, 8, 5);            // minor
//! w(&mut d, 12, 80);          // header size
//! d[16] = 0; d[17] = 4;       // width 1024
//! d[18] = 0; d[19] = 4;       // height
//! w(&mut d, 52, 13);          // DXT5 image format
//! d[56] = 4;                  // mipmaps
//! let t = parse(&d).unwrap();
//! assert_eq!(t.width, 1024);
//! assert_eq!(t.version(), (7, 5));
//! ```

/// `VTF\0` signature.
pub const MAGIC: [u8; 4] = *b"VTF\0";
/// Fixed v7.x header size.
pub const HEADER: usize = 80;

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

/// A parsed VTF header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vtf {
    /// Version major/minor.
    pub version_major: u32,
    /// Minor.
    pub version_minor: u32,
    /// Declared header size (≥ 80).
    pub header_size: u32,
    /// Texture width/height.
    pub width: u16,
    /// Height.
    pub height: u16,
    /// `flags` bitfield (point sampling, trilinear, envmap, etc.).
    pub flags: u32,
    /// Animation frames.
    pub frames: u16,
    /// First frame index.
    pub first_frame: u16,
    /// Reflectivity rgb — raw f32 bits.
    pub reflectivity_raw: [u32; 3],
    /// Bump-map scale — raw f32 bits.
    pub bump_scale_raw: u32,
    /// `IMAGE_FORMAT_*` of the full-res data.
    pub image_format: u32,
    /// Mipmap count.
    pub mipmaps: u8,
    /// Low-res thumbnail format (`IMAGE_FORMAT_*` or 0xFFFFFFFF).
    pub low_res_format: u32,
    /// Low-res width/height.
    pub low_res_width: u8,
    /// Low-res height.
    pub low_res_height: u8,
    /// Texture depth (v7.2+; 0 otherwise).
    pub depth: u16,
}

impl Vtf {
    /// Version tuple.
    pub fn version(&self) -> (u32, u32) {
        (self.version_major, self.version_minor)
    }
    /// Byte offset where image data begins.
    pub fn data_at(&self) -> u64 {
        u64::from(self.header_size)
    }
    /// True when a low-res thumbnail block exists between the
    /// header and image data.
    pub fn has_thumbnail(&self) -> bool {
        self.low_res_format != 0xFFFF_FFFF && self.low_res_width > 0 && self.low_res_height > 0
    }
}

/// Parse a VTF header. Returns `None` on bad magic or a header
/// size below 80.
pub fn parse(d: &[u8]) -> Option<Vtf> {
    let h = d.get(..HEADER)?;
    if h.get(..4)? != MAGIC {
        return None;
    }
    let header_size = le32(h, 12)?;
    if header_size < HEADER as u32 || header_size as usize > d.len() {
        return None;
    }
    Some(Vtf {
        version_major: le32(h, 4)?,
        version_minor: le32(h, 8)?,
        header_size,
        width: le16(h, 16)?,
        height: le16(h, 18)?,
        flags: le32(h, 20)?,
        frames: le16(h, 24)?,
        first_frame: le16(h, 26)?,
        reflectivity_raw: [le32(h, 32)?, le32(h, 36)?, le32(h, 40)?],
        bump_scale_raw: le32(h, 48)?,
        image_format: le32(h, 52)?,
        mipmaps: h.get(56).copied()?,
        low_res_format: le32(h, 57)?,
        low_res_width: h.get(61).copied()?,
        low_res_height: h.get(62).copied()?,
        depth: le16(h, 63)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 256];
        d[..4].copy_from_slice(&MAGIC);
        let w = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        w(&mut d, 4, 7);
        w(&mut d, 8, 3);
        w(&mut d, 12, 80);
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
        };
        w16(&mut d, 16, 1024);
        w16(&mut d, 18, 512);
        w(&mut d, 20, 0x0000_2008); // flags: trilinear + eightbit
        w16(&mut d, 24, 5);
        w16(&mut d, 26, 1);
        w(&mut d, 32, 0x3E80_0000); // refl.x = 0.25
        w(&mut d, 48, 0x3F80_0000); // bump 1.0
        w(&mut d, 52, 13); // DXT5
        d[56] = 11;
        w(&mut d, 57, 1); // BGRA8888 thumbnail
        d[61] = 16;
        d[62] = 16;
        w16(&mut d, 63, 1); // depth
        d
    }

    #[test]
    fn fields() {
        let t = parse(&fixture()).unwrap();
        assert_eq!(t.version(), (7, 3));
        assert_eq!(t.header_size, 80);
        assert_eq!(t.width, 1024);
        assert_eq!(t.height, 512);
        assert_eq!(t.flags, 0x2008);
        assert_eq!(t.frames, 5);
        assert_eq!(t.first_frame, 1);
        assert_eq!(t.reflectivity_raw[0], 0x3E80_0000);
        assert_eq!(t.bump_scale_raw, 0x3F80_0000);
        assert_eq!(t.image_format, 13);
        assert_eq!(t.mipmaps, 11);
        assert!(t.has_thumbnail());
        assert_eq!(t.low_res_width, 16);
        assert_eq!(t.depth, 1);
        assert_eq!(t.data_at(), 80);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 40]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[12] = 60; // header_size < 80
        assert!(parse(&d2).is_none());
        let mut d3 = fixture();
        d3[12] = 255; // header past buffer
        d3[13] = 255;
        assert!(parse(&d3).is_none());
    }
}

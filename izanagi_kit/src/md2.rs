//! Quake II MD2 model file ("IDP2" version 8).
//!
//! The 68-byte header carries counts plus a six-slot offset table
//! (skins, texture coords, triangles, frames, GL commands,
//! end). `parse` bounds-checks every offset against the file so a
//! table never escapes the buffer.
//!
//! ```
//! use izanagi_kit::md2::{parse, MAGIC};
//!
//! let mut d = vec![0u8; 68];
//! d[..4].copy_from_slice(&MAGIC);
//! let w = |d: &mut [u8], o: usize, v: u32| {
//!     for i in 0..4 { d[o + i] = (v >> (i * 8)) as u8; }
//! };
//! w(&mut d, 4, 8);            // version
//! w(&mut d, 16, 64);          // frame size
//! w(&mut d, 24, 12);          // num_xyz vertices
//! w(&mut d, 64, 68);          // ofs_end = file length
//! let m = parse(&d).unwrap();
//! assert_eq!(m.num_xyz, 12);
//! assert_eq!(m.ofs_end, 68);
//! ```

/// `IDP2` magic.
pub const MAGIC: [u8; 4] = *b"IDP2";
/// Expected version.
pub const VERSION: u32 = 8;
/// Header size.
pub const HEADER: usize = 68;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// A parsed MD2 header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Md2 {
    /// Version (8).
    pub version: u32,
    /// Skin width/height.
    pub skin_width: u32,
    /// Skin height.
    pub skin_height: u32,
    /// Bytes per frame.
    pub frame_size: u32,
    /// Skin count.
    pub num_skins: u32,
    /// Vertex count (`num_xyz`).
    pub num_xyz: u32,
    /// Texture-coordinate count.
    pub num_st: u32,
    /// Triangle count.
    pub num_tris: u32,
    /// GL command stream words.
    pub num_glcmds: u32,
    /// Frame count.
    pub num_frames: u32,
    /// Offset of the skin name table.
    pub ofs_skins: u32,
    /// Offset of the st table.
    pub ofs_st: u32,
    /// Offset of the triangle table.
    pub ofs_tris: u32,
    /// Offset of the frame block.
    pub ofs_frames: u32,
    /// Offset of the GL command stream.
    pub ofs_glcmds: u32,
    /// Offset just past the end of the file.
    pub ofs_end: u32,
}

impl Md2 {
    /// Byte offset of a named section, or `None` for a bad index.
    /// Index: 0 skins, 1 st, 2 tris, 3 frames, 4 glcmds.
    pub fn section_at(&self, i: usize) -> Option<u32> {
        [
            self.ofs_skins,
            self.ofs_st,
            self.ofs_tris,
            self.ofs_frames,
            self.ofs_glcmds,
        ]
        .get(i)
        .copied()
    }
    /// Section name for `section_at` indices.
    pub fn section_name(i: usize) -> Option<&'static str> {
        ["skins", "st", "tris", "frames", "glcmds"].get(i).copied()
    }
}

/// Parse an MD2 header. Returns `None` on bad magic/version or a
/// section offset that escapes the buffer (`ofs_end` aside, which
/// must equal or sit below the file length).
pub fn parse(d: &[u8]) -> Option<Md2> {
    let h = d.get(..HEADER)?;
    if h.get(..4)? != MAGIC {
        return None;
    }
    let version = le32(h, 4)?;
    if version != VERSION {
        return None;
    }
    let m = Md2 {
        version,
        skin_width: le32(h, 8)?,
        skin_height: le32(h, 12)?,
        frame_size: le32(h, 16)?,
        num_skins: le32(h, 20)?,
        num_xyz: le32(h, 24)?,
        num_st: le32(h, 28)?,
        num_tris: le32(h, 32)?,
        num_glcmds: le32(h, 36)?,
        num_frames: le32(h, 40)?,
        ofs_skins: le32(h, 44)?,
        ofs_st: le32(h, 48)?,
        ofs_tris: le32(h, 52)?,
        ofs_frames: le32(h, 56)?,
        ofs_glcmds: le32(h, 60)?,
        ofs_end: le32(h, 64)?,
    };
    let len = d.len() as u64;
    for &ofs in [
        m.ofs_skins,
        m.ofs_st,
        m.ofs_tris,
        m.ofs_frames,
        m.ofs_glcmds,
    ]
    .iter()
    {
        // Offset may be 0/empty but must not point past EOF when
        // non-zero.
        if ofs != 0 && u64::from(ofs) > len {
            return None;
        }
    }
    if u64::from(m.ofs_end) > len {
        return None;
    }
    Some(m)
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
        w(&mut d, 4, 8);
        w(&mut d, 8, 256); // skinw
        w(&mut d, 12, 256); // skinh
        w(&mut d, 16, 40); // framesize
        w(&mut d, 20, 3); // skins
        w(&mut d, 24, 100); // xyz
        w(&mut d, 28, 80); // st
        w(&mut d, 32, 60); // tris
        w(&mut d, 36, 200); // glcmds
        w(&mut d, 40, 16); // frames
        w(&mut d, 44, 68); // skins ofs
        w(&mut d, 48, 128); // st
        w(&mut d, 52, 160); // tris
        w(&mut d, 56, 192); // frames
        w(&mut d, 60, 224); // glcmds
        w(&mut d, 64, 256); // end
        d
    }

    #[test]
    fn fields() {
        let m = parse(&fixture()).unwrap();
        assert_eq!(m.version, 8);
        assert_eq!(m.skin_width, 256);
        assert_eq!(m.frame_size, 40);
        assert_eq!(m.num_skins, 3);
        assert_eq!(m.num_xyz, 100);
        assert_eq!(m.num_st, 80);
        assert_eq!(m.num_tris, 60);
        assert_eq!(m.num_glcmds, 200);
        assert_eq!(m.num_frames, 16);
        assert_eq!(m.ofs_skins, 68);
        assert_eq!(m.ofs_end, 256);
        assert_eq!(m.section_at(0), Some(68));
        assert_eq!(m.section_at(4), Some(224));
        assert_eq!(m.section_at(5), None);
        assert_eq!(Md2::section_name(3), Some("frames"));
        assert_eq!(Md2::section_name(9), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[0] = 0;
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[64] = 255; // ofs_end past file (256) -> 511
        d2[65] = 1;
        assert!(parse(&d2).is_none());
    }
}

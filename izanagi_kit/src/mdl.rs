//! Quake MDL model file ("IDPO" version 6).
//!
//! The 84-byte header carries the magic, version, scale and
//! translation vectors (kept as raw u32 bits — the kit does not
//! interpret floats), bounding radius, eye position, skin
//! geometry and counts, sync type and flags.
//!
//! ```
//! use izanagi_kit::mdl::{parse, MAGIC};
//!
//! let mut d = vec![0u8; 84];
//! d[..4].copy_from_slice(&MAGIC);
//! d[4..8].copy_from_slice(&[6, 0, 0, 0]);       // version
//! d[60..64].copy_from_slice(&[10, 0, 0, 0]);    // numverts
//! d[68..72].copy_from_slice(&[3, 0, 0, 0]);     // numframes
//! let m = parse(&d).unwrap();
//! assert_eq!(m.version, 6);
//! assert_eq!(m.numverts, 10);
//! assert_eq!(m.numframes, 3);
//! ```

/// `IDPO` magic bytes.
pub const MAGIC: [u8; 4] = *b"IDPO";
/// Expected version.
pub const VERSION: u32 = 6;
/// Header size.
pub const HEADER: usize = 84;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// A parsed Quake MDL header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mdl {
    /// Version (6).
    pub version: u32,
    /// Scale xyz — raw f32 bits.
    pub scale_raw: [u32; 3],
    /// Translate xyz — raw f32 bits.
    pub translate_raw: [u32; 3],
    /// Bounding radius — raw f32 bits.
    pub radius_raw: u32,
    /// Eye position xyz — raw f32 bits.
    pub eye_raw: [u32; 3],
    /// Skin count.
    pub numskins: u32,
    /// Skin width/height.
    pub skin_width: u32,
    /// Skin height.
    pub skin_height: u32,
    /// Vertex count.
    pub numverts: u32,
    /// Triangle count.
    pub numtris: u32,
    /// Animation frame count.
    pub numframes: u32,
    /// Sync type (0 = async).
    pub synctype: u32,
    /// Flags (rocket/grenade/rotate gib trails).
    pub flags: u32,
    /// Average size — raw f32 bits.
    pub size_raw: u32,
}

impl Mdl {
    /// Vertex section offset (right after the header, before
    /// frames — callers walk skins first in practice; this is the
    /// canonical file start for geometry after the header).
    pub fn data_at(&self) -> usize {
        HEADER
    }
    /// Total on-disk vertex bytes (3 packed coords + normal idx).
    pub fn vertex_bytes(&self) -> u64 {
        u64::from(self.numverts) * 4
    }
}

/// Parse an MDL header. Returns `None` on a bad magic/version or
/// truncation.
pub fn parse(d: &[u8]) -> Option<Mdl> {
    let h = d.get(..HEADER)?;
    if h.get(..4)? != MAGIC {
        return None;
    }
    let version = le32(h, 4)?;
    if version != VERSION {
        return None;
    }
    let v3 =
        |o: usize| -> Option<[u32; 3]> { Some([le32(h, o)?, le32(h, o + 4)?, le32(h, o + 8)?]) };
    Some(Mdl {
        version,
        scale_raw: v3(8)?,
        translate_raw: v3(20)?,
        radius_raw: le32(h, 32)?,
        eye_raw: v3(36)?,
        numskins: le32(h, 48)?,
        skin_width: le32(h, 52)?,
        skin_height: le32(h, 56)?,
        numverts: le32(h, 60)?,
        numtris: le32(h, 64)?,
        numframes: le32(h, 68)?,
        synctype: le32(h, 72)?,
        flags: le32(h, 76)?,
        size_raw: le32(h, 80)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[..4].copy_from_slice(&MAGIC);
        let w = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> (i * 8)) as u8;
            }
        };
        w(&mut d, 4, 6);
        w(&mut d, 8, 0x3F80_0000); // scale.x = 1.0
        w(&mut d, 12, 0x3F80_0000);
        w(&mut d, 16, 0x3F80_0000);
        w(&mut d, 32, 0x4220_0000); // radius 40.0
        w(&mut d, 48, 2); // skins
        w(&mut d, 52, 64);
        w(&mut d, 56, 64);
        w(&mut d, 60, 128); // verts
        w(&mut d, 64, 200); // tris
        w(&mut d, 68, 12); // frames
        w(&mut d, 72, 1); // sync
        w(&mut d, 76, 8); // flags: rotate
        w(&mut d, 80, 0x4200_0000);
        d
    }

    #[test]
    fn fields() {
        let m = parse(&fixture()).unwrap();
        assert_eq!(m.version, 6);
        assert_eq!(m.scale_raw, [0x3F80_0000; 3]);
        assert_eq!(m.radius_raw, 0x4220_0000);
        assert_eq!(m.numskins, 2);
        assert_eq!(m.skin_width, 64);
        assert_eq!(m.numverts, 128);
        assert_eq!(m.numtris, 200);
        assert_eq!(m.numframes, 12);
        assert_eq!(m.synctype, 1);
        assert_eq!(m.flags, 8);
        assert_eq!(m.data_at(), HEADER);
        assert_eq!(m.vertex_bytes(), 512);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[4] = 7; // wrong version
        assert!(parse(&d2).is_none());
    }
}

//! Quake III Arena `.md3` — id Software mesh model.
//!
//! Fixed 108-byte little-endian header: `IDP3` magic, version 15,
//! a 64-byte model name, `flags`, then counts (`numFrames`,
//! `numTags`, `numSurfaces`, `numSkins`) and section offsets
//! (`ofsFrames`, `ofsTags`, `ofsSurfaces`, `ofsEnd`).
//!
//! ```
//! let mut d = b"IDP3".to_vec();
//! d.extend_from_slice(&[15, 0, 0, 0]);
//! let mut name = b"models/players/x/head".to_vec();
//! name.resize(64, 0);
//! d.extend_from_slice(&name);
//! d.extend_from_slice(&[0; 4]); // flags
//! for v in [1u32, 2, 3, 0] {
//!     d.extend_from_slice(&v.to_le_bytes());
//! }
//! for v in [108u32, 108, 108, 300] {
//!     d.extend_from_slice(&v.to_le_bytes());
//! }
//! d.resize(300, 0);
//! let f = izanagi_kit::md3::parse(&d).unwrap();
//! assert_eq!(f.name, "models/players/x/head");
//! assert_eq!(f.frames, 1);
//! assert_eq!(f.surfaces, 3);
//! ```
//!
//! Reference: MD3 format notes (id Tech 3 / ioquake3 source,
//! `q3map`/`md3` loader docs). Integer-only.

/// Parsed 108-byte `.md3` header.
#[derive(Debug, Clone, PartialEq)]
pub struct Md3 {
    /// Model name (64-byte field, NUL-trimmed).
    pub name: String,
    /// `numFrames` — animation frames.
    pub frames: u32,
    /// `numTags` — attachment points.
    pub tags: u32,
    /// `numSurfaces` — meshes.
    pub surfaces: u32,
    /// `numSkins` — skin names.
    pub skins: u32,
    /// `ofsFrames` — frame block offset.
    pub frames_offset: u32,
    /// `ofsTags` — tag block offset.
    pub tags_offset: u32,
    /// `ofsSurfaces` — surface block offset.
    pub surfaces_offset: u32,
    /// `ofsEnd` — declared end-of-data offset.
    pub eof_offset: u32,
    /// `ofsEnd` points past the actual buffer length.
    pub truncated: bool,
}

fn u32le(d: &[u8], i: usize) -> u32 {
    (d[i] as u32) | (d[i + 1] as u32) << 8 | (d[i + 2] as u32) << 16 | (d[i + 3] as u32) << 24
}

/// Parse the fixed header. `None` when shorter than 108 bytes,
/// the `IDP3` magic is absent, or the version is not 15.
pub fn parse(d: &[u8]) -> Option<Md3> {
    if d.len() < 108 || &d[..4] != b"IDP3" || u32le(d, 4) != 15 {
        return None;
    }
    let name = {
        let n = &d[8..72];
        let e = n.iter().position(|b| *b == 0).unwrap_or(64);
        String::from_utf8_lossy(&n[..e]).into_owned()
    };
    let eof = u32le(d, 104);
    Some(Md3 {
        name,
        frames: u32le(d, 76),
        tags: u32le(d, 80),
        surfaces: u32le(d, 84),
        skins: u32le(d, 88),
        frames_offset: u32le(d, 92),
        tags_offset: u32le(d, 96),
        surfaces_offset: u32le(d, 100),
        eof_offset: eof,
        truncated: eof as usize > d.len(),
    })
}

/// `true` when the `IDP3` magic + version 15 prefix is present.
pub fn detect(d: &[u8]) -> bool {
    d.len() >= 8 && &d[..4] == b"IDP3" && u32le(d, 4) == 15
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(eof: u32) -> Vec<u8> {
        let mut d = b"IDP3".to_vec();
        d.extend_from_slice(&[15, 0, 0, 0]);
        let mut name = b"q3model".to_vec();
        name.resize(64, 0);
        d.extend_from_slice(&name);
        d.extend_from_slice(&[0; 4]);
        for v in [4u32, 1, 2, 1] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        for v in [200u32, 300, 400, eof] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d
    }

    #[test]
    fn parses() {
        let d = doc(108);
        let f = parse(&d).unwrap();
        assert_eq!(f.name, "q3model");
        assert_eq!(f.frames, 4);
        assert_eq!(f.tags, 1);
        assert_eq!(f.surfaces, 2);
        assert_eq!(f.skins, 1);
        assert_eq!(f.frames_offset, 200);
        assert_eq!(f.tags_offset, 300);
        assert_eq!(f.surfaces_offset, 400);
        assert_eq!(f.eof_offset, 108);
        assert!(!f.truncated); // eof == len → exact fit
    }

    #[test]
    fn truncation_flag() {
        let mut d = doc(500);
        d.resize(200, 0);
        assert!(parse(&d).unwrap().truncated);
        let mut d2 = doc(200);
        d2.resize(300, 0);
        assert!(!parse(&d2).unwrap().truncated);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 200]).is_none());
        let mut d = doc(108);
        d[4] = 14; // version 14
        assert!(parse(&d).is_none());
        let mut m = doc(108);
        m[0] = b'X';
        assert!(parse(&m).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&doc(108)));
        let mut d = doc(108);
        d[7] = 1; // version 16
        assert!(!detect(&d));
    }
}

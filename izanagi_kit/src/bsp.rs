//! Quake BSP level file (version 29 "BSP29" / Quake II "IBSP").
//!
//! Quake 1 `.bsp` files open with a bare u32 version `29` (no
//! magic — `IBSP` is Quake II's), followed by 15 `(offset,
//! length)` lump directories. Quake II puts `IBSP` + version 38
//! and 19 lumps. `parse` detects both, checks every lump fits the
//! file, and exposes `lump()`/`lump_name()`.
//!
//! ```
//! use izanagi_kit::bsp::{parse, Kind};
//!
//! let mut d = vec![0u8; 4 + 15 * 8 + 8];
//! d[..4].copy_from_slice(&[29, 0, 0, 0]);
//! // lump 0 (entities): at 124, len 8
//! d[4] = 124; d[8] = 8;
//! let b = parse(&d).unwrap();
//! assert_eq!(b.kind, Kind::Quake1);
//! assert_eq!(b.lump_name(0), Some("entities"));
//! ```

/// Quake II (and some derivatives) signature.
pub const MAGIC_IBSP: u32 = 0x4942_5350; // "IBSP" LE
/// Quake 1 lump count.
pub const LUMPS_Q1: usize = 15;
/// Quake II lump count.
pub const LUMPS_Q2: usize = 19;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// Detected BSP family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Bare version 29 (Quake 1 / Half-Life).
    Quake1,
    /// `IBSP` + version 38 (Quake II) or 46 (Quake III).
    Ibsp,
    /// Unrecognized.
    Other,
}

/// One `(offset, length)` lump directory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lump {
    /// Byte offset into the file.
    pub offset: u32,
    /// Length in bytes.
    pub length: u32,
}

/// Quake 1 lump names (index = lump number).
pub const Q1_LUMPS: [&str; LUMPS_Q1] = [
    "entities",
    "planes",
    "textures",
    "vertices",
    "visibility",
    "nodes",
    "texinfo",
    "faces",
    "lighting",
    "clipnodes",
    "leaves",
    "marksurfaces",
    "edges",
    "surfedges",
    "models",
];
/// Quake II lump names.
pub const Q2_LUMPS: [&str; LUMPS_Q2] = [
    "entities",
    "planes",
    "vertices",
    "visibility",
    "nodes",
    "texinfo",
    "faces",
    "lighting",
    "leaves",
    "leaffaces",
    "leafbrushes",
    "edges",
    "surfedges",
    "models",
    "brushes",
    "brushsides",
    "pop",
    "areas",
    "areaportals",
];

/// A parsed BSP header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bsp {
    /// Detected family.
    pub kind: Kind,
    /// Version (29 / 38 / 46...).
    pub version: u32,
    /// Lump directory entries (bounds-checked against file len).
    pub lumps: Vec<Lump>,
}

impl Bsp {
    /// Lump `i` as a `&[u8]` view into `d`, or `None`.
    pub fn lump<'a>(&self, d: &'a [u8], i: usize) -> Option<&'a [u8]> {
        let l = self.lumps.get(i)?;
        d.get(l.offset as usize..l.offset as usize + l.length as usize)
    }
    /// Human-readable lump name (family-aware).
    pub fn lump_name(&self, i: usize) -> Option<&'static str> {
        match self.kind {
            Kind::Quake1 => Q1_LUMPS.get(i).copied(),
            Kind::Ibsp => Q2_LUMPS.get(i).copied(),
            Kind::Other => None,
        }
    }
}

/// Parse a BSP header. Returns `None` on a bad version or a lump
/// that overruns the file.
pub fn parse(d: &[u8]) -> Option<Bsp> {
    let (kind, version, dir_at, count) = if le32(d, 0)? == MAGIC_IBSP {
        (Kind::Ibsp, le32(d, 4)?, 8usize, LUMPS_Q2)
    } else {
        (Kind::Quake1, le32(d, 0)?, 4usize, LUMPS_Q1)
    };
    let ok = match kind {
        Kind::Quake1 => version == 29,
        Kind::Ibsp => version == 38 || version == 46,
        Kind::Other => false,
    };
    if !ok {
        return None;
    }
    let dir = d.get(dir_at..dir_at + count * 8)?;
    let mut lumps = Vec::with_capacity(count);
    for i in 0..count {
        let offset = le32(dir, i * 8)?;
        let length = le32(dir, i * 8 + 4)?;
        let end = offset.checked_add(length)? as usize;
        if end > d.len() {
            return None;
        }
        lumps.push(Lump { offset, length });
    }
    Some(Bsp {
        kind,
        version,
        lumps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn q1() -> Vec<u8> {
        let mut d = vec![0u8; 4 + LUMPS_Q1 * 8 + 16];
        d[..4].copy_from_slice(&29u32.to_le_bytes());
        // entities lump at 124 len 4, models at 128 len 8
        d[4..8].copy_from_slice(&124u32.to_le_bytes());
        d[8..12].copy_from_slice(&4u32.to_le_bytes());
        d[4 + 14 * 8..4 + 14 * 8 + 4].copy_from_slice(&128u32.to_le_bytes());
        d[4 + 14 * 8 + 4..4 + 14 * 8 + 8].copy_from_slice(&8u32.to_le_bytes());
        d[124] = b'{';
        d
    }

    #[test]
    fn quake1() {
        let b = parse(&q1()).unwrap();
        assert_eq!(b.kind, Kind::Quake1);
        assert_eq!(b.version, 29);
        assert_eq!(b.lumps.len(), 15);
        assert_eq!(
            b.lumps[0],
            Lump {
                offset: 124,
                length: 4
            }
        );
        assert_eq!(b.lump_name(0), Some("entities"));
        assert_eq!(b.lump_name(14), Some("models"));
        assert_eq!(b.lump(&q1(), 0), Some(&[b'{', 0, 0, 0][..]));
        assert!(b.lump(&q1(), 99).is_none());
    }

    #[test]
    fn ibsp_and_rejects() {
        let mut d = vec![0u8; 8 + LUMPS_Q2 * 8];
        d[..4].copy_from_slice(&MAGIC_IBSP.to_le_bytes());
        d[4..8].copy_from_slice(&38u32.to_le_bytes());
        let b = parse(&d).unwrap();
        assert_eq!(b.kind, Kind::Ibsp);
        assert_eq!(b.version, 38);
        assert_eq!(b.lumps.len(), 19);
        assert_eq!(b.lump_name(14), Some("brushes"));
        // bad version
        let mut bad = q1();
        bad[0] = 30;
        assert!(parse(&bad).is_none());
        // lump past EOF
        let mut bad2 = q1();
        bad2[4] = 255;
        assert!(parse(&bad2).is_none());
        assert!(parse(&[0u8; 4]).is_none());
    }
}

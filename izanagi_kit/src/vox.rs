//! MagicaVoxel `.vox` models — the RIFF-style voxel format from
//! ephtracy's editor, common in voxel art pipelines. A file is the
//! `VOX ` magic + version (150), then a `MAIN` root chunk whose
//! children are `PACK` (optional model count), alternating
//! `SIZE`/`XYZI` model pairs, and an optional `RGBA` palette. Each
//! chunk header is `id[4] | content_len u32 | children_len u32` —
//! children bytes directly follow content bytes.
//!
//! Pairs are folded in order: every `XYZI` belongs to the most recent
//! `SIZE`. Unknown chunk ids (scene-graph chunks like `nTRN`/`MATL`,
//! added later) are skipped by length, never an error. When `RGBA`
//! is absent, [`DEFAULT_PALETTE`] applies (spec §8); palette index 0
//! is unused and colors are stored at index `i+1` on the wire.
//!
//! ```
//! use izanagi_kit::vox::{parse, DEFAULT_PALETTE};
//!
//! let mut d = b"VOX \x96\x00\x00\x00".to_vec();
//! let kids: &[u8] = &[
//!     // SIZE 2x2x2
//!     b'S', b'I', b'Z', b'E', 12, 0, 0, 0, 0, 0, 0, 0,
//!     2, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0,
//!     // XYZI: one voxel at (0,0,0) palette index 5
//!     b'X', b'Y', b'Z', b'I', 8, 0, 0, 0, 0, 0, 0, 0,
//!     1, 0, 0, 0, 0, 0, 0, 5,
//! ];
//! d.extend_from_slice(b"MAIN");
//! d.extend_from_slice(&[0; 4]);
//! d.extend_from_slice(&(kids.len() as u32).to_le_bytes());
//! d.extend_from_slice(kids);
//! let v = parse(&d).unwrap();
//! assert_eq!(v.models[0].size, [2, 2, 2]);
//! assert_eq!(v.models[0].voxels[0].color, 5);
//! assert_eq!(v.palette().1[1], [0xff; 4]); // default when RGBA absent
//! ```

use std::vec::Vec;

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let b = d.get(at..at + 4)?;
    Some(u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16 | u32::from(b[3]) << 24)
}

/// One voxel: position plus palette color index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voxel {
    /// X coordinate (0 .. `size[0]`).
    pub x: u8,
    /// Y coordinate.
    pub y: u8,
    /// Z coordinate — gravity direction per the spec.
    pub z: u8,
    /// Palette index 1–255 (0 is empty / unused).
    pub color: u8,
}

/// A `SIZE`+`XYZI` pair.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// `[x, y, z]` dimensions in voxels.
    pub size: [u32; 3],
    /// Voxel list — `(x, y, z, colorIndex)` rows.
    pub voxels: Vec<Voxel>,
}

/// A parsed `.vox` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Vox {
    /// Version word from the header (150 in the 2016 spec).
    pub version: u32,
    /// Models in file order (one `SIZE`+`XYZI` pair each).
    pub models: Vec<Model>,
    /// The `RGBA` palette when present: `palette[1]` is wire color 0
    /// — the file's index space is shifted by one.
    pub rgba: Option<[[u8; 4]; 256]>,
}

/// Which palette the file carries: its own `RGBA` chunk or the spec
/// default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteKind {
    /// An `RGBA` chunk was present.
    Embedded,
    /// `RGBA` absent — [`DEFAULT_PALETTE`] applies.
    Default,
}

/// The spec §8 palette, verbatim (`0xAABBGGRR` words: byte order on
/// the wire is R,G,B,A — see [`rgba`]).
pub const DEFAULT_PALETTE: [u32; 256] = [
    0x00000000, 0xffffffff, 0xffccffff, 0xff99ffff, 0xff66ffff, 0xff33ffff, 0xff00ffff, 0xffffccff,
    0xffccccff, 0xff99ccff, 0xff66ccff, 0xff33ccff, 0xff00ccff, 0xffff99ff, 0xffcc99ff, 0xff9999ff,
    0xff6699ff, 0xff3399ff, 0xff0099ff, 0xffff66ff, 0xffcc66ff, 0xff9966ff, 0xff6666ff, 0xff3366ff,
    0xff0066ff, 0xffff33ff, 0xffcc33ff, 0xff9933ff, 0xff6633ff, 0xff3333ff, 0xff0033ff, 0xffff00ff,
    0xffcc00ff, 0xff9900ff, 0xff6600ff, 0xff3300ff, 0xff0000ff, 0xffffffcc, 0xffccffcc, 0xff99ffcc,
    0xff66ffcc, 0xff33ffcc, 0xff00ffcc, 0xffffcccc, 0xffcccccc, 0xff99cccc, 0xff66cccc, 0xff33cccc,
    0xff00cccc, 0xffff99cc, 0xffcc99cc, 0xff9999cc, 0xff6699cc, 0xff3399cc, 0xff0099cc, 0xffff66cc,
    0xffcc66cc, 0xff9966cc, 0xff6666cc, 0xff3366cc, 0xff0066cc, 0xffff33cc, 0xffcc33cc, 0xff9933cc,
    0xff6633cc, 0xff3333cc, 0xff0033cc, 0xffff00cc, 0xffcc00cc, 0xff9900cc, 0xff6600cc, 0xff3300cc,
    0xff0000cc, 0xffffff99, 0xffccff99, 0xff99ff99, 0xff66ff99, 0xff33ff99, 0xff00ff99, 0xffffcc99,
    0xffcccc99, 0xff99cc99, 0xff66cc99, 0xff33cc99, 0xff00cc99, 0xffff9999, 0xffcc9999, 0xff999999,
    0xff669999, 0xff339999, 0xff009999, 0xffff6699, 0xffcc6699, 0xff996699, 0xff666699, 0xff336699,
    0xff006699, 0xffff3399, 0xffcc3399, 0xff993399, 0xff663399, 0xff333399, 0xff003399, 0xffff0099,
    0xffcc0099, 0xff990099, 0xff660099, 0xff330099, 0xff000099, 0xffffff66, 0xffccff66, 0xff99ff66,
    0xff66ff66, 0xff33ff66, 0xff00ff66, 0xffffcc66, 0xffcccc66, 0xff99cc66, 0xff66cc66, 0xff33cc66,
    0xff00cc66, 0xffff9966, 0xffcc9966, 0xff999966, 0xff669966, 0xff339966, 0xff009966, 0xffff6666,
    0xffcc6666, 0xff996666, 0xff666666, 0xff336666, 0xff006666, 0xffff3366, 0xffcc3366, 0xff993366,
    0xff663366, 0xff333366, 0xff003366, 0xffff0066, 0xffcc0066, 0xff990066, 0xff660066, 0xff330066,
    0xff000066, 0xffffff33, 0xffccff33, 0xff99ff33, 0xff66ff33, 0xff33ff33, 0xff00ff33, 0xffffcc33,
    0xffcccc33, 0xff99cc33, 0xff66cc33, 0xff33cc33, 0xff00cc33, 0xffff9933, 0xffcc9933, 0xff999933,
    0xff669933, 0xff339933, 0xff009933, 0xffff6633, 0xffcc6633, 0xff996633, 0xff666633, 0xff336633,
    0xff006633, 0xffff3333, 0xffcc3333, 0xff993333, 0xff663333, 0xff333333, 0xff003333, 0xffff0033,
    0xffcc0033, 0xff990033, 0xff660033, 0xff330033, 0xff000033, 0xffffff00, 0xffccff00, 0xff99ff00,
    0xff66ff00, 0xff33ff00, 0xff00ff00, 0xffffcc00, 0xffcccc00, 0xff99cc00, 0xff66cc00, 0xff33cc00,
    0xff00cc00, 0xffff9900, 0xffcc9900, 0xff999900, 0xff669900, 0xff339900, 0xff009900, 0xffff6600,
    0xffcc6600, 0xff996600, 0xff666600, 0xff336600, 0xff006600, 0xffff3300, 0xffcc3300, 0xff993300,
    0xff663300, 0xff333300, 0xff003300, 0xffff0000, 0xffcc0000, 0xff990000, 0xff660000, 0xff330000,
    0xff0000ee, 0xff0000dd, 0xff0000bb, 0xff0000aa, 0xff000088, 0xff000077, 0xff000055, 0xff000044,
    0xff000022, 0xff000011, 0xff00ee00, 0xff00dd00, 0xff00bb00, 0xff00aa00, 0xff008800, 0xff007700,
    0xff005500, 0xff004400, 0xff002200, 0xff001100, 0xffee0000, 0xffdd0000, 0xffbb0000, 0xffaa0000,
    0xff880000, 0xff770000, 0xff550000, 0xff440000, 0xff220000, 0xff110000, 0xffeeeeee, 0xffdddddd,
    0xffbbbbbb, 0xffaaaaaa, 0xff888888, 0xff777777, 0xff555555, 0xff444444, 0xff222222, 0xff111111,
];

/// Expand a `DEFAULT_PALETTE` word to `(R, G, B, A)` bytes — the wire
/// order is little-endian RGBA, so the low byte is red.
pub fn rgba(word: u32) -> [u8; 4] {
    [
        (word & 0xff) as u8,
        ((word >> 8) & 0xff) as u8,
        ((word >> 16) & 0xff) as u8,
        ((word >> 24) & 0xff) as u8,
    ]
}

/// Parse a `.vox` file. `None` on a bad magic, a truncated chunk, a
/// `XYZI` without a preceding `SIZE`, or a size/count field that
/// overruns its chunk.
pub fn parse(d: &[u8]) -> Option<Vox> {
    if d.len() < 12 || &d[..4] != b"VOX " {
        return None;
    }
    let version = u32le(d, 4)?;
    // Root chunk header: must be MAIN; its children are the file body.
    if d.len() < 20 || &d[8..12] != b"MAIN" {
        return None;
    }
    let content_len = u32le(d, 12)? as usize;
    let children_len = u32le(d, 16)? as usize;
    let kids_at = 20usize.checked_add(content_len)?;
    let kids_end = kids_at.checked_add(children_len)?;
    if kids_end > d.len() {
        return None;
    }
    let mut models: Vec<Model> = Vec::new();
    let mut rgba: Option<[[u8; 4]; 256]> = None;
    let mut pending_size: Option<[u32; 3]> = None;
    let mut at = kids_at;
    while at < kids_end {
        let id = d.get(at..at + 4)?;
        let n = u32le(d, at + 4)? as usize;
        let m = u32le(d, at + 8)? as usize;
        let body = at + 12;
        let kids = body.checked_add(n)?;
        let next = kids.checked_add(m)?;
        if next > kids_end {
            return None;
        }
        match id {
            b"SIZE" => {
                if n < 12 {
                    return None;
                }
                pending_size = Some([u32le(d, body)?, u32le(d, body + 4)?, u32le(d, body + 8)?]);
            }
            b"XYZI" => {
                let size = pending_size.take()?;
                if n < 4 {
                    return None;
                }
                let count = u32le(d, body)? as usize;
                if 4usize.checked_add(count.checked_mul(4)?)? > n {
                    return None;
                }
                let mut voxels = Vec::with_capacity(count);
                for i in 0..count {
                    let o = body + 4 + i * 4;
                    voxels.push(Voxel {
                        x: d[o],
                        y: d[o + 1],
                        z: d[o + 2],
                        color: d[o + 3],
                    });
                }
                models.push(Model { size, voxels });
            }
            b"RGBA" => {
                if n < 1024 {
                    return None;
                }
                let mut p = [[0u8; 4]; 256];
                let mut i = 0;
                while i < 255 {
                    let o = body + i * 4;
                    p[i + 1] = [d[o], d[o + 1], d[o + 2], d[o + 3]];
                    i += 1;
                }
                rgba = Some(p);
            }
            _ => {}
        }
        at = next;
    }
    Some(Vox {
        version,
        models,
        rgba,
    })
}

impl Vox {
    /// `(kind, palette)` — the embedded `RGBA` chunk when present,
    /// otherwise the spec default expanded via [`rgba`].
    pub fn palette(&self) -> (PaletteKind, [[u8; 4]; 256]) {
        match self.rgba {
            Some(p) => (PaletteKind::Embedded, p),
            None => {
                let mut p = [[0u8; 4]; 256];
                let mut i = 0;
                while i < 256 {
                    p[i] = rgba(DEFAULT_PALETTE[i]);
                    i += 1;
                }
                (PaletteKind::Default, p)
            }
        }
    }

    /// `(R, G, B, A)` for a voxel's color index (index 0 → all zero).
    pub fn color(&self, index: u8) -> [u8; 4] {
        let (_, p) = self.palette();
        p[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vox(kids: &[u8]) -> Vec<u8> {
        let mut d = b"VOX \x96\x00\x00\x00".to_vec();
        d.extend_from_slice(b"MAIN");
        d.extend_from_slice(&[0; 4]);
        d.extend_from_slice(&(kids.len() as u32).to_le_bytes());
        d.extend_from_slice(kids);
        d
    }

    fn chunk(id: &[u8; 4], body: &[u8], kids: &[u8]) -> Vec<u8> {
        let mut c = id.to_vec();
        c.extend_from_slice(&(body.len() as u32).to_le_bytes());
        c.extend_from_slice(&(kids.len() as u32).to_le_bytes());
        c.extend_from_slice(body);
        c.extend_from_slice(kids);
        c
    }

    #[test]
    fn two_models_and_palette() {
        let mut kids = chunk(b"PACK", &2u32.to_le_bytes(), &[]);
        for (sz, n) in [(1u32, 1u32), (4, 2)] {
            let mut sb = Vec::new();
            for v in [sz, sz, sz] {
                sb.extend_from_slice(&v.to_le_bytes());
            }
            kids.extend(chunk(b"SIZE", &sb, &[]));
            let mut xb = n.to_le_bytes().to_vec();
            for _ in 0..n {
                xb.extend_from_slice(&[1, 2, 3, 9]);
            }
            kids.extend(chunk(b"XYZI", &xb, &[]));
        }
        let mut pal = Vec::new();
        for i in 0..256u32 {
            pal.extend_from_slice(&[i as u8, (i + 1) as u8, (i + 2) as u8, 0xff]);
        }
        kids.extend(chunk(b"RGBA", &pal, &[]));
        kids.extend(chunk(b"nTRN", &[1, 2, 3, 4], &[])); // unknown: skipped
        let v = parse(&vox(&kids)).unwrap();
        assert_eq!(v.version, 150);
        assert_eq!(v.models.len(), 2);
        assert_eq!(v.models[1].size, [4, 4, 4]);
        assert_eq!(
            v.models[0].voxels,
            [Voxel {
                x: 1,
                y: 2,
                z: 3,
                color: 9
            }]
        );
        let (kind, p) = v.palette();
        assert_eq!(kind, PaletteKind::Embedded);
        assert_eq!(p[1], [0, 1, 2, 0xff]);
        assert_eq!(v.color(2), [1, 2, 3, 0xff]);
    }

    #[test]
    fn default_palette_when_no_rgba() {
        let v = parse(&vox(&[])).unwrap();
        let (kind, p) = v.palette();
        assert_eq!(kind, PaletteKind::Default);
        assert_eq!(p[0], [0, 0, 0, 0]);
        assert_eq!(p[1], [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(p[255], [0x11, 0x11, 0x11, 0xff]);
        assert_eq!(rgba(0xffff00ff), [0xff, 0x00, 0xff, 0xff]);
    }

    #[test]
    fn malformed() {
        assert!(parse(b"").is_none());
        assert!(parse(b"VOX \x96\0\0\0").is_none());
        assert!(parse(b"VOX \x96\0\0\0NOPE\0\0\0\0\0\0\0\0").is_none());
        // XYZI without SIZE.
        let bad = chunk(b"XYZI", &[0, 0, 0, 0], &[]);
        assert!(parse(&vox(&bad)).is_none());
        // XYZI voxel bytes overrun the chunk.
        let mut xb = 5u32.to_le_bytes().to_vec();
        xb.extend_from_slice(&[0; 4]);
        let bad2 = {
            let mut k = chunk(b"SIZE", &[1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0], &[]);
            k.extend(chunk(b"XYZI", &xb, &[]));
            k
        };
        assert!(parse(&vox(&bad2)).is_none());
        // Child length overruns the root.
        let mut d = b"VOX \x96\0\0\0MAIN\0\0\0\0\xFF\xFF\xFF\x7F".to_vec();
        assert!(parse(&d).is_none());
        d.extend_from_slice(&[0; 4]);
        assert!(parse(&d).is_none());
    }
}

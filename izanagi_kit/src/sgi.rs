//! Silicon Graphics RGB (.sgi/.rgb/.bw) image scanner.
//!
//! An SGI image starts with a 512-byte big-endian header:
//! `0x01DA` magic, storage byte (0 verbatim / 1 RLE), bytes-per-
//! channel, dimension (1–3), x/y/z sizes, pixel min/max, a 4-byte
//! dummy, an 80-byte image name, and a colormap id.
//!
//! ```
//! let mut h = vec![0u8; 512];
//! h[0] = 0x01;
//! h[1] = 0xDA;
//! h[2] = 1; // RLE
//! h[3] = 1; // bpc
//! h[4] = 0;
//! h[5] = 3; // dimension
//! h[8] = 4;
//! h[9] = 8; // xsize 8... written BE
//! let s = izanagi_kit::sgi::parse(&h).unwrap();
//! assert!(s.rle);
//! assert_eq!(s.dimension, 3);
//! ```
//!
//! Reference: SGI image file format documentation (SGI developer
//! toolbox "File Format Specification") — `0x01DA` magic and the
//! 512-byte BE header layout.

/// Parsed SGI header fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Sgi {
    /// `true` for RLE-compressed storage.
    pub rle: bool,
    /// Bytes per channel (1 or 2).
    pub bytes_per_channel: u8,
    /// Dimension count (1–3).
    pub dimension: u16,
    /// X size in pixels.
    pub xsize: u16,
    /// Y size in pixels.
    pub ysize: u16,
    /// Z size (channel count).
    pub zsize: u16,
    /// Minimum pixel value.
    pub pixmin: u32,
    /// Maximum pixel value.
    pub pixmax: u32,
    /// Image name (first 80 bytes, trimmed at NUL).
    pub name: Option<String>,
    /// Colormap id (0 normal, 1 dithered, 2 screen, 3 colormap file).
    pub colormap: u32,
}

fn u16be(d: &[u8], i: usize) -> u16 {
    ((d[i] as u16) << 8) | d[i + 1] as u16
}

fn u32be(d: &[u8], i: usize) -> u32 {
    ((d[i] as u32) << 24) | ((d[i + 1] as u32) << 16) | ((d[i + 2] as u32) << 8) | d[i + 3] as u32
}

/// Parse an SGI header; `None` unless the `0x01DA` magic and sane
/// dimension/size fields are present.
pub fn parse(d: &[u8]) -> Option<Sgi> {
    if d.len() < 512 || d[0] != 0x01 || d[1] != 0xDA {
        return None;
    }
    let storage = d[2];
    if storage > 1 {
        return None;
    }
    let bpc = d[3];
    if !(1..=2).contains(&bpc) {
        return None;
    }
    let dimension = u16be(d, 4);
    if !(1..=3).contains(&dimension) {
        return None;
    }
    let name = {
        let raw = &d[28..108];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(80);
        core::str::from_utf8(&raw[..end])
            .ok()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    Some(Sgi {
        rle: storage == 1,
        bytes_per_channel: bpc,
        dimension,
        xsize: u16be(d, 6),
        ysize: u16be(d, 8),
        zsize: u16be(d, 10),
        pixmin: u32be(d, 16),
        pixmax: u32be(d, 20),
        name,
        colormap: u32be(d, 108),
    })
}

/// `true` if the buffer looks like an SGI image.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Vec<u8> {
        let mut h = vec![0u8; 512];
        h[0] = 0x01;
        h[1] = 0xDA;
        h[2] = 1;
        h[3] = 1;
        h[5] = 3;
        h[6] = 0;
        h[7] = 8;
        h[8] = 0;
        h[9] = 8;
        h[10] = 0;
        h[11] = 3;
        h[23] = 0xFF; // pixmax = 255
        let n = b"test image";
        h[28..28 + n.len()].copy_from_slice(n);
        h
    }

    #[test]
    fn parses() {
        let s = parse(&doc()).unwrap();
        assert!(s.rle);
        assert_eq!(s.bytes_per_channel, 1);
        assert_eq!(s.dimension, 3);
        assert_eq!(s.xsize, 8);
        assert_eq!(s.zsize, 3);
        assert_eq!(s.pixmin, 0);
        assert_eq!(s.colormap, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x01, 0xDA, 0, 0]).is_none()); // too short
        let mut h = doc();
        h[2] = 7; // bad storage
        assert!(parse(&h).is_none());
        let mut h = doc();
        h[5] = 9; // bad dimension
        assert!(parse(&h).is_none());
    }

    #[test]
    fn detects() {
        assert!(detect(&doc()));
        assert!(!detect(&[0x01, 0xDA, 0, 0]));
    }
}

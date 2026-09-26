//! Sun Rasterfile images (`.ras` / `.sun`).
//!
//! The 32-byte header is big-endian throughout: magic
//! `0x59A66A95`, width, height, bits-per-pixel depth, encoded data
//! length, encoding type (`0` old-format raw, `1` standard raw,
//! `2` byte-encoded RLE, `3` RGB-format, `4` TIFF/IFF), colormap type
//! and colormap length. Raster data follows the header + colormap.
//!
//! ```
//! use izanagi_kit::ras::{parse, Kind};
//!
//! let mut d = vec![0u8; 32];
//! let put = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = (v >> 24) as u8; d[at + 1] = (v >> 16) as u8;
//!     d[at + 2] = (v >> 8) as u8; d[at + 3] = v as u8;
//! };
//! put(&mut d, 0, 0x59A66A95);
//! put(&mut d, 4, 4);      // width
//! put(&mut d, 8, 2);      // height
//! put(&mut d, 12, 8);     // depth
//! put(&mut d, 16, 8);     // length
//! put(&mut d, 20, 1);     // standard
//! d.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
//! let r = parse(&d).unwrap();
//! assert_eq!(r.kind, Kind::Standard);
//! assert_eq!((r.width, r.height, r.depth), (4, 2, 8));
//! assert_eq!(r.data(&d).unwrap(), &[1, 2, 3, 4, 5, 6, 7, 8]);
//! ```

/// Header size in bytes.
pub const HEADER: usize = 32;
/// Big-endian magic.
pub const MAGIC: u32 = 0x59A6_6A95;

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

/// Encoding type field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `0` — raw pixels, old format.
    Old,
    /// `1` — raw pixels, standard layout.
    Standard,
    /// `2` — byte-encoded run-length data.
    ByteEncoded,
    /// `3` — raw pixels in RGB order.
    RgbFormat,
    /// `4` — data produced by TIFF/IFF tools.
    TiffIff,
    /// Any other encoding id.
    Other(u32),
}

impl Kind {
    fn of(v: u32) -> Self {
        match v {
            0 => Kind::Old,
            1 => Kind::Standard,
            2 => Kind::ByteEncoded,
            3 => Kind::RgbFormat,
            4 => Kind::TiffIff,
            v => Kind::Other(v),
        }
    }
}

/// A parsed Sun Raster header.
#[derive(Clone, Debug, PartialEq)]
pub struct Ras {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bits per pixel (1, 8, 24, 32...).
    pub depth: u32,
    /// Encoded data length in bytes (0 = infer from geometry).
    pub length: u32,
    /// Encoding type.
    pub kind: Kind,
    /// Colormap type (`0` none, `1` RGB triplets, `2` raw bytes).
    pub map_type: u32,
    /// Colormap length in bytes.
    pub map_length: u32,
}

impl Ras {
    /// Byte offset where the raster starts (`32 + map_length`).
    pub fn data_at(&self) -> usize {
        HEADER + usize::try_from(self.map_length).unwrap_or(u32::MAX as usize)
    }

    /// The encoded raster bytes inside `d` (`length` bytes, or the
    /// remainder when `length` is 0).
    pub fn data<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        let at = self.data_at();
        if self.length == 0 {
            return d.get(at..);
        }
        d.get(at..at + usize::try_from(self.length).ok()?)
    }

    /// The colormap bytes inside `d`, or `None` when there is none
    /// or it overruns the buffer.
    pub fn colormap<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        if self.map_length == 0 {
            return None;
        }
        d.get(HEADER..HEADER + usize::try_from(self.map_length).ok()?)
    }
}

/// Parse a Sun Raster header. Returns `None` on a bad magic or a
/// buffer shorter than 32 bytes.
pub fn parse(d: &[u8]) -> Option<Ras> {
    if be32(d, 0)? != MAGIC {
        return None;
    }
    Some(Ras {
        width: be32(d, 4)?,
        height: be32(d, 8)?,
        depth: be32(d, 12)?,
        length: be32(d, 16)?,
        kind: Kind::of(be32(d, 20)?),
        map_type: be32(d, 24)?,
        map_length: be32(d, 28)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn put(d: &mut [u8], at: usize, v: u32) {
        d[at] = (v >> 24) as u8;
        d[at + 1] = (v >> 16) as u8;
        d[at + 2] = (v >> 8) as u8;
        d[at + 3] = v as u8;
    }

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        put(&mut d, 0, MAGIC);
        put(&mut d, 4, 4);
        put(&mut d, 8, 2);
        put(&mut d, 12, 8);
        put(&mut d, 16, 8);
        put(&mut d, 20, 1);
        put(&mut d, 24, 1);
        put(&mut d, 28, 6);
        d.extend_from_slice(&[0xFF, 0, 0, 0, 0xFF, 0]); // colormap
        d.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]); // raster
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let r = parse(&image()).unwrap();
        assert_eq!((r.width, r.height, r.depth), (4, 2, 8));
        assert_eq!(r.length, 8);
        assert_eq!(r.kind, Kind::Standard);
        assert_eq!(r.map_type, 1);
        assert_eq!(r.map_length, 6);
        assert_eq!(r.data_at(), 38);
    }

    #[test]
    fn colormap_and_data() {
        let d = image();
        let r = parse(&d).unwrap();
        assert_eq!(r.colormap(&d), Some(&d[32..38]));
        assert_eq!(r.data(&d), Some(&d[38..]));
    }

    #[test]
    fn zero_length_means_rest() {
        let mut d = vec![0u8; HEADER];
        put(&mut d, 0, MAGIC);
        put(&mut d, 4, 4);
        put(&mut d, 8, 2);
        put(&mut d, 12, 8);
        put(&mut d, 16, 0); // length 0
        d.extend_from_slice(&[9; 8]);
        let r = parse(&d).unwrap();
        assert_eq!(r.data(&d), Some(&d[32..]));
        assert_eq!(r.colormap(&d), None);
    }

    #[test]
    fn kind_ids() {
        assert_eq!(Kind::of(0), Kind::Old);
        assert_eq!(Kind::of(2), Kind::ByteEncoded);
        assert_eq!(Kind::of(3), Kind::RgbFormat);
        assert_eq!(Kind::of(4), Kind::TiffIff);
        assert_eq!(Kind::of(7), Kind::Other(7));
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[0u8; 31]), None);
        let mut d = image();
        d[0] = 0;
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn constants() {
        assert_eq!(HEADER, 32);
        assert_eq!(MAGIC, 0x59A6_6A95);
    }
}

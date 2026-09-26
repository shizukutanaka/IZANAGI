//! farbfeld lossless images (suckless `ff` format).
//!
//! The simplest possible format: `"farbfeld"` magic, then big-endian
//! `u32` width and height, then `width * height` RGBA pixels of four
//! 16-bit big-endian channels. No compression, no palette, no
//! metadata.
//!
//! ```
//! use izanagi_kit::farbfeld::parse;
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(b"farbfeld");
//! d.extend_from_slice(&2u32.to_be_bytes());
//! d.extend_from_slice(&1u32.to_be_bytes());
//! d.extend_from_slice(&[0, 0xFF, 0, 0x80, 0, 0, 0xFF, 0xFF]); // px 0
//! d.extend_from_slice(&[0xFF, 0xFF, 0, 0, 0, 0xFF, 0, 0xFF]); // px 1
//! let f = parse(&d).unwrap();
//! assert_eq!((f.width, f.height), (2, 1));
//! assert_eq!(f.pixel(&d, 0, 0), Some([0xFF, 0x80, 0, 0xFFFF]));
//! assert_eq!(f.pixel(&d, 1, 0), Some([0xFFFF, 0, 0xFF, 0xFF]));
//! ```

/// Magic string length field offset (header is 16 bytes total).
pub const MAGIC: &[u8; 8] = b"farbfeld";
/// Header size in bytes.
pub const HEADER: usize = 16;
/// Bytes per pixel (RGBA x u16).
pub const PIXEL: usize = 8;

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

/// A parsed farbfeld header.
#[derive(Clone, Debug, PartialEq)]
pub struct Farbfeld {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Farbfeld {
    /// Total raster byte count (`width * height * 8`).
    pub fn raster_len(&self) -> usize {
        usize::try_from(self.width)
            .unwrap_or(u32::MAX as usize)
            .saturating_mul(usize::try_from(self.height).unwrap_or(u32::MAX as usize))
            .saturating_mul(PIXEL)
    }

    /// The whole raster (`HEADER` bytes into `d`), or `None` when the
    /// raster is shorter than the geometry requires.
    pub fn pixels<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(HEADER..HEADER + self.raster_len())
    }

    /// One pixel as `[r, g, b, a]` u16 big-endian channels, or `None`
    /// for out-of-bounds coordinates or a truncated raster.
    pub fn pixel(&self, d: &[u8], x: u32, y: u32) -> Option<[u16; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = usize::try_from(y)
            .ok()?
            .checked_mul(usize::try_from(self.width).ok()?)?
            .checked_add(usize::try_from(x).ok()?)?
            .checked_mul(PIXEL)?
            .checked_add(HEADER)?;
        Some([
            be16(d, i)?,
            be16(d, i + 2)?,
            be16(d, i + 4)?,
            be16(d, i + 6)?,
        ])
    }
}

/// Parse a farbfeld image header. Returns `None` on a bad magic or
/// when the buffer cannot hold the declared raster.
pub fn parse(d: &[u8]) -> Option<Farbfeld> {
    if d.get(..8)? != MAGIC {
        return None;
    }
    let f = Farbfeld {
        width: be32(d, 8)?,
        height: be32(d, 12)?,
    };
    d.get(HEADER..HEADER + f.raster_len())?;
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(MAGIC);
        d.extend_from_slice(&2u32.to_be_bytes());
        d.extend_from_slice(&2u32.to_be_bytes());
        for i in 0..4u8 {
            let r = u16::from(i) * 0x0101;
            d.extend_from_slice(&r.to_be_bytes());
            d.extend_from_slice(&r.to_be_bytes());
            d.extend_from_slice(&r.to_be_bytes());
            d.extend_from_slice(&r.to_be_bytes());
        }
        d
    }

    #[test]
    fn parse_and_index() {
        let d = image();
        let f = parse(&d).unwrap();
        assert_eq!((f.width, f.height), (2, 2));
        assert_eq!(f.raster_len(), 32);
        assert_eq!(f.pixels(&d).unwrap().len(), 32);
        assert_eq!(f.pixel(&d, 0, 0), Some([0; 4]));
        assert_eq!(f.pixel(&d, 1, 0), Some([0x0101; 4]));
        assert_eq!(f.pixel(&d, 0, 1), Some([0x0202; 4]));
        assert_eq!(f.pixel(&d, 1, 1), Some([0x0303; 4]));
        assert_eq!(f.pixel(&d, 2, 0), None);
        assert_eq!(f.pixel(&d, 0, 2), None);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0u8; 16]), None);
        let mut d = image();
        d.truncate(20); // raster short
        assert_eq!(parse(&d), None);
        let mut d2 = image();
        d2[0] = b'x';
        assert_eq!(parse(&d2), None);
    }

    #[test]
    fn constants() {
        assert_eq!(HEADER, 16);
        assert_eq!(PIXEL, 8);
        assert_eq!(MAGIC, b"farbfeld");
    }
}

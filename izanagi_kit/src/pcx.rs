//! PC Paintbrush `.pcx` images (ZSoft).
//!
//! The 128-byte header: maker `0x0A`, version, encoding (`1` = RLE),
//! bits per pixel per plane, the window rectangle (`xmin/ymin/xmax/
//! ymax` u16 LE), hres/vres, the 48-byte 16-color palette, reserved
//! byte, plane count, bytes per line, and palette info. Image data
//! follows at offset 128; RLE runs are `0xC0|count` + value, any other
//! byte is literal.
//!
//! ```
//! use izanagi_kit::pcx::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER];
//! d[0] = 0x0A;                    // maker
//! d[1] = 5;                       // version 3.0
//! d[2] = 1;                       // RLE
//! d[3] = 8;                       // 8 bits/pixel/plane
//! d[4..6].copy_from_slice(&0u16.to_le_bytes());
//! d[6..8].copy_from_slice(&0u16.to_le_bytes());
//! d[8..10].copy_from_slice(&3u16.to_le_bytes());  // xmax -> w 4
//! d[10..12].copy_from_slice(&1u16.to_le_bytes()); // ymax -> h 2
//! d[65] = 1;                      // 1 plane
//! d[66..68].copy_from_slice(&4u16.to_le_bytes()); // bytes/line
//! // 8 pixels: literal 1, run of 3 sevens, literal 9, run 3 x 2
//! d.extend_from_slice(&[1, 0xC3, 7, 9, 0xC3, 2]);
//! let p = parse(&d).unwrap();
//! assert_eq!((p.width(), p.height()), (4, 2));
//! assert_eq!(p.decode(&d).unwrap(), vec![1, 7, 7, 7, 9, 2, 2, 2]);
//! ```

use std::vec::Vec;

/// Header size in bytes.
pub const HEADER: usize = 128;
/// Maker byte (`0x0A`).
pub const MAKER: u8 = 0x0A;
/// Encoding value for RLE data.
pub const ENCODING_RLE: u8 = 1;
/// RLE marker top bits.
pub const RLE_TAG: u8 = 0xC0;

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

/// A parsed PCX header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pcx {
    /// Version byte (0=v2.5, 2=v2.8 pal, 3=v2.8 no pal, 4=win, 5=v3+).
    pub version: u8,
    /// Encoding byte (`1` = RLE).
    pub encoding: u8,
    /// Bits per pixel per plane.
    pub bits_per_pixel: u8,
    /// Left edge.
    pub xmin: u16,
    /// Top edge.
    pub ymin: u16,
    /// Right edge (inclusive).
    pub xmax: u16,
    /// Bottom edge (inclusive).
    pub ymax: u16,
    /// Horizontal resolution of the source device.
    pub hres: u16,
    /// Vertical resolution of the source device.
    pub vres: u16,
    /// 16-color palette (48 bytes of RGB triplets).
    pub palette: [u8; 48],
    /// Number of color planes.
    pub planes: u8,
    /// Bytes per scanline per plane.
    pub bytes_per_line: u16,
    /// Palette info (`1` = color/BW, `2` = grayscale).
    pub palette_info: u16,
}

impl Pcx {
    /// Image width (`xmax - xmin + 1`).
    pub fn width(&self) -> u32 {
        u32::from(self.xmax) - u32::from(self.xmin) + 1
    }

    /// Image height (`ymax - ymin + 1`).
    pub fn height(&self) -> u32 {
        u32::from(self.ymax) - u32::from(self.ymin) + 1
    }

    /// Total decoded raster bytes (`planes * bytes_per_line * height`).
    pub fn raster_len(&self) -> usize {
        usize::from(self.planes)
            .saturating_mul(usize::from(self.bytes_per_line))
            .saturating_mul(usize::try_from(self.height()).unwrap_or(u32::MAX as usize))
    }

    /// Decode the RLE raster after the header. Returns `None` when a
    /// run marker is the last byte, the decoded output is shorter than
    /// `raster_len`, or `encoding` is not RLE.
    pub fn decode(&self, d: &[u8]) -> Option<Vec<u8>> {
        if self.encoding != ENCODING_RLE {
            return None;
        }
        let want = self.raster_len();
        let mut out = Vec::with_capacity(want);
        let mut i = HEADER;
        while out.len() < want {
            let b = *d.get(i)?;
            i += 1;
            if b & RLE_TAG == RLE_TAG {
                let n = usize::from(b & !RLE_TAG);
                let v = *d.get(i)?;
                i += 1;
                for _ in 0..n {
                    if out.len() < want {
                        out.push(v);
                    }
                }
            } else {
                out.push(b);
            }
        }
        Some(out)
    }
}

/// Parse a PCX header. Returns `None` on a bad maker byte or a buffer
/// shorter than 128 bytes.
pub fn parse(d: &[u8]) -> Option<Pcx> {
    if *d.first()? != MAKER {
        return None;
    }
    let mut palette = [0u8; 48];
    palette.copy_from_slice(d.get(16..64)?);
    Some(Pcx {
        version: *d.get(1)?,
        encoding: *d.get(2)?,
        bits_per_pixel: *d.get(3)?,
        xmin: u16le(d, 4)?,
        ymin: u16le(d, 6)?,
        xmax: u16le(d, 8)?,
        ymax: u16le(d, 10)?,
        hres: u16le(d, 12)?,
        vres: u16le(d, 14)?,
        palette,
        planes: *d.get(65)?,
        bytes_per_line: u16le(d, 66)?,
        palette_info: u16le(d, 68)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[0] = MAKER;
        d[1] = 5;
        d[2] = ENCODING_RLE;
        d[3] = 8;
        d[8..10].copy_from_slice(&3u16.to_le_bytes());
        d[10..12].copy_from_slice(&1u16.to_le_bytes());
        d[12..14].copy_from_slice(&640u16.to_le_bytes());
        d[14..16].copy_from_slice(&480u16.to_le_bytes());
        d[16..19].copy_from_slice(&[0xFF, 0, 0]);
        d[65] = 1;
        d[66..68].copy_from_slice(&4u16.to_le_bytes());
        d[68..70].copy_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&[1, 0xC3, 7, 9, 0xC3, 2]);
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let p = parse(&image()).unwrap();
        assert_eq!(p.version, 5);
        assert_eq!(p.encoding, 1);
        assert_eq!(p.bits_per_pixel, 8);
        assert_eq!((p.xmin, p.ymin, p.xmax, p.ymax), (0, 0, 3, 1));
        assert_eq!((p.hres, p.vres), (640, 480));
        assert_eq!(p.palette[0], 0xFF);
        assert_eq!(p.planes, 1);
        assert_eq!(p.bytes_per_line, 4);
        assert_eq!(p.palette_info, 1);
        assert_eq!((p.width(), p.height()), (4, 2));
        assert_eq!(p.raster_len(), 8);
    }

    #[test]
    fn decode_rle() {
        let d = image();
        let p = parse(&d).unwrap();
        assert_eq!(p.decode(&d).unwrap(), vec![1, 7, 7, 7, 9, 2, 2, 2]);
    }

    #[test]
    fn decode_literal_marker_value() {
        // a literal byte >= 0xC0 must be escaped as a 1-byte run
        let mut d = image();
        d.truncate(HEADER);
        d.extend_from_slice(&[0xC1, 0xC5, 0xC7, 0x40]);
        let p = parse(&d).unwrap();
        assert_eq!(
            p.decode(&d).unwrap(),
            vec![0xC5, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40]
        );
    }

    #[test]
    fn decode_rejects_truncated_and_non_rle() {
        let mut d = image();
        d.truncate(HEADER);
        d.push(0xC3); // run marker without a value
        let p = parse(&d).unwrap();
        assert_eq!(p.decode(&d), None);
        let mut d2 = image();
        d2[2] = 0; // not RLE
        assert_eq!(parse(&d2).unwrap().decode(&d2), None);
        let mut d3 = image();
        d3.truncate(HEADER);
        assert_eq!(p.decode(&d3), None);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        let mut d = image();
        d[0] = 0x09;
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn constants() {
        assert_eq!(HEADER, 128);
        assert_eq!(MAKER, 0x0A);
        assert_eq!(ENCODING_RLE, 1);
        assert_eq!(RLE_TAG, 0xC0);
    }
}

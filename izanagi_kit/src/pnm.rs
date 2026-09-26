//! Netpbm portable-anything images (`P1`..`P6`).
//!
//! `P1`/`P4` are PBM bitmaps, `P2`/`P5` PGM grayscale, `P3`/`P6` PPM
//! color; `P1`–`P3` carry ASCII values, `P4`–`P6` raw bytes. The
//! header is whitespace-separated tokens — magic, width, height,
//! `maxval` (not present for PBM) — with `#` comments allowed between
//! tokens; one whitespace byte separates the header from raw data.
//!
//! ```
//! use izanagi_kit::pnm::{parse, Kind};
//!
//! let d = b"P5\n# a comment\n4 2\n255\n\x01\x02\x03\x04\x05\x06\x07\x08";
//! let p = parse(d).unwrap();
//! assert_eq!(p.kind, Kind::Pgm);
//! assert!(!p.ascii);
//! assert_eq!((p.width, p.height, p.maxval), (4, 2, 255));
//! assert_eq!(p.pixels(d).unwrap().len(), 8);
//! ```

/// Netpbm variant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `P1`/`P4` — bilevel bitmap (1 bit or 0/1 value).
    Pbm,
    /// `P2`/`P5` — grayscale.
    Pgm,
    /// `P3`/`P6` — RGB.
    Ppm,
}

/// A parsed Netpbm header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pnm {
    /// Magic digit (1..=6).
    pub magic: u8,
    /// Image kind.
    pub kind: Kind,
    /// True for the ASCII flavors (`P1`/`P2`/`P3`).
    pub ascii: bool,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Maximum sample value (1 for PBM).
    pub maxval: u32,
    /// Offset of the raster data in the input.
    pub data_at: usize,
}

impl Pnm {
    /// Bytes per pixel in raw form (PPM = 3, maxval > 255 = 2-byte
    /// big-endian samples → doubled).
    pub fn pixel_bytes(&self) -> usize {
        let chans = if self.kind == Kind::Ppm { 3 } else { 1 };
        let wide = if self.maxval > 255 { 2 } else { 1 };
        chans * wide
    }

    /// Bytes the raw raster occupies (PBM packs 8 pixels per byte).
    pub fn raster_len(&self) -> usize {
        let w = usize::try_from(self.width).unwrap_or(u32::MAX as usize);
        let h = usize::try_from(self.height).unwrap_or(u32::MAX as usize);
        if self.kind == Kind::Pbm {
            (w.saturating_add(7) / 8).saturating_mul(h)
        } else {
            w.saturating_mul(h).saturating_mul(self.pixel_bytes())
        }
    }

    /// The raw raster bytes for `P4`–`P6` inside `d` (ASCII flavors
    /// return `None` — their raster is text).
    pub fn pixels<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        if self.ascii {
            return None;
        }
        d.get(self.data_at..self.data_at + self.raster_len())
    }
}

fn tok(d: &[u8], mut i: usize) -> Option<(&[u8], usize)> {
    loop {
        match *d.get(i)? {
            b'#' => {
                while i < d.len() && d[i] != b'\n' {
                    i += 1;
                }
            }
            b' ' | b'\t' | b'\r' | b'\n' | 0x0B | 0x0C => i += 1,
            _ => break,
        }
    }
    let start = i;
    while i < d.len() && !matches!(d[i], b' ' | b'\t' | b'\r' | b'\n' | 0x0B | 0x0C | b'#') {
        i += 1;
    }
    Some((d.get(start..i)?, i))
}

fn u32_tok(d: &[u8], i: usize) -> Option<(u32, usize)> {
    let (t, i) = tok(d, i)?;
    let s = core::str::from_utf8(t).ok()?;
    Some((s.parse().ok()?, i))
}

/// Parse a Netpbm header. Returns `None` on an unknown magic, a
/// missing/invalid field, or a truncated raster region.
pub fn parse(d: &[u8]) -> Option<Pnm> {
    if d.first() != Some(&b'P') {
        return None;
    }
    let magic = *d.get(1)?;
    if !(1..=6).contains(&magic.wrapping_sub(b'0')) {
        return None;
    }
    let digit = magic - b'0';
    let (kind, ascii) = match digit {
        1 | 4 => (Kind::Pbm, digit == 1),
        2 | 5 => (Kind::Pgm, digit == 2),
        _ => (Kind::Ppm, digit == 3),
    };
    let mut i = 2;
    let (width, j) = u32_tok(d, i)?;
    i = j;
    let (height, j) = u32_tok(d, i)?;
    i = j;
    let maxval = if kind == Kind::Pbm {
        1
    } else {
        let (v, j) = u32_tok(d, i)?;
        i = j;
        v
    };
    if !ascii {
        // exactly one whitespace byte separates header from raw data
        i += 1;
    }
    let p = Pnm {
        magic: digit,
        kind,
        ascii,
        width,
        height,
        maxval,
        data_at: i,
    };
    if !ascii && p.raster_len() > 0 {
        d.get(i..i + p.raster_len())?;
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgm_raw() {
        let d = b"P5\n4 2\n255\n\x01\x02\x03\x04\x05\x06\x07\x08";
        let p = parse(d).unwrap();
        assert_eq!(p.magic, 5);
        assert_eq!(p.kind, Kind::Pgm);
        assert!(!p.ascii);
        assert_eq!(p.width, 4);
        assert_eq!(p.height, 2);
        assert_eq!(p.maxval, 255);
        assert_eq!(p.raster_len(), 8);
        assert_eq!(p.pixels(d), Some(&d[d.len() - 8..]));
    }

    #[test]
    fn ppm_16bit_raster_len() {
        let d = b"P6\n2 1\n65535\n";
        let mut dd = d.to_vec();
        dd.extend_from_slice(&[0; 12]);
        let p = parse(&dd).unwrap();
        assert_eq!(p.kind, Kind::Ppm);
        assert_eq!(p.pixel_bytes(), 6);
        assert_eq!(p.raster_len(), 12);
        assert_eq!(p.pixels(&dd).unwrap().len(), 12);
    }

    #[test]
    fn pbm_packed_bits() {
        // P4 16x2 -> 2 bytes/row -> 4 bytes
        let d = b"P4\n16 2\n\xF0\x0F\xAA\x55";
        let p = parse(d).unwrap();
        assert_eq!(p.kind, Kind::Pbm);
        assert_eq!(p.maxval, 1);
        assert_eq!(p.raster_len(), 4);
        assert_eq!(p.pixels(d), Some(b"\xF0\x0F\xAA\x55".as_ref()));
    }

    #[test]
    fn ascii_header_has_no_raster() {
        let p = parse(b"P3 2 1 255 255 0 0 0 255 0").unwrap();
        assert!(p.ascii);
        assert_eq!(p.pixels(b""), None);
        let p2 = parse(b"P2 1 1 9 5").unwrap();
        assert_eq!(p2.magic, 2);
        assert_eq!(p2.kind, Kind::Pgm);
        let p1 = parse(b"P1 1 1 0").unwrap();
        assert_eq!(p1.kind, Kind::Pbm);
    }

    #[test]
    fn comments_everywhere() {
        let p = parse(b"P5#c\n 4 #x\n 2\n255\n\x00\x01\x02\x03\x04\x05\x06\x07").unwrap();
        assert_eq!((p.width, p.height), (4, 2));
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(b""), None);
        assert_eq!(parse(b"X5\n1 1\n255\n"), None);
        assert_eq!(parse(b"P7\n1 1\n"), None);
        assert_eq!(parse(b"P0\n1 1\n"), None);
        assert_eq!(parse(b"P5\n4 2\n255\n\x01"), None); // short raster
        assert_eq!(parse(b"P5\nx 2\n255\n"), None); // bad number
    }
}

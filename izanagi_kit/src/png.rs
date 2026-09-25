//! PNG image codec (ISO/IEC 15948 / W3C PNG) — decode 8-bit grayscale,
//! palette, RGB, gray+alpha, and RGBA rasters, and encode 8-bit
//! grayscale/RGB. Builds directly on [`crate::inflate`],
//! [`crate::deflate`], and [`crate::crc`].
//!
//! Scope: bit depth 8 and no interlace only — enough for the common
//! `png`-in-the-wild shape and for this crate's own encoder. Adam7 and
//! 16-bit rasters degrade to `None`. Chunk CRCs are verified on decode
//! and generated on encode.
//!
//! ```
//! use izanagi_kit::png::{encode_gray, decode, Color};
//!
//! // A 2×2 checkerboard round-trips through a real PNG.
//! let px = [0u8, 255, 255, 0];
//! let bytes = encode_gray(2, 2, &px);
//! let img = decode(&bytes).unwrap();
//! assert_eq!(img.w, 2);
//! assert_eq!(img.color, Color::Gray);
//! assert_eq!(img.pixels, px);
//! ```

use std::vec::Vec;

/// PNG color type from IHDR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    /// 1 channel.
    Gray,
    /// 3 channels.
    Rgb,
    /// 1 channel, index into PLTE.
    Palette,
    /// 2 channels (gray + alpha).
    GrayAlpha,
    /// 4 channels.
    Rgba,
}

impl Color {
    fn from_tag(t: u8) -> Option<Self> {
        Some(match t {
            0 => Self::Gray,
            2 => Self::Rgb,
            3 => Self::Palette,
            4 => Self::GrayAlpha,
            6 => Self::Rgba,
            _ => return None,
        })
    }
    fn tag(self) -> u8 {
        match self {
            Self::Gray => 0,
            Self::Rgb => 2,
            Self::Palette => 3,
            Self::GrayAlpha => 4,
            Self::Rgba => 6,
        }
    }
    /// Bytes per pixel at bit depth 8.
    pub fn bpp(self) -> usize {
        match self {
            Self::Gray | Self::Palette => 1,
            Self::Rgb => 3,
            Self::GrayAlpha => 2,
            Self::Rgba => 4,
        }
    }
}

/// A decoded PNG: `pixels` is `w * h * bpp` channel bytes, row-major.
pub struct Png {
    /// Image width in pixels.
    pub w: u32,
    /// Image height in pixels.
    pub h: u32,
    /// Color type.
    pub color: Color,
    /// Channel bytes. Palette images stay indexed (`bpp` = 1); consult
    /// `palette` to map them to RGB.
    pub pixels: Vec<u8>,
    /// PLTE entries (RGB triples) — empty unless `color` is `Palette`.
    pub palette: Vec<u8>,
}

fn be32(b: &[u8]) -> u32 {
    ((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | b[3] as u32
}

fn push_be32(out: &mut Vec<u8>, v: u32) {
    out.push((v >> 24) as u8);
    out.push((v >> 16) as u8);
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

const SIG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (a, b, c) = (a as i32, b as i32, c as i32);
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

/// Decode a PNG byte stream. Returns `None` on any malformed chunk,
/// bad CRC, unsupported bit depth/interlace, or truncated IDAT.
pub fn decode(data: &[u8]) -> Option<Png> {
    if data.len() < 8 || data[..8] != SIG {
        return None;
    }
    let mut at = 8usize;
    let mut w = 0u32;
    let mut h = 0u32;
    let mut color = Color::Gray;
    let mut palette = Vec::new();
    let mut idat = Vec::new();
    let mut seen_ihdr = false;
    let mut seen_iend = false;
    while at + 12 <= data.len() {
        let len = be32(&data[at..]) as usize;
        let ty = &data[at + 4..at + 8];
        let (body, crc_at) = (at + 8, at + 8 + len);
        if crc_at + 4 > data.len() {
            return None;
        }
        let want = be32(&data[crc_at..]);
        // CRC covers type tag + body.
        let mut c = crate::crc::Crc32::new();
        c.write(&data[at + 4..crc_at]);
        if c.finish() != want {
            return None;
        }
        match ty {
            b"IHDR" => {
                if seen_ihdr || len != 13 {
                    return None;
                }
                seen_ihdr = true;
                let ih = &data[body..body + 13];
                w = be32(&ih[0..]);
                h = be32(&ih[4..]);
                let (depth, ctag, comp, filt, inter) = (ih[8], ih[9], ih[10], ih[11], ih[12]);
                color = Color::from_tag(ctag)?;
                if depth != 8 || comp != 0 || filt != 0 || inter != 0 || w == 0 || h == 0 {
                    return None;
                }
            }
            b"PLTE" => palette = data[body..body + len].to_vec(),
            b"IDAT" => idat.extend_from_slice(&data[body..body + len]),
            b"IEND" => {
                seen_iend = true;
                break;
            }
            _ => {} // ancillary chunks are skippable
        }
        at = crc_at + 4;
    }
    if !seen_ihdr || !seen_iend {
        return None;
    }
    let raw = crate::inflate::inflate_zlib(&idat)?;
    let (w, h) = (w as usize, h as usize);
    let bpp = color.bpp();
    let stride = w.checked_mul(bpp)?;
    if raw.len() != (stride + 1) * h {
        return None;
    }
    // Unfilter in place over rows.
    let mut pixels = vec![0u8; stride * h];
    for y in 0..h {
        let f = raw[y * (stride + 1)];
        let src = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let (done, cur) = pixels.split_at_mut(y * stride);
        let row = &mut cur[..stride];
        let up = if y > 0 {
            Some(&done[stride * (y - 1)..])
        } else {
            None
        };
        match f {
            0 => row.copy_from_slice(src),
            1 => {
                for x in 0..stride {
                    let a = if x >= bpp { row[x - bpp] } else { 0 };
                    row[x] = src[x].wrapping_add(a);
                }
            }
            2 => {
                for x in 0..stride {
                    row[x] = src[x].wrapping_add(up.map_or(0, |u| u[x]));
                }
            }
            3 => {
                for x in 0..stride {
                    let a = if x >= bpp { row[x - bpp] as u32 } else { 0 };
                    let b = up.map_or(0, |u| u[x] as u32);
                    row[x] = src[x].wrapping_add(((a + b) / 2) as u8);
                }
            }
            4 => {
                for x in 0..stride {
                    let a = if x >= bpp { row[x - bpp] } else { 0 };
                    let b = up.map_or(0, |u| u[x]);
                    let c = if x >= bpp {
                        up.map_or(0, |u| u[x - bpp])
                    } else {
                        0
                    };
                    row[x] = src[x].wrapping_add(paeth(a, b, c));
                }
            }
            _ => return None,
        }
    }
    Some(Png {
        w: w as u32,
        h: h as u32,
        color,
        pixels,
        palette,
    })
}

fn chunk(out: &mut Vec<u8>, ty: &[u8; 4], body: &[u8]) {
    push_be32(out, body.len() as u32);
    out.extend_from_slice(ty);
    out.extend_from_slice(body);
    let mut c = crate::crc::Crc32::new();
    c.write(ty);
    c.write(body);
    push_be32(out, c.finish());
}

fn encode(w: u32, h: u32, color: Color, pixels: &[u8]) -> Vec<u8> {
    let bpp = color.bpp();
    let stride = w as usize * bpp;
    let mut raw = Vec::with_capacity((stride + 1) * h as usize);
    for y in 0..h as usize {
        raw.push(0); // filter: none — correctness over density
        raw.extend_from_slice(&pixels[y * stride..(y + 1) * stride]);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&SIG);
    let mut ihdr = Vec::with_capacity(13);
    push_be32(&mut ihdr, w);
    push_be32(&mut ihdr, h);
    ihdr.extend_from_slice(&[8, color.tag(), 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &crate::deflate::deflate_zlib(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Encode an 8-bit grayscale image (`pixels.len()` must be `w*h`).
/// Returns an empty vector on size mismatch rather than panicking.
pub fn encode_gray(w: u32, h: u32, pixels: &[u8]) -> Vec<u8> {
    if pixels.len() != w as usize * h as usize {
        return Vec::new();
    }
    encode(w, h, Color::Gray, pixels)
}

/// Encode an 8-bit RGB image (`pixels.len()` must be `w*h*3`).
pub fn encode_rgb(w: u32, h: u32, pixels: &[u8]) -> Vec<u8> {
    if pixels.len() != w as usize * h as usize * 3 {
        return Vec::new();
    }
    encode(w, h, Color::Rgb, pixels)
}

/// Encode an 8-bit RGBA image (`pixels.len()` must be `w*h*4`).
pub fn encode_rgba(w: u32, h: u32, pixels: &[u8]) -> Vec<u8> {
    if pixels.len() != w as usize * h as usize * 4 {
        return Vec::new();
    }
    encode(w, h, Color::Rgba, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gray_roundtrip() {
        let px: Vec<u8> = (0..16u8).map(|v| v * 16).collect();
        let bytes = encode_gray(4, 4, &px);
        let img = decode(&bytes).unwrap();
        assert_eq!((img.w, img.h, img.color), (4, 4, Color::Gray));
        assert_eq!(img.pixels, px);
    }

    #[test]
    fn rgba_roundtrip() {
        let px: Vec<u8> = (0..48u8).map(|v| v.wrapping_mul(5)).collect();
        let bytes = encode_rgba(4, 3, &px);
        let img = decode(&bytes).unwrap();
        assert_eq!(img.color, Color::Rgba);
        assert_eq!(img.pixels, px);
    }

    #[test]
    fn rgb_roundtrip_and_bpp() {
        assert_eq!(Color::Gray.bpp(), 1);
        assert_eq!(Color::Rgb.bpp(), 3);
        assert_eq!(Color::Palette.bpp(), 1);
        assert_eq!(Color::GrayAlpha.bpp(), 2);
        assert_eq!(Color::Rgba.bpp(), 4);
        let px: Vec<u8> = (0..24u8).map(|v| v.wrapping_mul(7)).collect();
        let bytes = encode_rgb(4, 2, &px);
        let img = decode(&bytes).unwrap();
        assert_eq!(img.color, Color::Rgb);
        assert_eq!(img.pixels, px);
    }

    #[test]
    fn real_png_decodes() {
        // 2×2 RGB PNG produced by Python zlib + hand-rolled chunks.
        // Pixels: red, green / blue, white.
        let bytes: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00,
            0x00, 0xfd, 0xd4, 0x9a, 0x73, 0x00, 0x00, 0x00, 0x12, 0x49, 0x44, 0x41, 0x54, 0x78,
            0xda, 0x63, 0xf8, 0xcf, 0xc0, 0xc0, 0x00, 0xc2, 0x0c, 0xff, 0x81, 0x00, 0x00, 0x1f,
            0xee, 0x05, 0xfb, 0xf1, 0xab, 0xba, 0x77, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
            0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        let img = decode(bytes).unwrap();
        assert_eq!((img.w, img.h, img.color), (2, 2, Color::Rgb));
        assert_eq!(img.pixels, [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]);
    }

    #[test]
    fn bad_crc_rejected() {
        let px = [1u8, 2, 3, 4];
        let mut bytes = encode_gray(2, 2, &px);
        let n = bytes.len();
        bytes[n - 1] ^= 0xFF; // corrupt IEND crc
        assert!(decode(&bytes).is_none());
    }

    #[test]
    fn malformed_degrades() {
        assert!(decode(b"").is_none());
        assert!(decode(&SIG).is_none());
        assert!(decode(b"not a png").is_none());
        assert!(encode_gray(3, 3, &[0u8; 4]).is_empty());
    }
}

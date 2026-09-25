//! Windows BMP codec — uncompressed `BI_RGB` bitmaps, 24- and 32-bit.
//!
//! The deterministic side of [`png`](crate::png)/[`gif`](crate::gif)/
//! [`qoi`](crate::qoi): BMP stores raw BGR rows padded to a 4-byte
//! stride, bottom-up unless the height field is negative (top-down).
//! [`encode`] always emits a canonical 24-bit bottom-up file; [`decode`]
//! accepts 24/32-bit, top-down or bottom-up, and rejects anything else
//! (compression, bitfields, palettes, planes != 1, w <= 0).
//!
//! ```
//! use izanagi_kit::bmp::{Bmp, decode, encode};
//! let img = Bmp { w: 2, h: 1, px: vec![[255, 0, 0, 255], [0, 255, 0, 255]] };
//! let bytes = encode(&img);
//! assert_eq!(decode(&bytes).unwrap().px, img.px);
//! ```

/// A decoded image: `w`*`h` RGBA pixels, row-major from the top-left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bmp {
    /// Width in pixels (> 0).
    pub w: u32,
    /// Height in pixels (> 0).
    pub h: u32,
    /// `w*h` pixels as `[r, g, b, a]`.
    pub px: Vec<[u8; 4]>,
}

fn le16(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(i)?, *d.get(i + 1)?]))
}

fn le32(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(i)?,
        *d.get(i + 1)?,
        *d.get(i + 2)?,
        *d.get(i + 3)?,
    ]))
}

/// Encode `img` as a canonical 24-bit bottom-up BMP.
pub fn encode(img: &Bmp) -> Vec<u8> {
    let w = img.w.max(1);
    let h = img.h.max(1);
    let stride = (w * 3).div_ceil(4) * 4;
    let img_size = stride * h;
    let mut out = Vec::with_capacity(54 + img_size as usize);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(54 + img_size).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // reserved
    out.extend_from_slice(&54u32.to_le_bytes()); // pixel offset
    out.extend_from_slice(&40u32.to_le_bytes()); // DIB size
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes()); // positive = bottom-up
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&24u16.to_le_bytes()); // bpp
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&img_size.to_le_bytes());
    out.extend_from_slice(&2835i32.to_le_bytes()); // ~72 dpi
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // palette
    out.extend_from_slice(&0u32.to_le_bytes()); // important
    for y in (0..h).rev() {
        for x in 0..w {
            let [r, g, b, _] = img.px[(y * w + x) as usize];
            out.extend_from_slice(&[b, g, r]);
        }
        let pad = stride - w * 3;
        out.resize(out.len() + pad as usize, 0);
    }
    out
}

/// Decode a BMP into RGBA. `None` on any malformed or unsupported file.
pub fn decode(d: &[u8]) -> Option<Bmp> {
    if d.len() < 54 || d[0] != b'B' || d[1] != b'M' {
        return None;
    }
    let off = le32(d, 10)? as usize;
    let dib = le32(d, 14)?;
    if dib < 40 {
        return None;
    }
    let w = le32(d, 18)? as i32;
    let h_raw = le32(d, 22)? as i32;
    let planes = le16(d, 26)?;
    let bpp = le16(d, 28)?;
    let comp = le32(d, 30)?;
    if w <= 0 || h_raw == 0 || planes != 1 || comp != 0 {
        return None;
    }
    if bpp != 24 && bpp != 32 {
        return None;
    }
    let w = w as u32;
    let h = h_raw.unsigned_abs();
    let top_down = h_raw < 0;
    let stride = if bpp == 24 {
        (w as u64 * 3).div_ceil(4) * 4
    } else {
        w as u64 * 4
    };
    let end = off.checked_add((stride * h as u64) as usize)?;
    if end > d.len() {
        return None;
    }
    let mut px = vec![[0u8; 4]; (w * h) as usize];
    for row in 0..h as usize {
        let src = off + row * stride as usize;
        let y = if top_down { row } else { h as usize - 1 - row };
        for x in 0..w as usize {
            let i = src + x * (bpp as usize / 8);
            let (b, g, r, a) = if bpp == 24 {
                (d[i], d[i + 1], d[i + 2], 255)
            } else {
                (d[i], d[i + 1], d[i + 2], d[i + 3])
            };
            px[y * w as usize + x] = [r, g, b, a];
        }
    }
    Some(Bmp { w, h, px })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let img = Bmp {
            w: 3,
            h: 2,
            px: vec![
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                [9, 8, 7, 255],
                [6, 5, 4, 255],
                [3, 2, 1, 255],
            ],
        };
        assert_eq!(decode(&encode(&img)).unwrap(), img);
    }

    #[test]
    fn canonical_two_by_one_vector() {
        // Red then green pixel, one row: "BM", size 62, off 54.
        let img = Bmp {
            w: 2,
            h: 1,
            px: vec![[255, 0, 0, 255], [0, 255, 0, 255]],
        };
        let b = encode(&img);
        assert_eq!(&b[..2], b"BM");
        assert_eq!(u32::from_le_bytes(b[2..6].try_into().unwrap()), 62);
        assert_eq!(&b[54..60], &[0, 0, 255, 0, 255, 0]); // BGR order
        assert_eq!(&b[60..62], &[0, 0]); // stride pad
    }

    #[test]
    fn decodes_32bpp_and_top_down() {
        // 1x2 32-bit top-down: top pixel magenta, bottom cyan.
        let mut d = Vec::new();
        d.extend_from_slice(b"BM");
        d.extend_from_slice(&62u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&54u32.to_le_bytes());
        d.extend_from_slice(&40u32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&(-2i32).to_le_bytes()); // top-down
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&32u16.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&[0; 16]);
        d.extend_from_slice(&[255, 0, 255, 200]); // magenta BGRA
        d.extend_from_slice(&[255, 255, 0, 128]); // cyan BGRA
        let img = decode(&d).unwrap();
        assert_eq!(img.px[0], [255, 0, 255, 200]);
        assert_eq!(img.px[1], [0, 255, 255, 128]);
    }

    #[test]
    fn malformed_inputs_rejected() {
        assert!(decode(&[]).is_none());
        let mut ok = encode(&Bmp {
            w: 1,
            h: 1,
            px: vec![[1, 2, 3, 4]],
        });
        let mut bad = ok.clone();
        bad[0] = b'Z';
        assert!(decode(&bad).is_none());
        ok.pop();
        assert!(decode(&ok).is_none()); // truncated pixels
        let mut c = encode(&Bmp {
            w: 1,
            h: 1,
            px: vec![[0; 4]],
        });
        c[30] = 1; // compression != BI_RGB
        assert!(decode(&c).is_none());
        let mut b = encode(&Bmp {
            w: 1,
            h: 1,
            px: vec![[0; 4]],
        });
        b[28] = 8; // 8 bpp needs a palette -> unsupported
        assert!(decode(&b).is_none());
    }
}

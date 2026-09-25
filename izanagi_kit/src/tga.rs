//! TGA (Targa) images — the `bmp`/`png`/`qoi` family's other classic:
//! an 18-byte header then BGR(A) pixel data, uncompressed (type 2) or
//! packetized RLE (type 10). [`decode`] yields `Tga { w, h, px }` with
//! `w*h` RGBA pixels row-major from the top-left (the descriptor's
//! origin bits are honoured); [`encode`] writes canonical type-2 32bpp
//! top-left-origin.
//!
//! ```
//! use izanagi_kit::tga::{Tga, encode, decode};
//! let img = Tga { w: 2, h: 1, px: vec![[255, 0, 0, 255], [0, 255, 0, 255]] };
//! assert_eq!(decode(&encode(&img)).unwrap(), img);
//! ```

use std::vec::Vec;

/// A decoded image: `w`*`h` RGBA pixels, row-major from the top-left.
#[derive(Clone, Debug, PartialEq)]
pub struct Tga {
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

/// Canonical encode: type-2 uncompressed, 32bpp, top-left origin.
/// `img.px` is used in order (row-major from top-left); missing pixels
/// encode as transparent black.
pub fn encode(img: &Tga) -> Vec<u8> {
    let mut out = vec![
        0, // no id field
        0, // no colormap
        2, // uncompressed truecolor
        0, 0, 0, 0, 0, // colormap spec
        0, 0, 0, 0, // x/y origin
    ];
    out.extend_from_slice(&(img.w as u16).to_le_bytes());
    out.extend_from_slice(&(img.h as u16).to_le_bytes());
    out.push(32); // bpp
    out.push(0x28); // descriptor: top-origin + 8 alpha bits
    for i in 0..(img.w as u64 * img.h as u64) as usize {
        let p = img.px.get(i).copied().unwrap_or([0, 0, 0, 0]);
        out.extend_from_slice(&[p[2], p[1], p[0], p[3]]); // BGRA
    }
    out
}

/// Decode type 2/10 truecolor TGAs; `None` on colormapped/grayscale
/// types, truncated data, or zero dimensions. Both RLE packet kinds
/// are supported; images may be left- or right-origin and top- or
/// bottom-origin — output is always top-left-first.
pub fn decode(d: &[u8]) -> Option<Tga> {
    if d.len() < 18 {
        return None;
    }
    let (idlen, cmap, ty) = (d[0] as usize, d[1], d[2]);
    if cmap != 0 || (ty != 2 && ty != 10) {
        return None;
    }
    let (w, h) = (le16(d, 12)? as u32, le16(d, 14)? as u32);
    let (bpp, desc) = (d[16], d[17]);
    if w == 0 || h == 0 || (bpp != 24 && bpp != 32) {
        return None;
    }
    let top = desc & 0x20 != 0;
    let right = desc & 0x10 != 0;
    let bytes = (bpp / 8) as usize;
    let mut pos = 18usize.checked_add(idlen)?;
    let n = (w as u64 * h as u64) as usize;
    let mut px = vec![[0u8; 4]; n];
    let mut put = |idx: usize, bgra: &[u8]| {
        let (col, row) = (idx % w as usize, idx / w as usize);
        let x = if right { w as usize - 1 - col } else { col };
        let y = if top { row } else { h as usize - 1 - row };
        px[y * w as usize + x] = [bgra[2], bgra[1], bgra[0], *bgra.get(3).unwrap_or(&255)];
    };
    let mut idx = 0usize;
    if ty == 2 {
        for i in 0..n {
            let p = pos.checked_add(i.checked_mul(bytes)?)?;
            let b = d.get(p..p.checked_add(bytes)?)?;
            put(i, b);
        }
    } else {
        while idx < n {
            let hdr = *d.get(pos)? as usize;
            pos += 1;
            let (count, rle) = ((hdr & 0x7F) + 1, hdr & 0x80 != 0);
            if rle {
                let b = d.get(pos..pos.checked_add(bytes)?)?;
                pos += bytes;
                for _ in 0..count {
                    if idx >= n {
                        return None; // packet overruns the image
                    }
                    put(idx, b);
                    idx += 1;
                }
            } else {
                for _ in 0..count {
                    let b = d.get(pos..pos.checked_add(bytes)?)?;
                    pos += bytes;
                    if idx >= n {
                        return None;
                    }
                    put(idx, b);
                    idx += 1;
                }
            }
        }
    }
    Some(Tga { w, h, px })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img() -> Tga {
        Tga {
            w: 3,
            h: 2,
            px: vec![
                [255, 0, 0, 255],
                [0, 255, 0, 200],
                [0, 0, 255, 128],
                [1, 2, 3, 4],
                [9, 8, 7, 6],
                [250, 251, 252, 253],
            ],
        }
    }

    #[test]
    fn encode_roundtrip() {
        let i = img();
        let d = encode(&i);
        assert_eq!(d.len(), 18 + 6 * 4);
        assert_eq!(d[2], 2);
        assert_eq!(d[16], 32);
        assert_eq!(d[17], 0x28);
        assert_eq!(decode(&d).unwrap(), i);
    }

    #[test]
    fn decode_rle() {
        // type 10: one RLE packet of 3 red + raw packet of 2 blue
        let mut d = vec![
            0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, // header start
            5, 0, 1, 0, // w=5 h=1
            24, 0x20,
        ];
        d.push(0x82); // RLE packet, count 3
        d.extend_from_slice(&[0, 0, 255]); // BGR red
        d.push(0x01); // raw packet, count 2
        d.extend_from_slice(&[255, 0, 0]); // BGR blue? B=255 → blue
        d.extend_from_slice(&[255, 0, 0]);
        let t = decode(&d).unwrap();
        assert_eq!(t.w, 5);
        let reds = t.px.iter().filter(|p| p[0] == 255).count();
        let blues = t.px.iter().filter(|p| p[2] == 255).count();
        assert_eq!((reds, blues), (3, 2));
        for p in &t.px {
            assert_eq!(p[3], 255); // 24bpp → opaque
        }
    }

    #[test]
    fn origin_flip() {
        // bottom-left origin (desc=0) — rows stored bottom-up
        let mut d = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 24, 0x00];
        // stored: bottom row first — bottom = [9,8,7], top = [255,0,0]
        d.extend_from_slice(&[7, 8, 9]); // bottom-left BGR
        d.extend_from_slice(&[0, 0, 255]); // top-left BGR
        let t = decode(&d).unwrap();
        assert_eq!(t.px[0], [255, 0, 0, 255]); // top-left first
        assert_eq!(t.px[1], [9, 8, 7, 255]);
        // right-origin (desc=0x30): columns stored right-to-left
        let mut d = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 24, 0x30];
        d.extend_from_slice(&[0, 0, 1]); // stored right col first
        d.extend_from_slice(&[0, 0, 2]);
        let t = decode(&d).unwrap();
        assert_eq!(t.px[0], [2, 0, 0, 255]);
        assert_eq!(t.px[1], [1, 0, 0, 255]);
    }

    #[test]
    fn malformed_rejected() {
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[0u8; 18]), None); // w=0
                                              // colormapped → None
        let mut d = vec![0u8; 18];
        d[1] = 1;
        d[2] = 1;
        assert_eq!(decode(&d), None);
        // grayscale
        let mut d = vec![0u8; 18];
        d[2] = 3;
        d[12] = 1;
        d[14] = 1;
        assert_eq!(decode(&d), None);
        // truncated pixels
        let mut d = vec![0u8; 18];
        d[2] = 2;
        d[12] = 2;
        d[14] = 1;
        d[16] = 24;
        assert_eq!(decode(&d), None);
        // RLE overrun
        let mut d = vec![0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 24, 0x20];
        d.push(0x82);
        d.extend_from_slice(&[0, 0, 0]);
        assert_eq!(decode(&d), None);
    }

    #[test]
    fn determinism_twice() {
        let i = img();
        assert_eq!(encode(&i), encode(&i));
        assert_eq!(decode(&encode(&i)), decode(&encode(&i)));
    }
}

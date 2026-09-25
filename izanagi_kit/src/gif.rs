//! GIF89a image decoder (CompuServe spec) — the first image the kit
//! can pull *off the wire*: header, logical screen descriptor, global
//! color table, image descriptors, extension skip, and the GIF-specific
//! variable-width LZW (clear/EOI codes, code width grows 3→12 as the
//! dictionary fills, LSB-first bit packing via [`crate::bits`]).
//!
//! Note this is GIF-LZW, not the fixed-12-bit variant in
//! [`crate::lzw`]: GIF codes start at `min_code_size + 1` bits and widen
//! as entries are added, with `clear` and `end-of-information` in-band.
//!
//! Output is palette indices — the same shape as [`crate::png`]
//! `Color::Palette` produces, so both pipelines share
//! `palette`-to-RGB lookups downstream.
//!
//! Scope: non-interlaced images only (interlace → `None`), one image
//! descriptor (later frames ignored), transparency/animation
//! extensions skipped.
//!
//! ```
//! use izanagi_kit::gif::decode;
//!
//! // Minimal 1×1 GIF89a, index 0 (black palette entry).
//! let g: &[u8] = &[
//!     b'G', b'I', b'F', b'8', b'9', b'a', 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00,
//!     0x00, 0x00, 0x00, 0xff, 0xff, 0xff, // GCT: black, white
//!     0x2c, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, // image desc
//!     0x02, 0x02, 0x44, 0x01, 0x00, // LZW min=2, sub-block {0x44,0x01}
//!     0x3b,
//! ];
//! let img = decode(g).unwrap();
//! assert_eq!((img.w, img.h, img.pixels.as_slice()), (1, 1, &[0u8][..]));
//! ```

use std::vec::Vec;

/// A decoded GIF frame: `pixels` are palette indices into `palette`
/// (RGB triples).
pub struct Gif {
    /// Image width.
    pub w: u16,
    /// Image height.
    pub h: u16,
    /// One index byte per pixel, row-major.
    pub pixels: Vec<u8>,
    /// Palette as `r,g,b` triples.
    pub palette: Vec<u8>,
}

/// GIF LZW decoder: variable-width codes, clear/EOI in-band.
fn lzw_decode(data: &[u8], min_code: u8, want: usize) -> Option<Vec<u8>> {
    if !(2..=8).contains(&min_code) {
        return None;
    }
    let clear = 1usize << min_code;
    let eoi = clear + 1;
    let mut dict: Vec<Vec<u8>> = Vec::new();
    let mut r = crate::bits::BitReader::new(data);
    let mut out = Vec::with_capacity(want);
    let mut width = min_code as u32 + 1;
    let mut next = eoi + 1;
    let mut prev: Option<Vec<u8>> = None;
    // Reset the table to literals + clear + eoi.
    fn reset(dict: &mut Vec<Vec<u8>>, min_code: u8) {
        dict.clear();
        for i in 0..(1usize << min_code) {
            dict.push(vec![i as u8]);
        }
        dict.push(Vec::new()); // clear
        dict.push(Vec::new()); // eoi
    }
    reset(&mut dict, min_code);
    while !r.is_at_end() {
        let code = r.read_bits(width).ok()? as usize;
        if code == clear {
            reset(&mut dict, min_code);
            width = min_code as u32 + 1;
            next = eoi + 1;
            prev = None;
            continue;
        }
        if code == eoi {
            break;
        }
        let entry = if code < dict.len() {
            dict[code].clone()
        } else if code == dict.len() && prev.is_some() {
            // KwKwK: the phrase is prev + its own first byte.
            let mut e = prev.clone()?;
            e.push(e[0]);
            e
        } else {
            return None;
        };
        out.extend_from_slice(&entry);
        if let Some(p) = &prev {
            let mut add = p.clone();
            add.push(entry[0]);
            dict.push(add);
            next += 1;
            // GIF widens when `next` would overflow the current width
            // (early-change quirk: grow at next == 1<<width).
            if next == (1usize << width) && width < 12 {
                width += 1;
            }
        }
        prev = Some(entry);
    }
    if out.len() != want {
        return None;
    }
    Some(out)
}

fn le16(b: &[u8]) -> u16 {
    (b[0] as u16) | ((b[1] as u16) << 8)
}

/// Decode a GIF89a/GIF87a byte stream. `None` on a bad header,
/// missing tables, unsupported interlace, or a truncated image block.
pub fn decode(data: &[u8]) -> Option<Gif> {
    if data.len() < 13 || !(data[..6] == *b"GIF89a" || data[..6] == *b"GIF87a") {
        return None;
    }
    let lsd = &data[6..13];
    let packed = lsd[4];
    let gct_flag = packed & 0x80 != 0;
    let gct_size = 1usize << ((packed & 0x07) + 1);
    let mut at = 13usize;
    let mut palette = Vec::new();
    if gct_flag {
        if at + 3 * gct_size > data.len() {
            return None;
        }
        palette = data[at..at + 3 * gct_size].to_vec();
        at += 3 * gct_size;
    }
    let mut image: Option<Gif> = None;
    while at < data.len() {
        match data[at] {
            0x2c => {
                // Image descriptor: 9 bytes after the separator.
                if at + 10 > data.len() {
                    return None;
                }
                let d = &data[at + 1..at + 10];
                let w = le16(&d[4..]);
                let h = le16(&d[6..]);
                let ipacked = d[8];
                let lct_flag = ipacked & 0x80 != 0;
                let interlace = ipacked & 0x40 != 0;
                let lct_size = 1usize << ((ipacked & 0x07) + 1);
                at += 10;
                let mut pal = palette.clone();
                if lct_flag {
                    if at + 3 * lct_size > data.len() {
                        return None;
                    }
                    pal = data[at..at + 3 * lct_size].to_vec();
                    at += 3 * lct_size;
                }
                if pal.is_empty() || interlace {
                    return None;
                }
                if at >= data.len() {
                    return None;
                }
                let min_code = data[at];
                at += 1;
                // Concatenate sub-blocks.
                let mut payload = Vec::new();
                loop {
                    if at >= data.len() {
                        return None;
                    }
                    let n = data[at] as usize;
                    at += 1;
                    if n == 0 {
                        break;
                    }
                    if at + n > data.len() {
                        return None;
                    }
                    payload.extend_from_slice(&data[at..at + n]);
                    at += n;
                }
                let pixels = lzw_decode(&payload, min_code, w as usize * h as usize)?;
                image = Some(Gif {
                    w,
                    h,
                    pixels,
                    palette: pal,
                });
            }
            0x21 => {
                // Extension: label byte, then sub-blocks.
                at += 1;
                if at >= data.len() {
                    return None;
                }
                at += 1;
                loop {
                    if at >= data.len() {
                        return None;
                    }
                    let n = data[at] as usize;
                    at += 1;
                    if n == 0 {
                        break;
                    }
                    if at + n > data.len() {
                        return None;
                    }
                    at += n;
                }
            }
            0x3b => break, // trailer
            _ => return None,
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2×1 GIF89a, pixels [0, 1] (black, white) — hand-verified LZW:
    // min_code 2 → codes emitted: clear(4), 0, 1, eoi(5), width 3.
    const G2X1: &[u8] = &[
        b'G', b'I', b'F', b'8', b'9', b'a', 0x02, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xff, 0xff, 0xff, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x02,
        0x02, 0x44, 0x0a, 0x00, 0x3b,
    ];

    #[test]
    fn tiny_gif_decodes() {
        let img = decode(G2X1).unwrap();
        assert_eq!((img.w, img.h), (2, 1));
        assert_eq!(img.pixels, [0u8, 1]);
        assert_eq!(img.palette.len(), 6);
    }

    #[test]
    fn real_gif_pixels() {
        // Python GIF-LZW encoder vectors, byte-exact.
        let px = lzw_decode(&[0x44, 0x02, 0x05], 2, 4).unwrap();
        assert_eq!(px, [0u8, 1, 1, 0]);
        // Dictionary growth + the KwKwK case: codes 6,7,8 are added
        // phrases (emit [1,1], [1,1,1], ...).
        let px = lzw_decode(&[0x8c, 0x8f, 0x05], 2, 10).unwrap();
        assert_eq!(px, [1u8; 10]);
        // Length mismatch is a decode failure, not a truncation.
        assert!(lzw_decode(&[0x44, 0x01], 2, 2).is_none());
    }

    #[test]
    fn malformed_degrades() {
        assert!(decode(b"").is_none());
        assert!(decode(b"GIF89a").is_none());
        assert!(decode(&G2X1[..20]).is_none());
        let mut bad = G2X1.to_vec();
        bad[10] |= 0x20; // not a real flag — still must parse or fail cleanly
        let _ = decode(&bad);
    }
}

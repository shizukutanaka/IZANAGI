//! WebP — the RIFF-based image container: `RIFF` size `WEBP`, then a
//! chunk stream. `VP8 ` (lossy) reads dimensions from the frame's
//! `0x9D012A` signature + 14-bit width/height; `VP8L` (lossless) reads
//! the `0x2F` signature byte + 14-bit fields; `VP8X` (extended) reads
//! 24-bit canvas fields minus one. `ICCP`/`EXIF`/`XMP`/`ANIM`/`ANMF`
//! chunks are walked as `(fourcc, offset, size)`.
//!
//! `png`/`bmp`/`tga` cover the pixel codecs; `webp` covers the RIFF
//! wrapper that hosts all three payloads.
//!
//! ```
//! use izanagi_kit::webp;
//! let mut f = b"RIFF".to_vec();
//! f.extend_from_slice(&[0, 0, 0, 0]); // size patched below
//! f.extend_from_slice(b"WEBP");
//! f.extend_from_slice(b"VP8X");
//! f.extend_from_slice(&[10, 0, 0, 0]); // chunk size 10
//! f.extend_from_slice(&[0, 0, 0, 0]);   // flags
//! f.extend_from_slice(&[9, 0, 0]);      // width-1 = 9 → 10
//! f.extend_from_slice(&[19, 0, 0]);     // height-1 = 19 → 20
//! let riff_len = (f.len() - 8) as u32;
//! f[4] = riff_len as u8; f[5] = (riff_len >> 8) as u8;
//! let w = webp::parse(&f).unwrap();
//! assert_eq!((w.width, w.height), (10, 20));
//! ```

/// A RIFF chunk inside the WebP container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    /// Four-byte code (`"VP8 "`, `"VP8L"`, `"VP8X"`, `"ANIM"`, …).
    pub fourcc: [u8; 4],
    /// File offset of the chunk payload.
    pub offset: usize,
    /// Payload size in bytes.
    pub size: usize,
}

impl Chunk {
    /// The chunk code as a display string.
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.fourcc).to_string()
    }
}

/// A parsed WebP image header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebP {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// `true` when a `VP8X` chunk claims animation/alpha/transparency.
    pub extended: bool,
    /// All chunks in file order.
    pub chunks: Vec<Chunk>,
}

fn r32(d: &[u8], at: usize) -> Option<u32> {
    let a = *d.get(at)? as u32;
    let b = *d.get(at + 1)? as u32;
    let c = *d.get(at + 2)? as u32;
    let e = *d.get(at + 3)? as u32;
    Some((e << 24) | (c << 16) | (b << 8) | a)
}

fn r16(d: &[u8], at: usize) -> Option<u32> {
    let a = *d.get(at)? as u32;
    let b = *d.get(at + 1)? as u32;
    Some((b << 8) | a)
}

/// Parse the RIFF/WEBP wrapper. `None` on bad magic, truncation, or
/// no dimension-bearing chunk (`VP8X`/`VP8L`/`VP8 `).
pub fn parse(d: &[u8]) -> Option<WebP> {
    if d.get(..4)? != b"RIFF" || d.get(8..12)? != b"WEBP" {
        return None;
    }
    let riff_size = r32(d, 4)? as usize;
    let end = (riff_size + 8).min(d.len());
    let mut at = 12usize;
    let mut chunks = Vec::new();
    let mut width = 0u32;
    let mut height = 0u32;
    let mut extended = false;
    while at + 8 <= end {
        let fourcc = [d[at], d[at + 1], d[at + 2], d[at + 3]];
        let size = r32(d, at + 4)? as usize;
        let payload = at + 8;
        if payload.checked_add(size)? > end {
            break; // truncated trailing chunk — degrade
        }
        match &fourcc {
            b"VP8X" => {
                // flags(1) reserved(3) canvas-width-1(3) canvas-height-1(3)
                width = (*d.get(payload + 4)? as u32)
                    | ((*d.get(payload + 5)? as u32) << 8)
                    | ((*d.get(payload + 6)? as u32) << 16);
                width += 1;
                height = (*d.get(payload + 7)? as u32)
                    | ((*d.get(payload + 8)? as u32) << 8)
                    | ((*d.get(payload + 9)? as u32) << 16);
                height += 1;
                extended = true;
            }
            b"VP8L" => {
                // signature 0x2F then 14-bit fields
                if *d.get(payload)? == 0x2F {
                    let v = r32(d, payload + 1)?;
                    width = (v & 0x3FFF) + 1;
                    height = ((v >> 14) & 0x3FFF) + 1;
                }
            }
            // lossy: frame tag(3) + 0x9D012A + w16(14b) + h16(14b)
            b"VP8 " if d.get(payload + 3..payload + 6)? == b"\x9D\x01\x2A" => {
                width = r16(d, payload + 6)? & 0x3FFF;
                height = r16(d, payload + 8)? & 0x3FFF;
            }
            b"VP8 " => {}
            _ => {}
        }
        chunks.push(Chunk {
            fourcc,
            offset: payload,
            size,
        });
        // chunks are word-aligned — odd sizes pad one byte
        at = payload + size + (size & 1);
    }
    if width == 0 || height == 0 {
        return None;
    }
    Some(WebP {
        width,
        height,
        extended,
        chunks,
    })
}

/// The bytes of chunk `n`, or `None` if out of range.
pub fn chunk<'a>(d: &'a [u8], w: &WebP, n: usize) -> Option<&'a [u8]> {
    let c = w.chunks.get(n)?;
    let end = c.offset.checked_add(c.size)?;
    d.get(c.offset..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp8x(w: u32, h: u32) -> Vec<u8> {
        let mut f = b"RIFF".to_vec();
        f.extend_from_slice(&[0; 4]);
        f.extend_from_slice(b"WEBP");
        f.extend_from_slice(b"VP8X");
        f.extend_from_slice(&[10, 0, 0, 0]);
        f.extend_from_slice(&[0x30, 0, 0, 0]); // flags: alpha+animation
        let w1 = w - 1;
        f.extend_from_slice(&[w1 as u8, (w1 >> 8) as u8, (w1 >> 16) as u8]);
        let h1 = h - 1;
        f.extend_from_slice(&[h1 as u8, (h1 >> 8) as u8, (h1 >> 16) as u8]);
        let riff = (f.len() - 8) as u32;
        f[4] = riff as u8;
        f[5] = (riff >> 8) as u8;
        f
    }

    #[test]
    fn vp8x_dims() {
        let w = parse(&vp8x(10, 20)).unwrap();
        assert_eq!((w.width, w.height), (10, 20));
        assert!(w.extended);
        assert_eq!(w.chunks.len(), 1);
        assert_eq!(w.chunks[0].name(), "VP8X");
    }

    #[test]
    fn vp8l_dims() {
        let mut f = b"RIFF".to_vec();
        f.extend_from_slice(&[0; 4]);
        f.extend_from_slice(b"WEBP");
        f.extend_from_slice(b"VP8L");
        f.extend_from_slice(&[5, 0, 0, 0]); // chunk size 5
        f.push(0x2F); // signature
                      // width 1 → field=0, height 1 → field=0 → u32 = 0
        f.extend_from_slice(&[0, 0, 0, 0]);
        let riff = (f.len() - 8) as u32;
        f[4] = riff as u8;
        f[5] = (riff >> 8) as u8;
        let w = parse(&f).unwrap();
        assert_eq!((w.width, w.height), (1, 1));
    }

    #[test]
    fn vp8_lossy_dims() {
        let mut f = b"RIFF".to_vec();
        f.extend_from_slice(&[0; 4]);
        f.extend_from_slice(b"WEBP");
        f.extend_from_slice(b"VP8 ");
        f.extend_from_slice(&[10, 0, 0, 0]); // chunk size 10
        f.extend_from_slice(&[0, 0, 0]); // frame tag
        f.extend_from_slice(&[0x9D, 0x01, 0x2A]); // start code
        f.extend_from_slice(&[0x20, 0x00]); // width 0x0020 = 32
        f.extend_from_slice(&[0x10, 0x00]); // height 0x0010 = 16
        let riff = (f.len() - 8) as u32;
        f[4] = riff as u8;
        f[5] = (riff >> 8) as u8;
        let w = parse(&f).unwrap();
        assert_eq!((w.width, w.height), (32, 16));
        assert!(!w.extended);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"RIFX").is_none()); // wrong magic
        assert!(parse(b"RIFF\x08\0\0\0WEBX").is_none()); // wrong form
        assert!(parse(&vp8x(1, 1)[..14]).is_none()); // truncated
                                                     // VP8X with a corrupt dim byte still parses structurally
        let mut bad = vp8x(1, 1);
        bad[26] = 0xFF;
        assert!(parse(&bad).is_some());
    }
}

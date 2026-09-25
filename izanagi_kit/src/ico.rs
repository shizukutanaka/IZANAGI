//! ICO/CUR — the Windows icon/cursor container: a 6-byte directory
//! (`reserved u16 = 0`, `type u16` — 1 ICO, 2 CUR, `count u16`) followed
//! by `count` 16-byte entries (`width`, `height`, `colors`, `reserved`,
//! `planes`/`x-hotspot`, `bpp`/`y-hotspot`, `size`, `offset`). The
//! payload at `offset` is either a PNG (`\x89PNG`) or a BMP-with-mask
//! (DIB `BITMAPINFOHEADER`) — `image(n)` hands back whichever bytes.
//!
//! `bmp`/`png` cover the payloads; `ico` covers the directory wrapper.
//!
//! ```
//! use izanagi_kit::ico;
//! let mut f = vec![0, 0, 1, 0, 1, 0]; // ICO, 1 image
//! f.extend_from_slice(&[16, 16, 0, 0, 1, 0, 32, 0, 4, 0, 0, 0, 22, 0, 0, 0]);
//! f.extend_from_slice(&[1, 2, 3, 4]); // the image bytes
//! let i = ico::parse(&f).unwrap();
//! assert_eq!(i.count, 1);
//! assert_eq!(i.images[0].width, 16);
//! assert_eq!(ico::image(&f, &i, 0).unwrap(), &[1, 2, 3, 4]);
//! ```

/// One icon directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Pixel width (0 = 256).
    pub width: u16,
    /// Pixel height (0 = 256).
    pub height: u16,
    /// Bits-per-pixel field (`u16`; for CUR files this is the Y hotspot).
    pub bpp: u16,
    /// Planes field (`u16`; for CUR files this is the X hotspot).
    pub planes: u16,
    /// Byte size of the embedded image.
    pub size: u32,
    /// File offset of the embedded image.
    pub offset: u32,
}

/// A parsed ICO/CUR directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ico {
    /// `1` for ICO (icon), `2` for CUR (cursor).
    pub ty: u16,
    /// Number of images.
    pub count: u16,
    /// Directory entries.
    pub images: Vec<Entry>,
}

fn r16(d: &[u8], at: usize) -> Option<u16> {
    let a = *d.get(at)? as u16;
    let b = *d.get(at + 1)? as u16;
    Some((b << 8) | a) // ICO is little-endian
}

fn r32(d: &[u8], at: usize) -> Option<u32> {
    let a = r16(d, at)? as u32;
    let b = r16(d, at + 2)? as u32;
    Some((b << 16) | a)
}

/// Parse an ICO/CUR directory. `None` on bad magic, truncation, or
/// zero count. Entries are walked even when their payloads lie past
/// the file end — `image()` reports `None` for those.
pub fn parse(d: &[u8]) -> Option<Ico> {
    let reserved = r16(d, 0)?;
    if reserved != 0 {
        return None;
    }
    let ty = r16(d, 2)?;
    if ty != 1 && ty != 2 {
        return None;
    }
    let count = r16(d, 4)?;
    if count == 0 {
        return None;
    }
    let mut images = Vec::new();
    for i in 0..count {
        let at = 6usize.checked_add((i as usize).checked_mul(16)?)?;
        let w = *d.get(at)?;
        let h = *d.get(at + 1)?;
        let planes = r16(d, at + 4)?;
        let bpp = r16(d, at + 6)?;
        let size = r32(d, at + 8)?;
        let offset = r32(d, at + 12)?;
        images.push(Entry {
            width: if w == 0 { 256 } else { w as u16 },
            height: if h == 0 { 256 } else { h as u16 },
            planes,
            bpp,
            size,
            offset,
        });
    }
    Some(Ico { ty, count, images })
}

/// The bytes of image `n`, or `None` if `n` is out of range or the
/// payload runs past the file end. Sniff `b"\x89PNG"` for PNG or
/// check `u32 LE` at `0` for a BMP DIB header.
pub fn image<'a>(d: &'a [u8], i: &Ico, n: usize) -> Option<&'a [u8]> {
    let e = i.images.get(n)?;
    let start = e.offset as usize;
    let end = start.checked_add(e.size as usize)?;
    d.get(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_icon() -> Vec<u8> {
        let mut f = vec![0, 0, 1, 0, 2, 0];
        // entry 1: 16x16, 4 bytes at offset 38
        f.extend_from_slice(&[16, 16, 0, 0, 1, 0, 32, 0]);
        f.extend_from_slice(&[4, 0, 0, 0, 38, 0, 0, 0]);
        // entry 2: 32x32, 8 bytes at offset 42
        f.extend_from_slice(&[32, 32, 0, 0, 1, 0, 32, 0]);
        f.extend_from_slice(&[8, 0, 0, 0, 42, 0, 0, 0]);
        // payload
        f.extend_from_slice(&[0x89, b'P', b'N', b'G']); // PNG magic
        f.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
        f
    }

    #[test]
    fn two_images() {
        let f = two_icon();
        let i = parse(&f).unwrap();
        assert_eq!(i.ty, 1);
        assert_eq!(i.count, 2);
        assert_eq!(i.images.len(), 2);
        assert_eq!(i.images[0].width, 16);
        assert_eq!(i.images[0].bpp, 32);
        assert_eq!(i.images[1].width, 32);
        assert_eq!(image(&f, &i, 0).unwrap(), &[0x89, b'P', b'N', b'G']);
        assert_eq!(
            image(&f, &i, 1).unwrap(),
            &[0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3, 4]
        );
        assert!(image(&f, &i, 2).is_none());
    }

    #[test]
    fn cursor_type() {
        let mut f = vec![0, 0, 2, 0, 1, 0];
        f.extend_from_slice(&[16, 16, 0, 0, 5, 0, 7, 0, 4, 0, 0, 0, 22, 0, 0, 0]);
        f.extend_from_slice(&[1, 2, 3, 4]);
        let i = parse(&f).unwrap();
        assert_eq!(i.ty, 2);
        assert_eq!(i.images[0].planes, 5); // x hotspot
        assert_eq!(i.images[0].bpp, 7); // y hotspot
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 0, 9, 0, 1, 0]).is_none()); // bad type
        assert!(parse(&[0, 0, 1, 0, 0, 0]).is_none()); // zero count
        let mut f = two_icon();
        f.truncate(20); // truncated directory
        assert!(parse(&f).is_none());
    }

    #[test]
    fn out_of_range_payloads() {
        let mut f = vec![0, 0, 1, 0, 1, 0];
        f.extend_from_slice(&[16, 16, 0, 0, 1, 0, 32, 0, 4, 0, 0, 0, 200, 0, 0, 0]);
        // payload claims offset 200, file ends at 22
        let i = parse(&f).unwrap();
        assert!(image(&f, &i, 0).is_none());
    }
}

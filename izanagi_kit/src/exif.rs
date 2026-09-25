//! EXIF metadata — the TIFF-structured tag table inside JPEG APP1
//! segments (and standalone TIFF files). [`parse_jpeg`] finds the
//! `Exif\0\0` payload and walks IFD0 plus the EXIF (0x8769) and GPS
//! (0x8825) sub-IFDs; [`parse_tiff`] starts directly at an
//! `II*\0`/`MM\0` header. Entries keep their raw bytes; helpers
//! [`Exif::get_str`], [`Exif::get_int`], and [`Exif::get_rational`]
//! decode the common field types honouring the file's byte order.
//!
//! ```
//! use izanagi_kit::exif::{parse_tiff, Entry};
//! // Minimal TIFF: header + IFD0 with one SHORT tag (0x0100 width = 320)
//! let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
//! d.extend_from_slice(&[1, 0]);                    // 1 entry
//! d.extend_from_slice(&[0x00, 0x01, 3, 0, 1, 0, 0, 0, 0x40, 0x01, 0, 0]);
//! d.extend_from_slice(&[0, 0, 0, 0]);              // no next IFD
//! let x = parse_tiff(&d).unwrap();
//! assert_eq!(x.get_int(0x0100).unwrap(), 320);
//! ```

use std::vec::Vec;

/// One IFD entry: tag id, TIFF type, element count, and the raw value
/// bytes (already resolved through the value/offset field).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// TIFF tag number (e.g. 0x010F = Make).
    pub tag: u16,
    /// TIFF type (1 BYTE, 2 ASCII, 3 SHORT, 4 LONG, 5 RATIONAL,
    /// 7 UNDEFINED, 9 SLONG, 10 SRATIONAL).
    pub ty: u16,
    /// Element count.
    pub count: u32,
    /// Raw value bytes in file byte order.
    pub bytes: Vec<u8>,
}

/// A parsed EXIF block: byte order plus every entry from IFD0 and the
/// EXIF/GPS sub-IFDs (merged — tag collisions across IFDs are legal,
/// later entries do not shadow earlier ones; use [`Exif::entries`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Exif {
    /// True when the file is little-endian (`II`).
    pub le: bool,
    /// All decoded entries, IFD0 first then sub-IFD contents.
    pub entries: Vec<Entry>,
}

fn tsz(ty: u16) -> usize {
    match ty {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => 0,
    }
}

fn u16x(le: bool, d: &[u8], i: usize) -> Option<u16> {
    let a = *d.get(i)? as u16;
    let b = *d.get(i + 1)? as u16;
    Some(if le { a | (b << 8) } else { (a << 8) | b })
}

fn u32x(le: bool, d: &[u8], i: usize) -> Option<u32> {
    let hi = u16x(le, d, i)? as u32;
    let lo = u16x(le, d, i + 2)? as u32;
    Some(if le { (lo << 16) | hi } else { (hi << 16) | lo })
}

fn ifd(le: bool, d: &[u8], at: usize, out: &mut Vec<Entry>, sub: &mut Vec<u32>) -> Option<()> {
    let n = u16x(le, d, at)? as usize;
    for e in 0..n {
        let o = at.checked_add(2)?.checked_add(e.checked_mul(12)?)?;
        let tag = u16x(le, d, o)?;
        let ty = u16x(le, d, o + 2)?;
        let count = u32x(le, d, o + 4)?;
        let size = tsz(ty).checked_mul(count as usize)?;
        let bytes = if size <= 4 {
            d.get(o + 8..o + 8 + size)?.to_vec()
        } else {
            let off = u32x(le, d, o + 8)? as usize;
            d.get(off..off.checked_add(size)?)?.to_vec()
        };
        if tag == 0x8769 || tag == 0x8825 {
            sub.push(u32x(le, d, o + 8)?);
        }
        out.push(Entry {
            tag,
            ty,
            count,
            bytes,
        });
    }
    Some(())
}

/// Parse a TIFF/EXIF block starting at the `II*\0` or `MM\0` magic.
/// `None` on a bad magic, bad byte order mark, or truncated IFD.
pub fn parse_tiff(d: &[u8]) -> Option<Exif> {
    let le = match d.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    if u16x(le, d, 2)? != 42 {
        return None;
    }
    let ifd0 = u32x(le, d, 4)? as usize;
    let mut entries = Vec::new();
    let mut subs = Vec::new();
    ifd(le, d, ifd0, &mut entries, &mut subs)?;
    for s in subs {
        ifd(le, d, s as usize, &mut entries, &mut Vec::new())?;
    }
    Some(Exif { le, entries })
}

/// Find the EXIF APP1 segment in a JPEG stream (`FF D8` then
/// marker-prefixed segments) and parse it; `None` when no
/// `Exif\0\0` segment exists or the TIFF block is malformed.
pub fn parse_jpeg(d: &[u8]) -> Option<Exif> {
    if d.get(..2)? != b"\xFF\xD8" {
        return None;
    }
    let mut i = 2;
    while i + 4 <= d.len() {
        if d[i] != 0xFF {
            return None;
        }
        let marker = d[i + 1];
        if marker == 0xD9 || marker == 0xDA {
            return None; // EOI / start of scan data — no EXIF
        }
        let seg_len = ((d[i + 2] as usize) << 8) | d[i + 3] as usize;
        let seg = d.get(i + 4..i + 2 + seg_len)?;
        if marker == 0xE1 && seg.get(..6)? == b"Exif\x00\x00" {
            return parse_tiff(seg.get(6..)?);
        }
        i += 2 + seg_len;
    }
    None
}

impl Exif {
    /// All entries carrying `tag`, in IFD order.
    pub fn entries(&self, tag: u16) -> Vec<&Entry> {
        self.entries.iter().filter(|e| e.tag == tag).collect()
    }

    /// First entry with `tag`, or `None`.
    pub fn get(&self, tag: u16) -> Option<&Entry> {
        self.entries.iter().find(|e| e.tag == tag)
    }

    /// ASCII (type 2) value with trailing NUL stripped.
    pub fn get_str(&self, tag: u16) -> Option<String> {
        let e = self.get(tag)?;
        if e.ty != 2 {
            return None;
        }
        let end = e
            .bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(e.bytes.len());
        String::from_utf8(e.bytes[..end].to_vec()).ok()
    }

    /// First integer value of a BYTE/SHORT/LONG/SLONG entry.
    pub fn get_int(&self, tag: u16) -> Option<i64> {
        let e = self.get(tag)?;
        let b = &e.bytes;
        match e.ty {
            1 | 7 => Some(*b.first()? as i64),
            3 => Some(u16x(self.le, b, 0)? as i64),
            4 => Some(u32x(self.le, b, 0)? as i64),
            9 => {
                let v = u32x(self.le, b, 0)? as i32;
                Some(v as i64)
            }
            _ => None,
        }
    }

    /// First RATIONAL/SRATIONAL pair as `(num, den)` i64; `den` may be
    /// zero (EXIF encodes "unknown" that way).
    pub fn get_rational(&self, tag: u16) -> Option<(i64, i64)> {
        let e = self.get(tag)?;
        match e.ty {
            5 => Some((
                u32x(self.le, &e.bytes, 0)? as i64,
                u32x(self.le, &e.bytes, 4)? as i64,
            )),
            10 => Some((
                u32x(self.le, &e.bytes, 0)? as i32 as i64,
                u32x(self.le, &e.bytes, 4)? as i32 as i64,
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiff_with(entries: &[&[u8]], tail: &[u8]) -> Vec<u8> {
        // header
        let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
        d.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        for e in entries {
            d.extend_from_slice(e);
        }
        d.extend_from_slice(&[0, 0, 0, 0]); // no next IFD
        d.extend_from_slice(tail);
        d
    }

    #[test]
    fn short_and_long() {
        // tag 0x0100 SHORT ×1 inline = 320; tag 0x010F ASCII ×5 at offset 26 = "Cam\0!"
        let e1 = [0x00, 0x01, 3, 0, 1, 0, 0, 0, 0x40, 0x01, 0, 0];
        let e2 = [0x0F, 0x01, 2, 0, 5, 0, 0, 0, 0x26, 0, 0, 0];
        let d = tiff_with(&[&e1, &e2], b"Cam\x00!");
        let x = parse_tiff(&d).unwrap();
        assert_eq!(x.get_int(0x0100).unwrap(), 320);
        assert_eq!(x.get_str(0x010F).unwrap(), "Cam");
    }

    #[test]
    fn big_endian() {
        let mut d = b"MM\x00*\x00\x00\x00\x08".to_vec();
        d.extend_from_slice(&[0, 1]);
        // LONG ×1 inline, BE: tag 0x0102, 00 00 01 00 = 256
        d.extend_from_slice(&[0x01, 0x02, 0, 4, 0, 0, 0, 1, 0, 0, 1, 0]);
        d.extend_from_slice(&[0; 4]);
        let x = parse_tiff(&d).unwrap();
        assert!(!x.le);
        assert_eq!(x.get_int(0x0102).unwrap(), 256);
    }

    #[test]
    fn rational() {
        // tag 5 RATIONAL at offset 26: 300/10
        let e = [0x05, 0x00, 5, 0, 1, 0, 0, 0, 0x1A, 0, 0, 0];
        let mut tail = Vec::new();
        tail.extend_from_slice(&300u32.to_le_bytes());
        tail.extend_from_slice(&10u32.to_le_bytes());
        let d = tiff_with(&[&e], &tail);
        assert_eq!(parse_tiff(&d).unwrap().get_rational(5).unwrap(), (300, 10));
    }

    #[test]
    fn jpeg_wrapping() {
        // ASCII ×5 > 4 bytes → value lives at offset 0x1A, not inline
        let inner = tiff_with(
            &[&[0x10, 0x01, 2, 0, 5, 0, 0, 0, 0x1A, 0, 0, 0]],
            b"Xyz!\x00",
        );
        let mut j = vec![0xFF, 0xD8, 0xFF, 0xE1];
        let seg_len = inner.len() + 6 + 2;
        j.extend_from_slice(&[(seg_len >> 8) as u8, seg_len as u8]); // JPEG segment length is BE
        j.extend_from_slice(b"Exif\x00\x00");
        j.extend_from_slice(&inner);
        let x = parse_jpeg(&j).unwrap();
        assert_eq!(x.get_str(0x0110).unwrap(), "Xyz!");
    }

    #[test]
    fn subifd_merged() {
        // IFD0 with EXIF pointer tag 0x8769 → subIFD at offset 26 holding SHORT tag 0x9003
        let ptr = [0x69, 0x87, 4, 0, 1, 0, 0, 0, 0x1A, 0, 0, 0];
        let mut d = b"II*\x00\x08\x00\x00\x00".to_vec();
        d.extend_from_slice(&[1, 0]);
        d.extend_from_slice(&ptr);
        d.extend_from_slice(&[0; 4]);
        // sub-IFD at 26: one entry tag 0x9003 SHORT 7
        d.extend_from_slice(&[1, 0]);
        d.extend_from_slice(&[0x03, 0x90, 3, 0, 1, 0, 0, 0, 7, 0, 0, 0]);
        d.extend_from_slice(&[0; 4]);
        let x = parse_tiff(&d).unwrap();
        assert_eq!(x.get_int(0x9003).unwrap(), 7);
    }

    #[test]
    fn bad_inputs() {
        assert!(parse_tiff(b"").is_none());
        assert!(parse_tiff(b"ZZ*\x00\x08\x00\x00\x00").is_none());
        assert!(parse_tiff(b"II+\x00\x08\x00\x00\x00").is_none()); // magic ≠ 42
        assert!(parse_jpeg(b"\xFF\xD8\xFF\xD9").is_none());
        assert!(parse_jpeg(b"not jpeg").is_none());
    }

    #[test]
    fn determinism() {
        let e = [0x00, 0x01, 3, 0, 1, 0, 0, 0, 0x40, 0x01, 0, 0];
        let d = tiff_with(&[&e], b"");
        assert_eq!(parse_tiff(&d), parse_tiff(&d));
    }
}

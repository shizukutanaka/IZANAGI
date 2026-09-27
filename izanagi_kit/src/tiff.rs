//! TIFF / GeoTIFF image file directory (TIFF 6.0 + BigTIFF).
//!
//! Bytes 0–1 select byte order (`II` little / `MM` big), bytes 2–3
//! hold magic 42 (classic) or 43 (BigTIFF). Classic: u32 offset @4 to
//! IFD0 `{u16 count, count×12B entries, u32 next}` where an entry is
//! `{tag u16, type u16, count u32, value-or-offset u32}` — a value
//! fits inline when `count * type_size <= 4`, else the field is an
//! offset. BigTIFF: u16 `8`, u16 `0`, u64 offset @8, entries 20B with
//! u64 counts/values.
//!
//! ```
//! use izanagi_kit::tiff::{parse, entries, Type};
//! let mut d = b"II".to_vec();
//! d.extend_from_slice(&[42, 0, 8, 0, 0, 0]); // magic, IFD0 at 8
//! d.extend_from_slice(&[2, 0]);              // 2 entries
//! d.extend_from_slice(&[0x00, 0x01]);        // tag 0x0100 ImageWidth
//! d.extend_from_slice(&[3, 0]);              // type SHORT
//! d.extend_from_slice(&[1, 0, 0, 0]);        // count 1
//! d.extend_from_slice(&[200, 0, 0, 0]);      // inline value 200
//! d.extend_from_slice(&[0x01, 0x01, 4, 0]);  // tag 0x0101, LONG
//! d.extend_from_slice(&[1, 0, 0, 0, 57, 0, 0, 0]); // 57
//! d.extend_from_slice(&[0, 0, 0, 0]);        // no next IFD
//! let t = parse(&d).unwrap();
//! let es = entries(&d, &t).unwrap();
//! assert_eq!(es.len(), 2);
//! assert_eq!(es[0].value, 200);
//! assert_eq!(Type::from_id(es[0].kind), Some(Type::Short));
//! ```

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some((*d.get(o)? as u16) | (*d.get(o + 1)? as u16) << 8)
}
fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32)
            | (*d.get(o + 1)? as u32) << 8
            | (*d.get(o + 2)? as u32) << 16
            | (*d.get(o + 3)? as u32) << 24,
    )
}
fn le64(d: &[u8], o: usize) -> Option<u64> {
    Some(le32(d, o)? as u64 | (le32(d, o + 4)? as u64) << 32)
}
fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some((*d.get(o)? as u16) << 8 | *d.get(o + 1)? as u16)
}
fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32) << 24
            | (*d.get(o + 1)? as u32) << 16
            | (*d.get(o + 2)? as u32) << 8
            | *d.get(o + 3)? as u32,
    )
}
fn be64(d: &[u8], o: usize) -> Option<u64> {
    Some((be32(d, o)? as u64) << 32 | be32(d, o + 4)? as u64)
}

fn u16s(d: &[u8], o: usize, little: bool) -> Option<u16> {
    if little {
        le16(d, o)
    } else {
        be16(d, o)
    }
}
fn u32s(d: &[u8], o: usize, little: bool) -> Option<u32> {
    if little {
        le32(d, o)
    } else {
        be32(d, o)
    }
}
fn u64s(d: &[u8], o: usize, little: bool) -> Option<u64> {
    if little {
        le64(d, o)
    } else {
        be64(d, o)
    }
}

/// TIFF field types (TIFF 6.0 §2). Values 13+ (IFD/IFD64) are kept as
/// `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// `1` unsigned byte.
    Byte,
    /// `2` NUL-terminated ASCII.
    Ascii,
    /// `3` u16.
    Short,
    /// `4` u32.
    Long,
    /// `5` two u32s (numerator/denominator).
    Rational,
    /// `6` i8.
    SByte,
    /// `7` raw bytes.
    Undefined,
    /// `8` i16.
    SShort,
    /// `9` i32.
    SLong,
    /// `10` two i32s.
    SRational,
    /// `11` 4-byte float (stored raw).
    Float,
    /// `12` 8-byte float (stored raw).
    Double,
    /// `13` u32 pointing at another IFD (TIFF spec supplement).
    Ifd,
    /// `16`/`17`/`18` — BigTIFF u64/i64/u64-rational fields.
    Long8,
    /// `16`/`17`/`18` family member.
    SLong8,
    /// `16`/`17`/`18` family member.
    Ifd8,
    /// Any other id.
    Other(u16),
}

impl Type {
    /// Map a raw field-type id.
    pub fn from_id(id: u16) -> Option<Self> {
        Some(match id {
            1 => Self::Byte,
            2 => Self::Ascii,
            3 => Self::Short,
            4 => Self::Long,
            5 => Self::Rational,
            6 => Self::SByte,
            7 => Self::Undefined,
            8 => Self::SShort,
            9 => Self::SLong,
            10 => Self::SRational,
            11 => Self::Float,
            12 => Self::Double,
            13 => Self::Ifd,
            16 => Self::Long8,
            17 => Self::SLong8,
            18 => Self::Ifd8,
            other => Self::Other(other),
        })
    }
    /// Bytes per value element (`None` for unknown types).
    pub fn size(&self) -> Option<u64> {
        Some(match self {
            Self::Byte | Self::Ascii | Self::SByte | Self::Undefined => 1,
            Self::Short | Self::SShort => 2,
            Self::Long | Self::SLong | Self::Float | Self::Ifd => 4,
            Self::Rational
            | Self::SRational
            | Self::Double
            | Self::Long8
            | Self::SLong8
            | Self::Ifd8 => 8,
            Self::Other(_) => return None,
        })
    }
}

/// One IFD entry: `value` is the inline payload (low bytes of the
/// 4/8-byte field) or an offset when `count * type_size` exceeds the
/// field width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Tag id (`0x0100` ImageWidth, `0x0101` ImageLength, …).
    pub tag: u16,
    /// Field type id (see [`Type::from_id`]).
    pub kind: u16,
    /// Number of values.
    pub count: u64,
    /// Raw value/offset field (u32 zero-extended in classic files).
    pub value: u64,
    /// Offset of this entry inside the file.
    pub at: usize,
}

/// Parsed TIFF header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tiff {
    /// `II` little-endian when true, `MM` big-endian when false.
    pub little: bool,
    /// `true` for BigTIFF (magic 43, 20-byte entries).
    pub bigtiff: bool,
    /// Offset of the first image file directory.
    pub ifd0: usize,
}

/// Parse the byte-order mark, magic, and first-IFD offset. `None`
/// on a bad order mark or magic.
pub fn parse(d: &[u8]) -> Option<Tiff> {
    let little = match d.get(0..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    match u16s(d, 2, little)? {
        42 => Some(Tiff {
            little,
            bigtiff: false,
            ifd0: u32s(d, 4, little)? as usize,
        }),
        43 => {
            if u16s(d, 4, little)? != 8 || u16s(d, 6, little)? != 0 {
                return None;
            }
            let ifd0 = usize::try_from(u64s(d, 8, little)?).ok()?;
            Some(Tiff {
                little,
                bigtiff: true,
                ifd0,
            })
        }
        _ => None,
    }
}

/// Entries of the IFD at `at` (typically `t.ifd0`). `None` on any
/// truncation. The returned vec is empty on an out-of-bounds offset.
pub fn entries(d: &[u8], t: &Tiff) -> Option<Vec<Entry>> {
    let (count, mut at, width): (u64, usize, usize) = if t.bigtiff {
        (u64s(d, t.ifd0, t.little)?, t.ifd0 + 8, 20)
    } else {
        (u16s(d, t.ifd0, t.little)? as u64, t.ifd0 + 2, 12)
    };
    let mut out = Vec::new();
    for _ in 0..count {
        let tag = u16s(d, at, t.little)?;
        let kind = u16s(d, at + 2, t.little)?;
        let (count, value) = if t.bigtiff {
            (u64s(d, at + 4, t.little)?, u64s(d, at + 12, t.little)?)
        } else {
            (
                u32s(d, at + 4, t.little)? as u64,
                u32s(d, at + 8, t.little)? as u64,
            )
        };
        out.push(Entry {
            tag,
            kind,
            count,
            value,
            at,
        });
        at = at.checked_add(width)?;
    }
    Some(out)
}

/// Offset of the next IFD after the one at `t.ifd0` (`0`/`None` ends
/// the chain).
pub fn next_ifd(d: &[u8], t: &Tiff) -> Option<usize> {
    let (count, count_end, width): (u64, usize, usize) = if t.bigtiff {
        (u64s(d, t.ifd0, t.little)?, t.ifd0 + 8, 20)
    } else {
        (u16s(d, t.ifd0, t.little)? as u64, t.ifd0 + 2, 12)
    };
    let at = count_end.checked_add(count.checked_mul(width as u64)? as usize)?;
    let v = if t.bigtiff {
        u64s(d, at, t.little)?
    } else {
        u32s(d, at, t.little)? as u64
    };
    if v == 0 {
        None
    } else {
        usize::try_from(v).ok()
    }
}

/// Total bytes an entry's payload occupies (`None` for unknown types
/// or overflow).
pub fn byte_len(e: &Entry) -> Option<u64> {
    e.count.checked_mul(Type::from_id(e.kind)?.size()?)
}

/// Where the entry's payload bytes live: inline inside the entry's
/// value field when they fit, else at the recorded offset. Returns
/// the offset; bounds are the caller's to check against `d.len()`.
pub fn data_at(t: &Tiff, e: &Entry) -> Option<usize> {
    let len = byte_len(e)?;
    let inline = if t.bigtiff { 8 } else { 4 };
    if len <= inline {
        Some(e.at + 4 + if t.bigtiff { 8 } else { 4 })
    } else {
        usize::try_from(e.value).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(little: bool) -> Vec<u8> {
        let mut d = if little {
            b"II".to_vec()
        } else {
            b"MM".to_vec()
        };
        let w16 = |d: &mut Vec<u8>, v: u16| {
            if little {
                d.extend_from_slice(&v.to_le_bytes());
            } else {
                d.extend_from_slice(&[(v >> 8) as u8, (v & 0xFF) as u8]);
            }
        };
        let w32 = |d: &mut Vec<u8>, v: u32| {
            if little {
                d.extend_from_slice(&v.to_le_bytes());
            } else {
                d.extend_from_slice(&[
                    (v >> 24) as u8,
                    ((v >> 16) & 0xFF) as u8,
                    ((v >> 8) & 0xFF) as u8,
                    (v & 0xFF) as u8,
                ]);
            }
        };
        w16(&mut d, 42);
        w32(&mut d, 8); // IFD0 @ 8
        w16(&mut d, 2); // two entries
        w16(&mut d, 0x0100); // ImageWidth
        w16(&mut d, 3); // SHORT
        w32(&mut d, 1);
        w16(&mut d, 200);
        w16(&mut d, 0); // inline u16 in u32 field
        w16(&mut d, 0x0101); // ImageLength
        w16(&mut d, 4); // LONG
        w32(&mut d, 1);
        w32(&mut d, 57);
        w32(&mut d, 0); // next = end
        d
    }

    #[test]
    fn classic_both_endians() {
        for little in [true, false] {
            let d = fixture(little);
            let t = parse(&d).unwrap();
            assert_eq!(t.little, little);
            assert!(!t.bigtiff);
            assert_eq!(t.ifd0, 8);
            let es = entries(&d, &t).unwrap();
            assert_eq!(es.len(), 2);
            assert_eq!(es[0].tag, 0x0100);
            assert_eq!(es[0].kind, 3);
            assert_eq!(Type::from_id(es[0].kind), Some(Type::Short));
            assert_eq!(es[0].count, 1);
            // inline SHORT lives at the start of the field: low half
            // for II, high half for MM.
            if little {
                assert_eq!(es[0].value, 200);
            } else {
                assert_eq!(es[0].value, 200 << 16);
            }
            assert_eq!(es[1].value, 57);
            assert_eq!(next_ifd(&d, &t), None);
            // SHORT×1 fits inline → data at entry+8
            assert_eq!(data_at(&t, &es[0]), Some(es[0].at + 8));
            assert_eq!(byte_len(&es[0]), Some(2));
        }
    }

    #[test]
    fn bigtiff() {
        let mut d = b"II".to_vec();
        d.extend_from_slice(&[43, 0, 8, 0, 0, 0]);
        d.extend_from_slice(&16u64.to_le_bytes()); // ifd0 at 16
        d.extend_from_slice(&1u64.to_le_bytes()); // 1 entry
        d.extend_from_slice(&0x0100u16.to_le_bytes());
        d.extend_from_slice(&4u16.to_le_bytes());
        d.extend_from_slice(&1u64.to_le_bytes());
        d.extend_from_slice(&200u64.to_le_bytes());
        d.extend_from_slice(&0u64.to_le_bytes()); // next
        let t = parse(&d).unwrap();
        assert!(t.bigtiff);
        assert_eq!(t.ifd0, 16);
        let es = entries(&d, &t).unwrap();
        assert_eq!(es[0].value, 200);
        assert_eq!(data_at(&t, &es[0]), Some(es[0].at + 12));
        assert_eq!(next_ifd(&d, &t), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"MM").is_none());
        assert!(parse(b"ZZ*\0\0\0\0\0").is_none());
        let mut d = fixture(true);
        d[2] = 44; // bad magic
        assert!(parse(&d).is_none());
        // BigTIFF bad offsetsize
        let mut b = b"II".to_vec();
        b.extend_from_slice(&[43, 0, 4, 0, 0, 0]);
        assert!(parse(&b).is_none());
        // out-of-bounds ifd
        let mut d2 = fixture(true);
        d2[4] = 200;
        assert_eq!(entries(&d2, &parse(&d2).unwrap()), None);
    }
}

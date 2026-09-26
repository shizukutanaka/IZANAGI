//! IPS — the International Patching System ROM-patch format, the
//! thirty-year-old standard for distributing ROM hacks without
//! distributing the ROM. Grammar: `PATCH` magic, then records of
//! `offset u24 | size u16 | bytes` where `size == 0` instead means an
//! RLE record `count u16 | byte u8`, terminated by `EOF`. Lunar IPS's
//! de-facto extension follows `EOF` with a u24 size to truncate the
//! result to.
//!
//! [`parse`] is total and bounds-checks every record;
//! [`apply`]/`[`emit`] round-trip (`parse(emit p) == p`), and
//! [`diff`] generates a minimal patch between two byte strings.
//!
//! ```
//! use izanagi_kit::ips::{apply, parse};
//!
//! // PATCH + one 4-byte write at 0x10 + EOF.
//! let p = parse(b"PATCH\x00\x00\x10\x00\x04ABCDEOF").unwrap();
//! let rom = [0u8; 32];
//! let patched = apply(&rom, &p).unwrap();
//! assert_eq!(&patched[0x10..0x14], b"ABCD");
//! ```

use std::vec::Vec;

/// Every IPS file starts with this.
pub const MAGIC: &[u8] = b"PATCH";
/// Records stop at this marker.
pub const EOF_MARK: &[u8] = b"EOF";
/// Largest offset expressible (24-bit).
pub const MAX_OFFSET: usize = 0xFF_FFFF;

/// One patch record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rec {
    /// Write `bytes` at `offset` (the common case).
    Data {
        /// 24-bit target offset.
        offset: u32,
        /// Replacement bytes (1–65535).
        bytes: Vec<u8>,
    },
    /// Fill `count` bytes at `offset` with `byte` (`size == 0` form).
    Rle {
        /// 24-bit target offset.
        offset: u32,
        /// Run length (1–65535).
        count: u16,
        /// Fill value.
        byte: u8,
    },
}

/// A parsed IPS patch.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    /// Records in file order.
    pub records: Vec<Rec>,
    /// Lunar IPS truncation extension: bytes after `EOF` give a u24
    /// output length.
    pub truncate_to: Option<u32>,
}

fn u24be(d: &[u8], at: usize) -> Option<u32> {
    let b = d.get(at..at + 3)?;
    Some(u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]))
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    let b = d.get(at..at + 2)?;
    Some(u16::from(b[0]) << 8 | u16::from(b[1]))
}

/// Parse a patch. `None` on a bad magic, truncated record header,
/// truncated record body, or a missing `EOF` — a record whose
/// declared size overruns the file is an error, not a truncation.
pub fn parse(d: &[u8]) -> Option<Patch> {
    if d.len() < MAGIC.len() || &d[..5] != MAGIC {
        return None;
    }
    let mut records = Vec::new();
    let mut at = 5;
    loop {
        let header = d.get(at..at + 3)?;
        if header == EOF_MARK {
            at += 3;
            break;
        }
        let offset = u24be(d, at)?;
        let size = u16be(d, at + 3)? as usize;
        if size == 0 {
            let count = u16be(d, at + 5)?;
            let byte = *d.get(at + 7)?;
            records.push(Rec::Rle {
                offset,
                count,
                byte,
            });
            at += 8;
        } else {
            let bytes = d.get(at + 5..at + 5 + size)?.to_vec();
            records.push(Rec::Data { offset, bytes });
            at += 5 + size;
        }
    }
    let truncate_to = if at == d.len() {
        None
    } else {
        u24be(d, at) // Some(0) if the tail is short — still the mark.
    };
    Some(Patch {
        records,
        truncate_to,
    })
}

fn u16w(v: usize) -> [u8; 2] {
    let v = v as u16;
    [((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8]
}

/// Serialize back to the wire form; `parse(emit(&p)) == Some(p)`
/// whenever every record fits the 16-bit size/count fields (the
/// truncation extension is emitted when present).
pub fn emit(p: &Patch) -> Vec<u8> {
    let mut out = Vec::from(MAGIC);
    for r in &p.records {
        let offset = match r {
            Rec::Data { offset, .. } | Rec::Rle { offset, .. } => *offset,
        };
        out.push(((offset >> 16) & 0xFF) as u8);
        out.push(((offset >> 8) & 0xFF) as u8);
        out.push((offset & 0xFF) as u8);
        match r {
            Rec::Data { bytes, .. } => {
                let n = bytes.len().min(0xFFFF);
                out.extend_from_slice(&u16w(n));
                out.extend_from_slice(&bytes[..n]);
            }
            Rec::Rle { count, byte, .. } => {
                out.extend_from_slice(&u16w(0));
                out.extend_from_slice(&u16w(usize::from(*count)));
                out.push(*byte);
            }
        }
    }
    out.extend_from_slice(EOF_MARK);
    if let Some(t) = p.truncate_to {
        out.push(((t >> 16) & 0xFF) as u8);
        out.push(((t >> 8) & 0xFF) as u8);
        out.push((t & 0xFF) as u8);
    }
    out
}

/// Apply a patch: writes grow the ROM when they run past its end
/// (zero-filled), a `truncate_to` shrinks or pads the result.
pub fn apply(rom: &[u8], p: &Patch) -> Option<Vec<u8>> {
    let mut out = rom.to_vec();
    for r in &p.records {
        let (offset, len) = match r {
            Rec::Data { offset, bytes } => (*offset as usize, bytes.len()),
            Rec::Rle { offset, count, .. } => (*offset as usize, usize::from(*count)),
        };
        let end = offset.checked_add(len)?;
        if end > MAX_OFFSET + 1 {
            return None;
        }
        if out.len() < end {
            out.resize(end, 0);
        }
        match r {
            Rec::Data { bytes, .. } => out[offset..end].copy_from_slice(bytes),
            Rec::Rle { byte, .. } => out[offset..end].fill(*byte),
        }
    }
    if let Some(t) = p.truncate_to {
        out.resize(t as usize, 0);
    }
    Some(out)
}

/// Emit a `Data` record, chunking at the u16 length cap.
fn push_data(records: &mut Vec<Rec>, mut offset: u32, mut bytes: &[u8]) {
    while bytes.len() > 0xFFFF {
        records.push(Rec::Data {
            offset,
            bytes: bytes[..0xFFFF].to_vec(),
        });
        offset += 0xFFFF;
        bytes = &bytes[0xFFFF..];
    }
    if !bytes.is_empty() {
        records.push(Rec::Data {
            offset,
            bytes: bytes.to_vec(),
        });
    }
}

/// Emit `Rle` records, chunking at the u16 count cap.
fn push_rle(records: &mut Vec<Rec>, mut offset: u32, mut count: usize, byte: u8) {
    while count > 0 {
        let n = count.min(0xFFFF);
        records.push(Rec::Rle {
            offset,
            count: n as u16,
            byte,
        });
        offset += n as u32;
        count -= n;
    }
}

/// Diff two byte strings into a patch —
/// `apply(a, &diff(a, b)?)? == b`. `None` when `b` writes past the
/// 24-bit offset space. Same-byte runs of ≥4 emit as RLE records,
/// other changes as Data records, and a shorter `b` emits the
/// truncation extension.
pub fn diff(a: &[u8], b: &[u8]) -> Option<Patch> {
    if b.len() > MAX_OFFSET + 1 {
        return None;
    }
    let mut records = Vec::new();
    let n = a.len().max(b.len());
    let mut i = 0;
    while i < n {
        if a.get(i).copied().unwrap_or(0) == b.get(i).copied().unwrap_or(0) {
            i += 1;
            continue;
        }
        if i >= b.len() {
            break; // pure truncation tail — the extension covers it
        }
        // A differing span: extend while bytes differ.
        let start = i;
        while i < b.len() && a.get(i).copied().unwrap_or(0) != b[i] {
            i += 1;
        }
        let span = &b[start..i];
        // Split the span at same-byte runs ≥4 (RLE); the gaps between
        // them emit as Data records.
        let mut data_from = 0;
        let mut s = 0;
        while s < span.len() {
            let byte = span[s];
            let mut e = s + 1;
            while e < span.len() && span[e] == byte {
                e += 1;
            }
            if e - s >= 4 {
                push_data(
                    &mut records,
                    (start + data_from) as u32,
                    &span[data_from..s],
                );
                push_rle(&mut records, (start + s) as u32, e - s, byte);
                data_from = e;
            }
            s = e;
        }
        push_data(&mut records, (start + data_from) as u32, &span[data_from..]);
    }
    let truncate_to = if b.len() < a.len() {
        Some(b.len() as u32)
    } else {
        None
    };
    Some(Patch {
        records,
        truncate_to,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_and_rle_records() {
        let mut w = b"PATCH".to_vec();
        // Data record: offset 0x010000, 3 bytes.
        w.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x03]);
        w.extend_from_slice(b"xyz");
        // RLE record: offset 4, size 0, count 8, byte 0xAB.
        w.extend_from_slice(&[0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0xAB]);
        w.extend_from_slice(b"EOF");
        let p = parse(&w).unwrap();
        assert_eq!(p.records.len(), 2);
        assert_eq!(p.truncate_to, None);
        assert_eq!(emit(&p), w);
    }

    #[test]
    fn truncating_extension() {
        let mut w = b"PATCHEOF".to_vec();
        w.extend_from_slice(&[0x00, 0x00, 0x10]);
        let p = parse(&w).unwrap();
        assert_eq!(p.truncate_to, Some(0x10));
        let patched = apply(&[7u8; 100], &p).unwrap();
        assert_eq!(patched.len(), 16);
        assert!(patched.iter().all(|&b| b == 7));
    }

    #[test]
    fn apply_grows_and_fills() {
        let p = Patch {
            records: vec![
                Rec::Data {
                    offset: 2,
                    bytes: vec![9, 9],
                },
                Rec::Rle {
                    offset: 8,
                    count: 4,
                    byte: 0x55,
                },
            ],
            truncate_to: None,
        };
        let out = apply(&[1, 1, 1], &p).unwrap();
        assert_eq!(out, vec![1, 1, 9, 9, 0, 0, 0, 0, 0x55, 0x55, 0x55, 0x55]);
    }

    #[test]
    fn diff_round_trip() {
        let a = vec![0xA5u8; 64];
        let mut b = a.clone();
        b[3] = 1;
        b[4] = 2;
        for v in b.iter_mut().take(20).skip(10) {
            *v = 0xCC;
        }
        b[30] = 5;
        let p = diff(&a, &b).unwrap();
        assert!(p.records.iter().any(|r| matches!(r, Rec::Rle { .. })));
        assert_eq!(apply(&a, &p).unwrap(), b);
        // emit/parse round-trip preserves the patch.
        assert_eq!(parse(&emit(&p)).unwrap(), p);
        // truncation: b shorter than a.
        let p2 = diff(&a, &a[..8]).unwrap();
        assert_eq!(apply(&a, &p2).unwrap(), a[..8]);
    }

    #[test]
    fn malformed() {
        assert!(parse(b"").is_none());
        assert!(parse(b"PATC").is_none());
        assert!(parse(b"PATCH").is_none()); // no EOF
        assert!(parse(b"PATCH\x00\x00").is_none()); // truncated header
                                                    // declared record overruns file.
        assert!(parse(b"PATCH\x00\x00\x00\x00\x10xEOF").is_none());
        // RLE record truncated.
        assert!(parse(b"PATCH\x00\x00\x00\x00\x00\x00").is_none());
    }
}

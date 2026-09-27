//! EA IFF 85 ("Interchange File Format") container walk.
//!
//! An IFF file is a `FORM` (or `LIST`/`CAT `) whose 4-byte size is
//! big-endian and whose payload starts with a 4-byte form type
//! (`ILBM`, `8SVX`, `AIFF`, …), followed by chunks of
//! `{id:4, size:BE32, data, pad-to-even}`.
//!
//! ```
//! use izanagi_kit::iff::{parse, chunks, FORM, Chunk};
//! let mut d = b"FORM".to_vec();
//! d.extend_from_slice(&[0, 0, 0, 4 + 8 + 2]); // size
//! d.extend_from_slice(b"ILBM");
//! d.extend_from_slice(b"BODY");
//! d.extend_from_slice(&[0, 0, 0, 2]);
//! d.extend_from_slice(&[0xAA, 0xBB]);
//! let f = parse(&d).unwrap();
//! assert_eq!(f.kind, FORM);
//! assert_eq!(f.form_type, *b"ILBM");
//! let cs = chunks(&d, &f);
//! assert_eq!(cs.len(), 1);
//! assert_eq!(cs[0].id, *b"BODY");
//! ```

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32) << 24
            | (*d.get(o + 1)? as u32) << 16
            | (*d.get(o + 2)? as u32) << 8
            | *d.get(o + 3)? as u32,
    )
}

/// `FORM` — a file or nested typed group.
pub const FORM: [u8; 4] = *b"FORM";
/// `LIST` — a group of same-type forms sharing `LIST` properties.
pub const LIST: [u8; 4] = *b"LIST";
/// `CAT ` — a concatenation of possibly different forms.
pub const CAT: [u8; 4] = *b"CAT ";

/// One chunk header inside a form (`id`, declared `size`, and the
/// offset where its data starts — the data may be followed by one
/// pad byte to reach even length).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Four-character chunk identifier.
    pub id: [u8; 4],
    /// Declared data size in bytes (excludes the pad byte).
    pub size: usize,
    /// Offset of the chunk data within the buffer.
    pub at: usize,
}

/// A parsed top-level IFF form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Iff {
    /// `FORM`, `LIST`, or `CAT `.
    pub kind: [u8; 4],
    /// The form type word (`ILBM`, `8SVX`, …).
    pub form_type: [u8; 4],
    /// Offset where the chunk list begins (always `12`).
    pub chunks_at: usize,
    /// Offset where the declared form data ends.
    pub end: usize,
}

/// Parse the top-level form header. `None` when the buffer is short,
/// the kind isn't `FORM`/`LIST`/`CAT `, or the declared size overruns
/// the buffer.
pub fn parse(d: &[u8]) -> Option<Iff> {
    let mut kind = [0u8; 4];
    kind.copy_from_slice(d.get(0..4)?);
    if kind != FORM && kind != LIST && kind != CAT {
        return None;
    }
    let size = be32(d, 4)? as usize;
    let mut form_type = [0u8; 4];
    form_type.copy_from_slice(d.get(8..12)?);
    let end = 8usize.checked_add(size)?;
    if end > d.len() {
        return None;
    }
    Some(Iff {
        kind,
        form_type,
        chunks_at: 12,
        end,
    })
}

/// Read the chunk header starting at `at`, bounded by `end`.
/// `None` when fewer than 8 bytes remain or the chunk overruns `end`;
/// `Some(chunk)` with `chunk.at` pointing at the data.
pub fn chunk_at(d: &[u8], at: usize, end: usize) -> Option<Chunk> {
    let mut id = [0u8; 4];
    id.copy_from_slice(d.get(at..at + 4)?);
    let size = be32(d, at + 4)? as usize;
    let data_at = at.checked_add(8)?;
    if data_at.checked_add(size)? > end {
        return None;
    }
    Some(Chunk {
        id,
        size,
        at: data_at,
    })
}

/// Collect every top-level chunk inside a parsed form, stopping at
/// the first malformed one (trailing pad bytes are skipped).
pub fn chunks(d: &[u8], f: &Iff) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut at = f.chunks_at;
    while at + 8 <= f.end {
        match chunk_at(d, at, f.end) {
            Some(c) => {
                out.push(c);
                at = c.at + c.size + (c.size & 1);
            }
            None => break,
        }
    }
    out
}

/// Nested form walk: if `c` is itself a `FORM`/`LIST`/`CAT ` chunk,
/// parse its payload (`type` at data start, then its own chunk list).
pub fn subform(d: &[u8], c: &Chunk) -> Option<Iff> {
    if c.id != FORM && c.id != LIST && c.id != CAT {
        return None;
    }
    let mut form_type = [0u8; 4];
    form_type.copy_from_slice(d.get(c.at..c.at + 4)?);
    Some(Iff {
        kind: c.id,
        form_type,
        chunks_at: c.at + 4,
        end: c.at + c.size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> Vec<u8> {
        let mut d = b"FORM".to_vec();
        let inner: usize = 4 + (8 + 3 + 1) + (8 + 4);
        d.extend_from_slice(&[
            (inner >> 24) as u8,
            (inner >> 16) as u8,
            (inner >> 8) as u8,
            inner as u8,
        ]);
        d.extend_from_slice(b"ILBM");
        d.extend_from_slice(b"BMHD");
        d.extend_from_slice(&[0, 0, 0, 3]);
        d.extend_from_slice(&[1, 2, 3]);
        d.push(0); // pad to even
        d.extend_from_slice(b"BODY");
        d.extend_from_slice(&[0, 0, 0, 4]);
        d.extend_from_slice(&[9, 9, 9, 9]);
        d
    }

    #[test]
    fn walk_and_pad() {
        let d = body();
        let f = parse(&d).unwrap();
        assert_eq!(f.kind, FORM);
        assert_eq!(f.form_type, *b"ILBM");
        let cs = chunks(&d, &f);
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].id, *b"BMHD");
        assert_eq!(cs[0].size, 3);
        assert_eq!(&d[cs[0].at..cs[0].at + 3], &[1, 2, 3]);
        assert_eq!(cs[1].id, *b"BODY");
        // BMHD: header@12, data@20..23, pad@23 → BODY header@24, data@32
        assert_eq!(cs[1].at, 32);
        assert_eq!(&d[cs[1].at..cs[1].at + 4], &[9, 9, 9, 9]);
    }

    #[test]
    fn nested_and_rejects() {
        // LIST wrapping a FORM
        let inner = body();
        let mut d = b"LIST".to_vec();
        let n = inner.len();
        d.extend_from_slice(&[
            ((4 + n) >> 24) as u8,
            ((4 + n) >> 16) as u8,
            ((4 + n) >> 8) as u8,
            (4 + n) as u8,
        ]);
        d.extend_from_slice(b"PROP");
        d.extend_from_slice(&inner);
        let f = parse(&d).unwrap();
        assert_eq!(f.kind, LIST);
        let cs = chunks(&d, &f);
        assert_eq!(cs.len(), 1);
        let sub = subform(&d, &cs[0]).unwrap();
        assert_eq!(sub.kind, FORM);
        assert_eq!(sub.form_type, *b"ILBM");
        assert_eq!(chunks(&d, &sub).len(), 2);
        assert!(subform(&d, &cs[0]).is_some());
        // non-form chunks are not subforms
        assert!(subform(&d, &chunks(&d, &sub)[0]).is_none());

        assert!(parse(&[]).is_none());
        assert!(parse(b"XXXX\0\0\0\0TEST").is_none());
        let mut short = body();
        short[7] = 0xFF; // size overrun
        assert!(parse(&short).is_none());
        // truncated chunk stops the walk
        let mut t = body();
        t.truncate(20);
        let tf = Iff {
            kind: FORM,
            form_type: *b"ILBM",
            chunks_at: 12,
            end: t.len(),
        };
        assert!(chunks(&t, &tf).is_empty());
    }
}

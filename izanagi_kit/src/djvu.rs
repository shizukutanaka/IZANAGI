//! DjVu document parsing (IFF85 container).
//!
//! A DjVu file opens with `AT&T` inside a `FORM` chunk:
//! `FORM <len u32BE> AT&T <subkind>` where subkind is `DJVU`
//! (single page), `DJVM` (multi-page), `DJVI` (shared component),
//! or `THUM` (thumbnails). Inner chunks are `4cc u32BE` pairs
//! padded to even sizes.
//!
//! ```
//! use izanagi_kit::djvu;
//! let d = b"FORM\x00\x00\x00\x0CAT&TDJVUINFO\x00\x00\x00\x00";
//! let j = djvu::parse(d).unwrap();
//! assert_eq!(j.kind, djvu::Kind::SinglePage);
//! ```

use std::vec::Vec;

/// Document kind from the `FORM` subkind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `DJVU` — single-page document.
    SinglePage,
    /// `DJVM` — multi-page document.
    MultiPage,
    /// `DJVI` — shared (include) component.
    SharedComponent,
    /// `THUM` — thumbnail bundle.
    Thumbnails,
    /// Unrecognised subkind (raw bytes).
    Other([u8; 4]),
}

/// One inner chunk header.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chunk {
    /// Four-character chunk id.
    pub id: [u8; 4],
    /// Declared content length (unpadded).
    pub len: u32,
    /// Byte offset of the chunk body.
    pub at: usize,
}

/// A parsed DjVu file.
#[derive(Clone, Debug, PartialEq)]
pub struct Djvu {
    /// Document kind.
    pub kind: Kind,
    /// Declared `FORM` length.
    pub form_len: u32,
    /// Inner chunk headers (top level only).
    pub chunks: Vec<Chunk>,
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

/// Parses a DjVu file: `FORM … AT&T` envelope plus a known
/// subkind, then the top-level chunk list.
pub fn parse(d: &[u8]) -> Option<Djvu> {
    if d.len() < 16 || d.get(..4) != Some(b"FORM") {
        return None;
    }
    let form_len = u32be(d, 4)?;
    if d.get(8..12) != Some(b"AT&T") {
        return None;
    }
    let mut kind_b = [0u8; 4];
    kind_b.copy_from_slice(d.get(12..16)?);
    let kind = match &kind_b {
        b"DJVU" => Kind::SinglePage,
        b"DJVM" => Kind::MultiPage,
        b"DJVI" => Kind::SharedComponent,
        b"THUM" => Kind::Thumbnails,
        _ => Kind::Other(kind_b),
    };
    if matches!(kind, Kind::Other(_)) {
        return None;
    }
    // form body spans [12, 8+4+form_len); subkind occupies first 4.
    let end = 12usize.checked_add(form_len as usize)?.min(d.len());
    let mut chunks = Vec::new();
    let mut at = 16usize;
    while at + 8 <= end {
        let mut id = [0u8; 4];
        id.copy_from_slice(d.get(at..at + 4)?);
        let len = u32be(d, at + 4)?;
        let body = at + 8;
        if body.checked_add(len as usize)? > d.len() {
            return None;
        }
        chunks.push(Chunk { id, len, at: body });
        at = body + ((len as usize + 1) & !1);
    }
    Some(Djvu {
        kind,
        form_len,
        chunks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single() {
        // FORM len covers subkind(4) + INFO chunk(8+10)
        let mut d = b"FORM".to_vec();
        d.extend_from_slice(&22u32.to_be_bytes());
        d.extend_from_slice(b"AT&T");
        d.extend_from_slice(b"DJVU");
        d.extend_from_slice(b"INFO");
        d.extend_from_slice(&10u32.to_be_bytes());
        d.extend_from_slice(&[0u8; 10]);
        let j = parse(&d).unwrap();
        assert_eq!(j.kind, Kind::SinglePage);
        assert_eq!(j.chunks.len(), 1);
        assert_eq!(&j.chunks[0].id, b"INFO");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"FORM\x00\x00\x00\x04AT&T").is_none());
        assert!(parse(b"FORM\x00\x00\x00\x0CAT&TXYZW").is_none());
        assert!(parse(b"RIFF\x00\x00\x00\x0CAT&TDJVU").is_none());
    }
}

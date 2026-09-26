//! DLS — Downloadable Sounds Level 1/2 (`RIFF <size> "DLS "`):
//! `LIST "INFO"` metadata, `LIST "wvpl"` wave pool, `LIST "lins"`
//! instruments, `colh`/`vers`/`ptbl` chunks.
//!
//! ```
//! use izanagi_kit::dls::{parse};
//!
//! let mut d = b"RIFF".to_vec();
//! d.extend_from_slice(&[0, 0, 0, 0]);
//! d.extend_from_slice(b"DLS vers");
//! d.extend_from_slice(&[8, 0, 0, 0]);
//! d.extend_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0]);
//! let n = d.len() - 8;
//! d[4..8].copy_from_slice(&u32::to_le_bytes(n as u32));
//! let dl = parse(&d).unwrap();
//! assert_eq!(dl.version(), Some((1, 2)));
//! assert_eq!(dl.chunks().count(), 1);
//! ```

/// Form type (note the trailing space).
pub const FORM: &[u8; 4] = b"DLS ";

/// A top-level chunk or LIST body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    /// Four-byte chunk id (`"vers"`, `"colh"`, `"ptbl"`, `"LIST"` …).
    pub fourcc: [u8; 4],
    /// Chunk payload.
    pub data: &'a [u8],
}

/// Parsed DLS file.
#[derive(Debug, Clone, Copy)]
pub struct Dls<'a> {
    d: &'a [u8],
}

fn le32(d: &[u8], i: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(i)?)
            | (u32::from(*d.get(i + 1)?) << 8)
            | (u32::from(*d.get(i + 2)?) << 16)
            | (u32::from(*d.get(i + 3)?) << 24),
    )
}

/// Parse; requires RIFF form type `DLS ` and a consistent size.
pub fn parse(d: &[u8]) -> Option<Dls<'_>> {
    if d.get(..4)? != b"RIFF" || d.get(8..12)? != FORM {
        return None;
    }
    if (le32(d, 4)? as usize).checked_add(8)? > d.len() {
        return None;
    }
    Some(Dls { d })
}

impl<'a> Dls<'a> {
    /// Top-level chunk iterator.
    pub fn chunks(&self) -> Chunks<'a> {
        Chunks {
            d: self.d,
            at: 12,
            end: self.d.len(),
        }
    }

    /// `vers` chunk: `(major, minor)` u32 pair.
    pub fn version(&self) -> Option<(u32, u32)> {
        let c = self.chunks().find(|c| &c.fourcc == b"vers")?;
        Some((le32(c.data, 0)?, le32(c.data, 4)?))
    }

    /// `colh` chunk: instrument count.
    pub fn instruments(&self) -> Option<u32> {
        let c = self.chunks().find(|c| &c.fourcc == b"colh")?;
        le32(c.data, 0)
    }

    /// Sub-chunks inside a `LIST` body (`wvpl`, `lins`, `INFO` …).
    /// Pass the `data` field of a `LIST` chunk: its first four bytes are
    /// the list type.
    pub fn list(&self, data: &'a [u8]) -> Option<(List<'a>, Chunks<'a>)> {
        let name = std::str::from_utf8(data.get(..4)?).ok()?;
        Some((
            List { name },
            Chunks {
                d: data,
                at: 4,
                end: data.len(),
            },
        ))
    }
}

/// List type label for a `LIST` chunk body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct List<'a> {
    /// List type (`"wvpl"`, `"lins"`, `"INFO"`, `"wave"` …).
    pub name: &'a str,
}

/// `fourcc + u32LE size` chunk iterator over a range.
pub struct Chunks<'a> {
    d: &'a [u8],
    at: usize,
    end: usize,
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Chunk<'a>;
    fn next(&mut self) -> Option<Chunk<'a>> {
        if self.at.checked_add(8)? > self.end {
            return None;
        }
        let id: [u8; 4] = self.d.get(self.at..self.at + 4)?.try_into().ok()?;
        let size = le32(self.d, self.at + 4)? as usize;
        let body = self.at + 8;
        let end = body.checked_add(size)?;
        if end > self.end || end > self.d.len() {
            return None;
        }
        self.at = end + (size & 1);
        Some(Chunk {
            fourcc: id,
            data: &self.d[body..end],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(inner: &[u8]) -> Vec<u8> {
        let mut d = b"RIFF\0\0\0\0DLS ".to_vec();
        d.extend_from_slice(inner);
        let n = d.len() - 8;
        d[4..8].copy_from_slice(&u32::to_le_bytes(n as u32));
        d
    }

    #[test]
    fn version_and_colh() {
        let mut inner = b"vers".to_vec();
        inner.extend_from_slice(&[8, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]);
        inner.extend_from_slice(b"colh");
        inner.extend_from_slice(&[4, 0, 0, 0, 3, 0, 0, 0]);
        let d = mk(&inner);
        let dl = parse(&d).unwrap();
        assert_eq!(dl.version(), Some((1, 1)));
        assert_eq!(dl.instruments(), Some(3));
    }

    #[test]
    fn list_subchunks() {
        let mut inner = b"LIST".to_vec();
        let body = b"wvplwave\x04\0\0\0ABCD";
        inner.extend_from_slice(&u32::to_le_bytes(body.len() as u32));
        inner.extend_from_slice(body);
        let d = mk(&inner);
        let dl = parse(&d).unwrap();
        let list = dl.chunks().next().unwrap();
        assert_eq!(&list.fourcc, b"LIST");
        let (lt, mut sub) = dl.list(list.data).unwrap();
        assert_eq!(lt.name, "wvpl");
        let c = sub.next().unwrap();
        assert_eq!(&c.fourcc, b"wave");
        assert_eq!(c.data, b"ABCD");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RIFF\x04\0\0\0sfbk").is_none());
    }
}

//! SoundFont 2 (`RIFF <size> "sfbk"`) — sample+instrument container:
//! three top-level LISTs (`INFO`, `sdta`, `pdta`), each holding
//! `fourcc + u32LE size` sub-chunks.
//!
//! ```
//! use izanagi_kit::sf2::{parse, Chunk};
//!
//! let mut d = b"RIFF".to_vec();
//! d.extend_from_slice(&[0, 0, 0, 0]);
//! d.extend_from_slice(b"sfbkLIST");
//! d.extend_from_slice(&[16, 0, 0, 0]);
//! d.extend_from_slice(b"INFOifil");
//! d.extend_from_slice(&[4, 0, 0, 0]);
//! d.extend_from_slice(&[2, 0, 1, 0]);
//! let n = d.len() - 8;
//! d[4..8].copy_from_slice(&u32::to_le_bytes(n as u32));
//! let sf = parse(&d).unwrap();
//! let info = sf.lists().find(|l| l.name == "INFO").unwrap();
//! let c: Vec<_> = sf.chunks(&info).collect();
//! assert_eq!(c[0].fourcc, *b"ifil");
//! assert_eq!(sf.info_u16_pair(&info, b"ifil"), Some((2, 1)));
//! ```

/// RIFF tag.
pub const MAGIC: &[u8; 4] = b"RIFF";
/// Form type.
pub const FORM: &[u8; 4] = b"sfbk";

/// A top-level `LIST` inside the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct List<'a> {
    /// List type (`"INFO"`, `"sdta"`, `"pdta"`).
    pub name: &'a str,
    /// Offset of the first sub-chunk.
    pub at: usize,
    /// End offset of the list body.
    pub end: usize,
}

/// A leaf chunk inside a LIST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    /// Four-byte chunk id.
    pub fourcc: [u8; 4],
    /// Chunk payload (already bounds-checked).
    pub data: &'a [u8],
}

/// Parsed file.
#[derive(Debug, Clone, Copy)]
pub struct Sf2<'a> {
    d: &'a [u8],
}

fn le16(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from(*d.get(i)?) | (u16::from(*d.get(i + 1)?) << 8))
}
fn le32(d: &[u8], i: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(i)?)
            | (u32::from(*d.get(i + 1)?) << 8)
            | (u32::from(*d.get(i + 2)?) << 16)
            | (u32::from(*d.get(i + 3)?) << 24),
    )
}

/// Parse; requires `RIFF`-tagged data whose form type is `sfbk`.
pub fn parse(d: &[u8]) -> Option<Sf2<'_>> {
    if d.get(..4)? != MAGIC || d.get(8..12)? != FORM {
        return None;
    }
    let declared = le32(d, 4)? as usize;
    if declared.checked_add(8)? > d.len() {
        return None;
    }
    Some(Sf2 { d })
}

/// Iterate `fourcc + size` records starting at `at`, ending before `end`.
fn chunk_walk<'a>(d: &'a [u8], at: usize, end: usize) -> Chunks<'a> {
    Chunks { d, at, end }
}

impl<'a> Sf2<'a> {
    /// Top-level LIST iterator (`INFO`, `sdta`, `pdta`).
    pub fn lists(&self) -> Lists<'a> {
        Lists { d: self.d, at: 12 }
    }

    /// Sub-chunks of one LIST.
    pub fn chunks(&self, l: &List) -> Chunks<'a> {
        chunk_walk(self.d, l.at, l.end)
    }

    /// `INFO` two-u16 field (`ifil` = version, `iver`/`irom` …).
    pub fn info_u16_pair(&self, l: &List, tag: &[u8; 4]) -> Option<(u16, u16)> {
        let c = self.chunks(l).find(|c| &c.fourcc == tag)?;
        Some((le16(c.data, 0)?, le16(c.data, 2)?))
    }

    /// `INFO` NUL-terminated string field (`INAM`, `ISFT`, `ICOP` …).
    pub fn info_str(&self, l: &List, tag: &[u8; 4]) -> Option<&'a str> {
        let c = self.chunks(l).find(|c| &c.fourcc == tag)?;
        let end = c.data.iter().position(|&b| b == 0).unwrap_or(c.data.len());
        std::str::from_utf8(&c.data[..end]).ok()
    }

    /// First `sdta` `smpl` payload (the 16-bit sample blob).
    pub fn samples(&self) -> Option<Chunk<'a>> {
        let sdta = self.lists().find(|l| l.name == "sdta")?;
        self.chunks(&sdta).find(|c| &c.fourcc == b"smpl")
    }
}

/// LIST iterator over the form body.
pub struct Lists<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> Iterator for Lists<'a> {
    type Item = List<'a>;
    fn next(&mut self) -> Option<List<'a>> {
        loop {
            let id = self.d.get(self.at..self.at + 4)?;
            let size = le32(self.d, self.at + 4)? as usize;
            let body = self.at.checked_add(8)?;
            let end = body.checked_add(size)?;
            if end > self.d.len() {
                return None;
            }
            self.at = end + (size & 1);
            if id != b"LIST" {
                continue;
            }
            let name = std::str::from_utf8(self.d.get(body..body + 4)?).unwrap_or("");
            return Some(List {
                name,
                at: body + 4,
                end,
            });
        }
    }
}

/// `fourcc + size` chunk iterator.
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
    use std::vec::Vec;

    fn sf2() -> Vec<u8> {
        let mut d = b"RIFF\0\0\0\0sfbk".to_vec();
        let mut info = b"INFO".to_vec();
        info.extend_from_slice(b"ifil");
        info.extend_from_slice(&[4, 0, 0, 0]);
        info.extend_from_slice(&[2, 0, 1, 0]);
        info.extend_from_slice(b"INAM");
        info.extend_from_slice(&[5, 0, 0, 0]);
        info.extend_from_slice(b"Piano\0x"); // odd-size pad
        let mut list = b"LIST".to_vec();
        list.extend_from_slice(&u32::to_le_bytes(info.len() as u32));
        list.extend_from_slice(&info);
        list.push(0); // pad
        d.extend_from_slice(&list);
        let n = d.len() - 8;
        d[4..8].copy_from_slice(&u32::to_le_bytes(n as u32));
        d
    }

    #[test]
    fn parse_and_fields() {
        let d = sf2();
        let sf = parse(&d).unwrap();
        let ls: Vec<_> = sf.lists().collect();
        assert_eq!(ls.len(), 1);
        assert_eq!(ls[0].name, "INFO");
        assert_eq!(sf.info_u16_pair(&ls[0], b"ifil"), Some((2, 1)));
        assert_eq!(sf.info_str(&ls[0], b"INAM"), Some("Piano"));
        assert_eq!(sf.info_str(&ls[0], b"NOPE"), None);
        assert!(sf.samples().is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RIFF\xFF\xFF\xFF\xFFsfbk").is_none());
        assert!(parse(b"RIFF\x04\0\0\0xxxx").is_none());
    }
}

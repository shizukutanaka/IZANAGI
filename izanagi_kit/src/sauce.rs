//! SAUCE — the Standard Architecture for Universal Comment Extensions,
//! a 128-byte metadata trailer appended to ANSI art and related files.
//!
//! Layout (little-endian): `SAUCE00` signature, 35-byte title,
//! 20-byte author, 20-byte group, `CCYYMMDD` date, file size u32,
//! data-type/file-type bytes, four u16 TInfo fields, a comment
//! line count, flags, and a 22-byte TInfoS string. A `COMNT` block
//! of `comments` 64-byte lines sits immediately before the record.
//!
//! ```
//! use izanagi_kit::sauce::{Sauce, RECORD};
//! let mut d = vec![0u8; RECORD];
//! d[..7].copy_from_slice(b"SAUCE00");
//! d[7..13].copy_from_slice(b"Title!");
//! assert_eq!(Sauce::parse(&d).unwrap().0.title(), "Title!");
//! ```

/// Trailer record size in bytes.
pub const RECORD: usize = 128;
/// Record signature `SAUCE00`.
pub const SIG: &[u8; 7] = b"SAUCE00";
/// Comment block signature preceding the record.
pub const COMNT: &[u8; 5] = b"COMNT";

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}
fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

fn trim(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).trim_end().to_string()
}

/// A parsed SAUCE record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sauce {
    /// 35-byte title field (space/NUL padded).
    pub title_raw: [u8; 35],
    /// 20-byte author field.
    pub author_raw: [u8; 20],
    /// 20-byte group field.
    pub group_raw: [u8; 20],
    /// `CCYYMMDD` date field.
    pub date_raw: [u8; 8],
    /// Declared file size (excludes the SAUCE metadata).
    pub file_size: u32,
    /// DataType byte (0 none, 1 character, 2 bitmap, 3 vector, …).
    pub data_type: u8,
    /// FileType byte (meaning depends on `data_type`).
    pub file_type: u8,
    /// TInfo1/TInfo2 (width/height for character art).
    pub tinfo1: u16,
    /// TInfo2.
    pub tinfo2: u16,
    /// TInfo3/TInfo4 (usually zero).
    pub tinfo3: u16,
    /// TInfo4.
    pub tinfo4: u16,
    /// Number of 64-byte comment lines in the COMNT block.
    pub comments: u8,
    /// Flags byte (iCE color bit, letter-spacing bits, aspect).
    pub flags: u8,
    /// 22-byte font-name field (`TInfoS`).
    pub tinfos_raw: [u8; 22],
}

impl Sauce {
    /// Title with padding trimmed.
    pub fn title(&self) -> String {
        trim(&self.title_raw)
    }
    /// Author trimmed.
    pub fn author(&self) -> String {
        trim(&self.author_raw)
    }
    /// Group trimmed.
    pub fn group(&self) -> String {
        trim(&self.group_raw)
    }
    /// `CCYYMMDD` date string.
    pub fn date(&self) -> String {
        trim(&self.date_raw)
    }
    /// Font name (`TInfoS`) trimmed.
    pub fn font(&self) -> String {
        trim(&self.tinfos_raw)
    }
    /// True when the iCE-color flag bit is set.
    pub fn ice_colors(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Read the record at the end of `d`. Returns the record and its
    /// byte offset (`d.len() - 128` normally).
    pub fn parse(d: &[u8]) -> Option<(Sauce, usize)> {
        if d.len() < RECORD {
            return None;
        }
        Self::parse_at(d, d.len() - RECORD)
    }

    /// Read the record at byte offset `at`.
    pub fn parse_at(d: &[u8], at: usize) -> Option<(Sauce, usize)> {
        let r = d.get(at..at + RECORD)?;
        if r.get(..7)? != SIG {
            return None;
        }
        let mut title_raw = [0u8; 35];
        title_raw.copy_from_slice(r.get(7..42)?);
        let mut author_raw = [0u8; 20];
        author_raw.copy_from_slice(r.get(42..62)?);
        let mut group_raw = [0u8; 20];
        group_raw.copy_from_slice(r.get(62..82)?);
        let mut date_raw = [0u8; 8];
        date_raw.copy_from_slice(r.get(82..90)?);
        let mut tinfos_raw = [0u8; 22];
        tinfos_raw.copy_from_slice(r.get(106..128)?);
        Some((
            Sauce {
                title_raw,
                author_raw,
                group_raw,
                date_raw,
                file_size: le32(r, 90)?,
                data_type: *r.get(94)?,
                file_type: *r.get(95)?,
                tinfo1: le16(r, 96)?,
                tinfo2: le16(r, 98)?,
                tinfo3: le16(r, 100)?,
                tinfo4: le16(r, 102)?,
                comments: *r.get(104)?,
                flags: *r.get(105)?,
                tinfos_raw,
            },
            at,
        ))
    }

    /// The `comments` 64-byte lines of the COMNT block that precedes
    /// a record parsed at `sauce_at`. `None` when the signature is
    /// absent or the block would underflow.
    pub fn comment_lines(&self, d: &[u8], sauce_at: usize) -> Option<Vec<Vec<u8>>> {
        if self.comments == 0 {
            return Some(Vec::new());
        }
        let n = usize::from(self.comments);
        let start = sauce_at.checked_sub(5 + n * 64)?;
        if d.get(start..start + 5)? != COMNT {
            return None;
        }
        Some(
            (0..n)
                .map(|i| d[start + 5 + i * 64..start + 5 + (i + 1) * 64].to_vec())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 300 + RECORD];
        let r = &mut d[300..];
        r[..7].copy_from_slice(SIG);
        r[7..14].copy_from_slice(b"STATION");
        r[42..45].copy_from_slice(b"ume");
        r[62..67].copy_from_slice(b"ACiD.");
        r[82..90].copy_from_slice(b"20260926");
        r[90] = 44;
        r[91] = 1; // file_size 300
        r[94] = 1; // data_type character
        r[95] = 1; // file_type ansi
        r[96] = 80;
        r[98] = 25;
        r[105] = 3; // ice + ls bits
        r[106..110].copy_from_slice(b"IBM ");
        d
    }

    #[test]
    fn fields_and_trim() {
        let (s, at) = Sauce::parse(&fixture()).unwrap();
        assert_eq!(at, 300);
        assert_eq!(s.title(), "STATION");
        assert_eq!(s.author(), "ume");
        assert_eq!(s.group(), "ACiD.");
        assert_eq!(s.date(), "20260926");
        assert_eq!(s.file_size, 300);
        assert_eq!(s.data_type, 1);
        assert_eq!(s.file_type, 1);
        assert_eq!(s.tinfo1, 80);
        assert_eq!(s.tinfo2, 25);
        assert_eq!(s.tinfo3, 0);
        assert_eq!(s.tinfo4, 0);
        assert_eq!(s.comments, 0);
        assert!(s.ice_colors());
        assert_eq!(s.font(), "IBM");
    }

    #[test]
    fn comment_block() {
        let mut d = fixture();
        // record `comments=2`, then insert COMNT + 2 lines before it
        d[300 + 104] = 2;
        let mut tail = d.split_off(300);
        let mut block = COMNT.to_vec();
        block.extend_from_slice(&[b'A'; 64]);
        block.extend_from_slice(&[b'B'; 64]);
        d.extend_from_slice(&block);
        d.append(&mut tail);
        let (s, at) = Sauce::parse(&d).unwrap();
        let lines = s.comment_lines(&d, at).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], vec![b'A'; 64]);
        assert_eq!(lines[1], vec![b'B'; 64]);
        // no block present -> None
        assert!(Sauce::parse(&fixture()).unwrap().0.comments == 0);
    }

    #[test]
    fn rejects() {
        assert!(Sauce::parse(&[0u8; 64]).is_none());
        let mut d = fixture();
        d[300] = b'X';
        assert!(Sauce::parse(&d).is_none());
        let (s, at) = Sauce::parse(&fixture()).unwrap();
        // comments>0 without a COMNT block -> None
        let mut d2 = fixture();
        d2[300 + 104] = 1;
        let (s2, at2) = Sauce::parse(&d2).unwrap();
        assert!(s2.comment_lines(&d2, at2).is_none());
        let _ = (s, at);
    }
}

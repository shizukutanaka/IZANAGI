//! DICOM Part 10 file reader.
//!
//! A `.dcm` file carries a 128-byte preamble, the `DICM` magic,
//! then a chain of data elements. In the default Explicit VR
//! encoding each element is `(group, element)` u16 LE, a 2-byte VR
//! code and a length — u16 for most VRs, `0x0000` + u32 for
//! `OB`/`OW`/`OF`/`OD`/`OL`/`OV`/`SQ`/`UC`/`UN`/`UR`/`UT`.
//! `parse` walks the elements and exposes the well-known tags.
//!
//! ```
//! use izanagi_kit::dicom::{parse, MAGIC_AT};
//!
//! let mut d = vec![0u8; MAGIC_AT + 4 + 8 + 5];
//! d[128..132].copy_from_slice(b"DICM");
//! let t = 132; // (0008,0060) Modality, VR=CS, len 5? use 4
//! d[t] = 0x08; d[t + 1] = 0; d[t + 2] = 0x60; d[t + 3] = 0;
//! d[t + 4] = b'C'; d[t + 5] = b'T';
//! d[t + 6] = 4; d[t + 7] = 0;
//! d[t + 8..t + 12].copy_from_slice(b"CT  ");
//! let f = parse(&d).unwrap();
//! let e = f.get(0x0008, 0x0060).unwrap();
//! assert_eq!(e.text(&d), Some("CT  "));
//! ```

/// Offset of the `DICM` magic.
pub const MAGIC_AT: usize = 128;

/// VRs whose element uses a u32 length after two zero bytes.
pub const LONG_LEN: [&str; 10] = ["OB", "OW", "OF", "OD", "OL", "OV", "SQ", "UC", "UR", "UN"];
// note: UT uses u32 too; included in LONG check below via table.

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// One data element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    /// Tag group.
    pub group: u16,
    /// Tag element number.
    pub element: u16,
    /// Two-char value representation.
    pub vr: String,
    /// Value length in bytes.
    pub len: u32,
    /// File offset of the value.
    pub data_at: usize,
    /// Offset of the next element.
    pub next_at: usize,
}

impl Element {
    /// True for undefined-length sequence items (len 0xFFFFFFFF).
    pub fn indefinite(&self) -> bool {
        self.len == u32::MAX
    }
    /// Value bytes.
    pub fn value<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        if self.indefinite() {
            return None;
        }
        d.get(self.data_at..self.data_at.checked_add(usize::try_from(self.len).ok()?)?)
    }
    /// Value as text for string VRs (no trailing NULs trimmed —
    /// DICOM pads with spaces/NUL per the VR).
    pub fn text<'a>(&self, d: &'a [u8]) -> Option<&'a str> {
        core::str::from_utf8(self.value(d)?).ok()
    }
}

/// A parsed DICOM P10 file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dicom {
    /// The 128-byte preamble (kept raw; may hold e.g. icon data).
    pub preamble: [u8; 128],
    /// All elements in file order.
    pub elements: Vec<Element>,
}

impl Dicom {
    /// First element with the given tag.
    pub fn get(&self, group: u16, element: u16) -> Option<&Element> {
        self.elements
            .iter()
            .find(|e| e.group == group && e.element == element)
    }
}

/// Parse a file with explicit-VR little-endian transfer syntax.
/// Returns `None` when the magic is missing or an element is
/// truncated. Walking stops at the first unparseable boundary.
pub fn parse(d: &[u8]) -> Option<Dicom> {
    if d.get(MAGIC_AT..MAGIC_AT + 4)? != b"DICM" {
        return None;
    }
    let mut preamble = [0u8; 128];
    preamble.copy_from_slice(d.get(..128)?);
    let mut elements = Vec::new();
    let mut at = MAGIC_AT + 4;
    while let Some(group) = le16(d, at) {
        let element = le16(d, at + 2)?;
        let vr = core::str::from_utf8(d.get(at + 4..at + 6)?)
            .ok()?
            .to_string();
        let (len, data_at) = if LONG_LEN.contains(&vr.as_str()) || vr == "UT" {
            (le32(d, at + 8)?, at + 12)
        } else {
            (u32::from(le16(d, at + 6)?), at + 8)
        };
        let next_at = if len == u32::MAX {
            data_at
        } else {
            data_at.checked_add(usize::try_from(len).ok()?)?
        };
        if next_at > d.len() {
            return None;
        }
        elements.push(Element {
            group,
            element,
            vr,
            len,
            data_at,
            next_at,
        });
        at = next_at;
        if at == d.len() {
            break;
        }
    }
    Some(Dicom { preamble, elements })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn element(d: &mut Vec<u8>, g: u16, e: u16, vr: &str, val: &[u8]) {
        d.push((g & 0xff) as u8);
        d.push((g >> 8) as u8);
        d.push((e & 0xff) as u8);
        d.push((e >> 8) as u8);
        d.extend_from_slice(vr.as_bytes());
        if LONG_LEN.contains(&vr) || vr == "UT" {
            d.extend_from_slice(&[0, 0]);
            let l = val.len() as u32;
            d.extend_from_slice(&[l as u8, (l >> 8) as u8, (l >> 16) as u8, (l >> 24) as u8]);
        } else {
            let l = val.len() as u16;
            d.extend_from_slice(&[l as u8, (l >> 8) as u8]);
        }
        d.extend_from_slice(val);
    }

    #[test]
    fn explicit_vr_walk() {
        let mut d = vec![0u8; 132];
        d[128..132].copy_from_slice(b"DICM");
        element(&mut d, 0x0002, 0x0010, "UI", b"1.2.840.10008.1.2.1\0");
        element(&mut d, 0x0008, 0x0060, "CS", b"MR  ");
        element(&mut d, 0x0010, 0x0010, "PN", b"Doe^John ");
        element(&mut d, 0x7fe0, 0x0010, "OB", &[1, 2, 3, 4]);
        let f = parse(&d).unwrap();
        assert_eq!(f.elements.len(), 4);
        let ts = f.get(0x0002, 0x0010).unwrap();
        assert_eq!(ts.vr, "UI");
        assert!(ts.text(&d).unwrap().starts_with("1.2.840"));
        assert_eq!(f.get(0x0008, 0x0060).unwrap().text(&d), Some("MR  "));
        let px = f.get(0x7fe0, 0x0010).unwrap();
        assert_eq!(px.vr, "OB");
        assert_eq!(px.value(&d), Some(&[1u8, 2, 3, 4][..]));
        assert_eq!(px.data_at, px.next_at - 4);
        assert!(f.get(0x9999, 0x9999).is_none());
    }

    #[test]
    fn indefinite_and_truncation() {
        let mut d = vec![0u8; 132];
        d[128..132].copy_from_slice(b"DICM");
        d.extend_from_slice(&[0xe0, 0x7f, 0x00, 0x00, b'O', b'B', 0, 0]);
        d.extend_from_slice(&[0xff, 0xff, 0xff, 0xff]); // SQ-style indefinite
        let f = parse(&d).unwrap();
        assert_eq!(f.elements.len(), 1);
        assert!(f.elements[0].indefinite());
        assert!(f.elements[0].value(&d).is_none());
        // truncated value rejects
        let mut bad = vec![0u8; 132];
        bad[128..132].copy_from_slice(b"DICM");
        bad.extend_from_slice(&[0x08, 0x00, 0x60, 0x00, b'C', b'S', 99, 0]);
        bad.extend_from_slice(b"X");
        assert!(parse(&bad).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 64]).is_none());
        let mut d = vec![0u8; 140];
        d[128..132].copy_from_slice(b"DICX");
        assert!(parse(&d).is_none());
    }
}

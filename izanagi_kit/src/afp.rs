//! AFP/MODCA structured-field scanning.
//!
//! Each record: `0x5A` + u16 BE length + 3-byte field id (e.g. `D3 A8 8F`
//! = BPG) + flags. The id bytes identify Begin/End groups (BDT/BNG/BPG/BAG
//! ... EDT/ENG/EPG/EAG).
//!
//! ```
//! use izanagi_kit::afp::{parse, Id};
//!
//! let f = [
//!     0x5Au8, 0, 17, 0xD3, 0xA8, 0x89, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // BDT
//!     0x5A, 0, 17, 0xD3, 0xA9, 0x89, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // EDT
//! ];
//! let a = parse(&f).unwrap();
//! assert_eq!(a.fields.len(), 2);
//! assert_eq!(a.fields[0].id, Id::Bdt);
//! ```

/// Known structured-field ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id {
    /// `D3 A8 89` Begin Document.
    Bdt,
    /// `D3 A8 AF` Begin Named Group.
    Bng,
    /// `D3 A8 8F` Begin Page.
    Bpg,
    /// `D3 A8 C4` Begin Active Environment Group.
    Bag,
    /// `D3 A9 89` End Document.
    Edt,
    /// `D3 A9 AF` End Named Group.
    Eng,
    /// `D3 A9 8F` End Page.
    Epg,
    /// `D3 A9 C4` End Active Environment Group.
    Eag,
    /// `D3 A6 87` Page Descriptor.
    Pgd,
    /// `D3 EE 63` No Operation.
    Nop,
    /// Other 3-byte id.
    Other([u8; 3]),
}

/// One structured field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Field id.
    pub id: Id,
    /// Declared record length (including `0x5A`+len+id+flags).
    pub len: u16,
    /// Byte offset of the record.
    pub at: usize,
}

/// Scanned AFP stream.
pub struct Afp {
    /// Structured fields in order.
    pub fields: Vec<Field>,
}

/// Scans the structured-field chain. Returns `None` on bad `0x5A` flag or
/// a length that runs past the input.
pub fn parse(d: &[u8]) -> Option<Afp> {
    let mut fields = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        if d[i] != 0x5A || i + 6 > d.len() {
            return None;
        }
        let len = ((d[i + 1] as u16) << 8) | d[i + 2] as u16;
        if (len as usize) < 6 || i + len as usize > d.len() {
            return None;
        }
        let raw = [d[i + 3], d[i + 4], d[i + 5]];
        fields.push(Field {
            id: match raw {
                [0xD3, 0xA8, 0x89] => Id::Bdt,
                [0xD3, 0xA8, 0xAF] => Id::Bng,
                [0xD3, 0xA8, 0x8F] => Id::Bpg,
                [0xD3, 0xA8, 0xC4] => Id::Bag,
                [0xD3, 0xA9, 0x89] => Id::Edt,
                [0xD3, 0xA9, 0xAF] => Id::Eng,
                [0xD3, 0xA9, 0x8F] => Id::Epg,
                [0xD3, 0xA9, 0xC4] => Id::Eag,
                [0xD3, 0xA6, 0x87] => Id::Pgd,
                [0xD3, 0xEE, 0x63] => Id::Nop,
                o => Id::Other(o),
            },
            len,
            at: i,
        });
        i += len as usize;
    }
    if fields.is_empty() {
        None
    } else {
        Some(Afp { fields })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: [u8; 3], extra: usize) -> Vec<u8> {
        let len = 9 + extra;
        let mut v = vec![0x5A, (len >> 8) as u8, (len & 0xFF) as u8];
        v.extend_from_slice(&id);
        v.extend(std::iter::repeat(0).take(3 + extra));
        v
    }

    #[test]
    fn parses() {
        let mut f = rec([0xD3, 0xA8, 0x89], 8);
        f.extend_from_slice(&rec([0xD3, 0xA8, 0x8F], 0));
        f.extend_from_slice(&rec([0xD3, 0xA9, 0x8F], 0));
        f.extend_from_slice(&rec([0xD3, 0xEE, 0x63], 0));
        let a = parse(&f).unwrap();
        assert_eq!(a.fields.len(), 4);
        assert_eq!(a.fields[1].id, Id::Bpg);
        assert_eq!(a.fields[2].id, Id::Epg);
        assert_eq!(a.fields[3].id, Id::Nop);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x5A, 0, 4, 1, 2, 3]).is_none()); // len < 6
        let mut r = rec([0xD3, 0xA8, 0x89], 0);
        r[1] = 0xFF; // len runs past
        assert!(parse(&r).is_none());
        let mut r = rec([0xD3, 0xA8, 0x89], 0);
        r[0] = 0x00;
        assert!(parse(&r).is_none());
    }
}

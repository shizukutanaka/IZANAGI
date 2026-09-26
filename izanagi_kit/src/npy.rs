//! NumPy `.npy` array files (format versions 1.0–3.0).
//!
//! The file starts with `\x93NUMPY`, a major/minor version pair, and
//! a little-endian header length (`u16` for v1, `u32` for v2/v3).
//! The header itself is a Python dict literal
//! `{'descr': '<f8', 'fortran_order': False, 'shape': (3, 4), }`,
//! space-padded and newline-terminated. Raw array data follows.
//!
//! ```
//! use izanagi_kit::npy::parse;
//!
//! let mut d = vec![0x93];
//! d.extend_from_slice(b"NUMPY\x01\x00");
//! let h = b"{'descr': '<f8', 'fortran_order': False, 'shape': (2, 3), }";
//! let mut pad = h.to_vec();
//! pad.resize(pad.len() + (64 - (10 + pad.len()) % 64) - 1, b' ');
//! pad.push(b'\n');
//! d.extend_from_slice(&(pad.len() as u16).to_le_bytes());
//! d.extend_from_slice(&pad);
//! d.extend_from_slice(&[0u8; 48]); // 2*3 f64s
//! let a = parse(&d).unwrap();
//! assert_eq!(a.descr, "<f8");
//! assert_eq!(a.shape, vec![2, 3]);
//! assert_eq!(a.item_size(), Some(8));
//! assert_eq!(a.elems(), 6);
//! assert_eq!(a.data(&d).unwrap().len(), 48);
//! ```

use std::string::String;
use std::vec::Vec;

/// Magic bytes.
pub const MAGIC: &[u8; 6] = b"\x93NUMPY";

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

/// A parsed `.npy` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Npy {
    /// Major format version (1, 2 or 3).
    pub major: u8,
    /// Minor format version.
    pub minor: u8,
    /// `descr` dtype string (e.g. `"<f8"`, `"|u1"`, `"<i4"`).
    pub descr: String,
    /// `fortran_order` flag.
    pub fortran: bool,
    /// `shape` tuple.
    pub shape: Vec<u64>,
    /// Byte offset where the raw array data begins.
    pub data_at: usize,
}

impl Npy {
    /// Element size in bytes parsed from the trailing digits of
    /// `descr` (`|u1`→1, `<f8`→8, `<c16`→16). `None` for object or
    /// structured dtypes.
    pub fn item_size(&self) -> Option<u64> {
        let t = self.descr.trim_start_matches(['<', '>', '|', '=']);
        let digits: usize = t
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .map(|c| c.len_utf8())
            .sum();
        if digits == 0 {
            let rest: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
            if rest.is_empty() {
                return None;
            }
            return rest.parse().ok();
        }
        t.get(..digits)?.parse().ok()
    }

    /// Total element count (`shape` product; 0-d arrays count as 1).
    pub fn elems(&self) -> u64 {
        self.shape.iter().copied().product()
    }

    /// Total data bytes (`elems * item_size`), `None` when the dtype
    /// size cannot be determined.
    pub fn data_len(&self) -> Option<u64> {
        self.elems().checked_mul(self.item_size()?)
    }

    /// The raw array data inside `d`, or `None` when the buffer is
    /// shorter than the declared size.
    pub fn data<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        let end = self
            .data_at
            .checked_add(usize::try_from(self.data_len()?).ok()?)?;
        d.get(self.data_at..end)
    }
}

fn quoted<'a>(h: &'a str, key: &str) -> Option<&'a str> {
    let mut i = h.find(key.trim_matches(['\'', '"']))? + key.len();
    while h
        .as_bytes()
        .get(i)
        .is_some_and(|&b| b == b'\'' || b == b'"')
    {
        i += 1;
    }
    while h.as_bytes().get(i).is_some_and(|&b| b == b' ' || b == b':') {
        i += 1;
    }
    let q = *h.as_bytes().get(i)?;
    if q != b'\'' && q != b'"' {
        return None;
    }
    let rest = h.get(i + 1..)?;
    let end = rest.find(q as char)?;
    rest.get(..end)
}

fn bool_field(h: &str, key: &str) -> Option<bool> {
    let i = h.find(key)? + key.len();
    let rest = h.get(i..)?.trim_start_matches([' ', ':', '\'', '"']);
    Some(rest.starts_with("True"))
}

fn shape_field(h: &str, key: &str) -> Option<Vec<u64>> {
    let i = h.find(key)? + key.len();
    let rest = h.get(i..)?.trim_start_matches([' ', ':', '\'', '"']);
    let open = rest.find('(')?;
    let close = rest.get(open..)?.find(')')? + open;
    let mut v = Vec::new();
    for tok in rest.get(open + 1..close)?.split(',') {
        let t = tok.trim();
        if !t.is_empty() {
            v.push(t.parse().ok()?);
        }
    }
    Some(v)
}

/// Parse a `.npy` header. Returns `None` on a bad magic, an unknown
/// version, or a header dict missing `descr`/`shape`.
pub fn parse(d: &[u8]) -> Option<Npy> {
    if d.get(..6)? != MAGIC {
        return None;
    }
    let major = *d.get(6)?;
    let minor = *d.get(7)?;
    let (hlen, hat) = match major {
        1 => (usize::from(le16(d, 8)?), 10),
        2 | 3 => (usize::try_from(le32(d, 8)?).ok()?, 12),
        _ => return None,
    };
    let htext = core::str::from_utf8(d.get(hat..hat + hlen)?).ok()?;
    let descr = String::from(quoted(htext, "descr")?);
    let fortran = bool_field(htext, "fortran_order").unwrap_or(false);
    let shape = shape_field(htext, "shape")?;
    Some(Npy {
        major,
        minor,
        descr,
        fortran,
        shape,
        data_at: hat + hlen,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn npy(major: u8, minor: u8, dict: &[u8]) -> Vec<u8> {
        let mut d = vec![0x93];
        d.extend_from_slice(b"NUMPY");
        d.push(major);
        d.push(minor);
        let mut pad = dict.to_vec();
        let base = if major == 1 { 10 } else { 12 };
        let align = 64;
        pad.resize(pad.len() + (align - (base + pad.len()) % align) - 1, b' ');
        pad.push(b'\n');
        if major == 1 {
            d.extend_from_slice(&(pad.len() as u16).to_le_bytes());
        } else {
            d.extend_from_slice(&(pad.len() as u32).to_le_bytes());
        }
        d.extend_from_slice(&pad);
        d
    }

    #[test]
    fn v1_fortran_and_shape() {
        let mut d = npy(
            1,
            0,
            b"{'descr': '<f8', 'fortran_order': False, 'shape': (2, 3), }",
        );
        d.extend_from_slice(&[0u8; 48]);
        let a = parse(&d).unwrap();
        assert_eq!(a.descr, "<f8");
        assert!(!a.fortran);
        assert_eq!(a.shape, vec![2, 3]);
        assert_eq!(a.item_size(), Some(8));
        assert_eq!(a.elems(), 6);
        assert_eq!(a.data(&d).unwrap().len(), 48);
        assert_eq!(a.data_at, 128); // 10 + padded header
    }

    #[test]
    fn v2_u32_header_and_1d() {
        let mut d = npy(
            2,
            0,
            b"{'descr': '|u1', 'fortran_order': True, 'shape': (4,), }",
        );
        d.extend_from_slice(&[9, 8, 7, 6]);
        let a = parse(&d).unwrap();
        assert_eq!(a.major, 2);
        assert_eq!(a.descr, "|u1");
        assert!(a.fortran);
        assert_eq!(a.shape, vec![4]);
        assert_eq!(a.item_size(), Some(1));
        assert_eq!(a.data(&d), Some(&d[d.len() - 4..]));
    }

    #[test]
    fn scalar_and_unknown_dtype() {
        let d = npy(
            1,
            0,
            b"{'descr': '<i8', 'fortran_order': False, 'shape': (), }",
        );
        let a = parse(&d).unwrap();
        assert_eq!(a.shape, Vec::<u64>::new());
        assert_eq!(a.elems(), 1);
        let d2 = npy(
            1,
            0,
            b"{'descr': 'O', 'fortran_order': False, 'shape': (2,), }",
        );
        let a2 = parse(&d2).unwrap();
        assert_eq!(a2.item_size(), None);
        assert_eq!(a2.data(&d2), None);
    }

    #[test]
    fn double_quotes_ok() {
        let d = npy(
            1,
            0,
            b"{\"descr\": \"<u4\", \"fortran_order\": False, \"shape\": (1,), }",
        );
        let a = parse(&d).unwrap();
        assert_eq!(a.descr, "<u4");
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(b"\x93NOPY"), None);
        assert_eq!(parse(b"\x93NUMPY\x04\x00\x00\x00"), None); // version 4
        let d = npy(1, 0, b"{'descr': '<f8', 'fortran_order': False }");
        assert_eq!(parse(&d), None); // no shape
        let d2 = npy(1, 0, b"{'shape': (1,), }");
        assert_eq!(parse(&d2), None); // no descr
    }
}

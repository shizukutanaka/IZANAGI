//! NetCDF classic format header (`CDF`).
//!
//! The file opens with `CDF` + a version byte (1 = 32-bit
//! offsets, 2 = 64-bit offsets, 5 = CDF-5), `numrecs`, then the
//! dimension / global-attribute / variable lists. Each list is
//! either `ABSENT` (0, 0) or a tag (`NC_DIMENSION` 0x0A,
//! `NC_ATTRIBUTE` 0x0C, `NC_VARIABLE` 0x0B) plus a count. Names
//! are u32-length-prefixed and padded to a 4-byte boundary. All
//! fields are big-endian.
//!
//! ```
//! use izanagi_kit::nc::{parse, TAG_DIMENSION};
//!
//! let mut d = vec![0u8; 64];
//! d[..3].copy_from_slice(b"CDF");
//! d[3] = 1; // classic 32-bit
//! let put = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = (v >> 24) as u8; d[at + 1] = (v >> 16) as u8;
//!     d[at + 2] = (v >> 8) as u8; d[at + 3] = v as u8;
//! };
//! put(&mut d, 8, TAG_DIMENSION);
//! put(&mut d, 12, 1);      // one dim
//! put(&mut d, 16, 4);      // name len 4
//! d[20..24].copy_from_slice(b"time");
//! put(&mut d, 24, 100);    // dim length (record dim)
//! put(&mut d, 28, 0);      // gatts ABSENT tag
//! put(&mut d, 32, 0);
//! let n = parse(&d).unwrap();
//! assert_eq!(n.dims[0].name, "time");
//! assert_eq!(n.dims[0].len, 100);
//! ```

/// List tags.
pub const TAG_DIMENSION: u32 = 0x0a;
/// Attribute list tag.
pub const TAG_ATTRIBUTE: u32 = 0x0c;
/// Variable list tag.
pub const TAG_VARIABLE: u32 = 0x0b;

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

fn name(d: &[u8], at: usize) -> Option<(String, usize)> {
    let len = usize::try_from(be32(d, at)?).ok()?;
    let raw = d.get(at + 4..at + 4 + len)?;
    let next = at + 4 + len.div_ceil(4) * 4;
    Some((
        core::str::from_utf8(raw).ok()?.to_string(),
        next.min(d.len()),
    ))
}

/// One named dimension.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dim {
    /// Dimension name.
    pub name: String,
    /// Length (0 = unlimited record dim).
    pub len: u32,
}

/// One global attribute (type + count of values kept raw).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Att {
    /// Attribute name.
    pub name: String,
    /// NetCDF type code (1 byte, 2 char, 3 short, 4 int, 5 float, 6 double).
    pub xtype: u32,
    /// Number of values.
    pub count: u32,
    /// File offset of the value bytes.
    pub value_at: usize,
    /// Byte count of the value region (padded to 4).
    pub value_len: usize,
}

/// A parsed NetCDF header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nc {
    /// Format version (1, 2 or 5).
    pub version: u8,
    /// Record count (number of unlimited-dim records written).
    pub numrecs: u32,
    /// Dimension list.
    pub dims: Vec<Dim>,
    /// Global attributes.
    pub gatts: Vec<Att>,
    /// Number of variables in the var list (0 when absent).
    pub vars: u32,
}

fn xtype_size(t: u32) -> Option<u32> {
    match t {
        1 | 2 => Some(1),
        3 => Some(2),
        4 | 5 => Some(4),
        6 => Some(8),
        10 => Some(8), // NC_INT64 (CDF-5)
        11 => Some(4), // NC_UINT
        _ => None,
    }
}

fn att_list(d: &[u8], mut at: usize) -> Option<(Vec<Att>, usize)> {
    let tag = be32(d, at)?;
    let count = be32(d, at + 4)?;
    at += 8;
    if tag == 0 && count == 0 {
        return Some((Vec::new(), at));
    }
    if tag != TAG_ATTRIBUTE {
        return None;
    }
    let mut out = Vec::new();
    for _ in 0..count {
        let (nm, n) = name(d, at)?;
        at = n;
        let xtype = be32(d, at)?;
        let count = be32(d, at + 4)?;
        let sz = xtype_size(xtype)?.checked_mul(count)?.checked_add(3)? / 4 * 4;
        out.push(Att {
            name: nm,
            xtype,
            count,
            value_at: at + 8,
            value_len: usize::try_from(sz).ok()?,
        });
        at = at + 8 + usize::try_from(sz).ok()?;
        if at > d.len() {
            return None;
        }
    }
    Some((out, at))
}

/// Parse a NetCDF header through the var-list tag. Returns `None`
/// on a bad magic, unknown version or a truncated list.
pub fn parse(d: &[u8]) -> Option<Nc> {
    if d.get(..3)? != b"CDF" {
        return None;
    }
    let version = d.get(3).copied()?;
    if !(version == 1 || version == 2 || version == 5) {
        return None;
    }
    let numrecs = be32(d, 4)?;
    let mut at = 8;
    // dimension list
    let (dims, n) = {
        let tag = be32(d, at)?;
        let count = be32(d, at + 4)?;
        at += 8;
        if tag == 0 && count == 0 {
            (Vec::new(), at)
        } else if tag != TAG_DIMENSION {
            return None;
        } else {
            let mut dims = Vec::new();
            for _ in 0..count {
                let (nm, n2) = name(d, at)?;
                let len = be32(d, n2)?;
                dims.push(Dim { name: nm, len });
                at = n2 + 4;
                if at > d.len() {
                    return None;
                }
            }
            (dims, at)
        }
    };
    at = n;
    let (gatts, n) = att_list(d, at)?;
    at = n;
    // variable list: only the tag/count — entries are deep structures
    let vars = {
        let tag = be32(d, at)?;
        let count = be32(d, at + 4)?;
        if tag == 0 && count == 0 {
            0
        } else if tag != TAG_VARIABLE {
            return None;
        } else {
            count
        }
    };
    Some(Nc {
        version,
        numrecs,
        dims,
        gatts,
        vars,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn w(d: &mut Vec<u8>, v: u32) {
        d.extend_from_slice(&[(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }
    fn name_into(d: &mut Vec<u8>, s: &str) {
        w(d, s.len() as u32);
        d.extend_from_slice(s.as_bytes());
        while d.len() % 4 != 0 {
            d.push(0);
        }
    }

    #[test]
    fn dims_and_gatts() {
        let mut d = b"CDF".to_vec();
        d.push(1);
        w(&mut d, 3); // numrecs
        w(&mut d, TAG_DIMENSION);
        w(&mut d, 2);
        name_into(&mut d, "lat");
        w(&mut d, 180);
        name_into(&mut d, "lon");
        w(&mut d, 360);
        // gatts: one attr "title: char[5]"
        w(&mut d, TAG_ATTRIBUTE);
        w(&mut d, 1);
        name_into(&mut d, "title");
        w(&mut d, 2); // NC_CHAR
        w(&mut d, 5);
        d.extend_from_slice(b"hello\0\0\0");
        // vars absent
        w(&mut d, 0);
        w(&mut d, 0);
        let n = parse(&d).unwrap();
        assert_eq!(n.version, 1);
        assert_eq!(n.numrecs, 3);
        assert_eq!(n.dims.len(), 2);
        assert_eq!(n.dims[0].name, "lat");
        assert_eq!(n.dims[1].len, 360);
        assert_eq!(n.gatts.len(), 1);
        assert_eq!(n.gatts[0].name, "title");
        assert_eq!(n.gatts[0].xtype, 2);
        assert_eq!(n.gatts[0].count, 5);
        assert_eq!(&d[n.gatts[0].value_at..n.gatts[0].value_at + 5], b"hello");
        assert_eq!(n.vars, 0);
    }

    #[test]
    fn absent_lists_and_vars_count() {
        let mut d = b"CDF".to_vec();
        d.push(2); // 64-bit offsets version
        w(&mut d, 0);
        w(&mut d, 0);
        w(&mut d, 0); // dims absent
        w(&mut d, 0);
        w(&mut d, 0); // gatts absent
        w(&mut d, TAG_VARIABLE);
        w(&mut d, 7); // 7 vars (we stop at the count)
        let n = parse(&d).unwrap();
        assert_eq!(n.version, 2);
        assert!(n.dims.is_empty() && n.gatts.is_empty());
        assert_eq!(n.vars, 7);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"CDF").is_none()); // < 4 bytes
        assert!(parse(b"CDF\x07\0\0\0\0").is_none()); // bad version
        assert!(parse(b"XYZ\x01\0\0\0\0").is_none());
        // dim list tag wrong
        assert!(parse(b"CDF\x01\0\0\0\0\0\0\0\x0b\0\0\0\x01").is_none());
    }
}

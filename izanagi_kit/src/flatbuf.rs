//! FlatBuffers binary buffer (Google flatbuffers spec — the
//! "Internals of FlatBuffers" layout).
//!
//! Buffer layout: `u32 LE` offset to the root table, then an
//! optional 4-byte `file_identifier`, then the root table itself.
//! A table begins with an **i32** signed offset *back* to its
//! vtable; the vtable is `u16 vtable_len, u16 table_len, u16
//! field_offsets[]` where a zero entry means the field is absent.
//!
//! ```
//! use izanagi_kit::flatbuf::{parse, vtable, field};
//! let mut d = vec![14, 0, 0, 0];       // root table at +14
//! d.extend_from_slice(b"TEST");       // file_identifier
//! // vtable @8: len 6, table len 8, field 0 at voffset 4
//! d.extend_from_slice(&[6, 0, 8, 0, 4, 0]);
//! // table @14: i32 distance back to the vtable (14−8 = 6)
//! d.extend_from_slice(&[6, 0, 0, 0]);
//! d.extend_from_slice(&[9, 0, 0, 0]); // field 0 data
//! let b = parse(&d).unwrap();
//! assert_eq!(b.ident, Some(*b"TEST"));
//! assert_eq!(b.root, 14);
//! let vt = vtable(&d, b.root).unwrap();
//! assert_eq!(field(&d, b.root, &vt, 0), Some(18));
//! assert_eq!(field(&d, b.root, &vt, 1), None); // absent
//! ```

/// A parsed buffer head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flatbuf {
    /// Absolute offset of the root table.
    pub root: usize,
    /// The 4-byte file identifier when the root offset leaves a
    /// 4-byte gap (offset > 4), else `None`.
    pub ident: Option<[u8; 4]>,
}

/// A table's vtable descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vtable {
    /// Absolute offset of the vtable.
    pub at: usize,
    /// Byte size of the vtable itself.
    pub vtable_len: u16,
    /// Byte size of the table it describes.
    pub table_len: u16,
}

fn u16s(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32s(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn i32s(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Parse the buffer head: u32 root offset + optional identifier.
/// `None` when the root offset is out of range or < 4.
pub fn parse(d: &[u8]) -> Option<Flatbuf> {
    let root = u32s(d, 0)? as usize;
    if root < 4 || root + 4 > d.len() {
        return None;
    }
    let ident = if root >= 8 {
        let mut id = [0u8; 4];
        id.copy_from_slice(d.get(4..8)?);
        Some(id)
    } else {
        None
    };
    Some(Flatbuf { root, ident })
}

/// Resolve a table's vtable: the i32 at `table` is the distance
/// *backward* to the vtable start. `None` on bounds/type errors.
pub fn vtable(d: &[u8], table: usize) -> Option<Vtable> {
    let back = i32s(d, table)?;
    if back <= 0 {
        return None;
    }
    let at = table.checked_sub(back as usize)?;
    let vtable_len = u16s(d, at)?;
    let table_len = u16s(d, at + 2)?;
    if vtable_len < 4 || vtable_len % 2 != 0 {
        return None;
    }
    Some(Vtable {
        at,
        vtable_len,
        table_len,
    })
}

/// Number of field slots the vtable carries
/// (`(vtable_len - 4) / 2`).
pub fn field_count(vt: &Vtable) -> usize {
    (vt.vtable_len as usize - 4) / 2
}

/// Absolute offset of field `id`'s data inside the table, or
/// `None` when the field is absent (voffset 0) or `id` exceeds the
/// vtable.
pub fn field(d: &[u8], table: usize, vt: &Vtable, id: usize) -> Option<usize> {
    let ent_at = vt.at + 4 + id.checked_mul(2)?;
    if ent_at + 2 > vt.at + vt.vtable_len as usize {
        return None;
    }
    let voff = u16s(d, ent_at)? as usize;
    if voff == 0 || voff >= vt.table_len as usize {
        return None;
    }
    let at = table.checked_add(voff)?;
    if at >= d.len() {
        return None;
    }
    Some(at)
}

/// The u32 scalar stored at field `id` (helper for tests/consumers
/// that only need inline scalars).
pub fn u32_field(d: &[u8], table: usize, vt: &Vtable, id: usize) -> Option<u32> {
    u32s(d, field(d, table, vt, id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // [u32 root=14][ident "TEST"][vtable@8: len 6, tlen 8, f0=4]
    // [table@14: soffset=6, f0 data@18 u32=9]
    fn fixture() -> Vec<u8> {
        let mut d = vec![14, 0, 0, 0];
        d.extend_from_slice(b"TEST");
        d.extend_from_slice(&[6, 0, 8, 0, 4, 0]);
        d.extend_from_slice(&[6, 0, 0, 0]);
        d.extend_from_slice(&[9, 0, 0, 0]);
        d
    }

    #[test]
    fn header_and_table() {
        let d = fixture();
        let b = parse(&d).unwrap();
        assert_eq!(b.ident, Some(*b"TEST"));
        assert_eq!(b.root, 14);
        let vt = vtable(&d, b.root).unwrap();
        assert_eq!(vt.at, 8);
        assert_eq!(vt.vtable_len, 6);
        assert_eq!(vt.table_len, 8);
        assert_eq!(field_count(&vt), 1);
        assert_eq!(field(&d, b.root, &vt, 0), Some(18));
        assert_eq!(field(&d, b.root, &vt, 1), None);
        assert_eq!(field(&d, b.root, &vt, 9), None); // past vtable
                                                     // voff must stay inside the table: set table_len smaller
        let mut d2 = fixture();
        d2[10] = 4; // vtable[1] table_len = 4 → voff 4 out of range
        let vt2 = vtable(&d2, 14).unwrap();
        assert_eq!(field(&d2, 14, &vt2, 0), None);
        assert_eq!(vt2.table_len, 4);
    }

    #[test]
    fn u32_field_reads_scalar() {
        let d = fixture();
        let b = parse(&d).unwrap();
        let vt = vtable(&d, b.root).unwrap();
        assert_eq!(u32_field(&d, b.root, &vt, 0), Some(9));
    }

    #[test]
    fn no_ident() {
        // root at +4, no identifier gap at all
        let mut d = vec![4, 0, 0, 0];
        d.extend_from_slice(&[0, 0, 0, 0]); // table @4 (vtable-less stub)
        let b = parse(&d).unwrap();
        assert_eq!(b.root, 4);
        assert_eq!(b.ident, None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0, 0, 0, 0]).is_none()); // root 0 < 4
        let mut d = fixture();
        d.truncate(10); // root past end
        assert!(parse(&d).is_none());
        // negative/zero soffset
        let mut bad = vec![14, 0, 0, 0];
        bad.extend_from_slice(b"TEST");
        bad.extend_from_slice(&[6, 0, 8, 0, 4, 0]);
        bad.extend_from_slice(&[0, 0, 0, 0]); // soffset 0
        let b = parse(&bad).unwrap();
        assert!(vtable(&bad, b.root).is_none());
    }
}

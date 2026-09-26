//! Stanford PLY polygon mesh: an ASCII header (`ply`/`format`/
//! `element`/`property`/`end_header`) followed by ASCII rows or
//! binary records. All three encodings decode; in binary, float
//! properties come back as raw IEEE bits (kit keeps no f32/f64),
//! in ASCII, [`tokens`] returns the raw whitespace-split fields
//! so no precision is lost to a decimal→binary round-trip.
//!
//! ```
//! use izanagi_kit::ply;
//! let d = b"ply\nformat ascii 1.0\nelement vertex 2\nproperty float x\nproperty float y\nend_header\n1.5 2.5\n3.5 4.5\n";
//! let p = ply::parse(d).unwrap();
//! assert_eq!(p.elements.len(), 1);
//! assert_eq!(p.elements[0].name, "vertex");
//! let row0 = ply::tokens(d, &p, 0, 0).unwrap();
//! assert_eq!(row0, vec![b"1.5", b"2.5"]);
//! ```

use std::vec::Vec;

/// Property scalar type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    /// signed 8-bit.
    I8,
    /// unsigned 8-bit.
    U8,
    /// signed 16-bit.
    I16,
    /// unsigned 16-bit.
    U16,
    /// signed 32-bit.
    I32,
    /// unsigned 32-bit.
    U32,
    /// 32-bit float (raw bits).
    F32,
    /// 64-bit float (raw bits).
    F64,
}

impl Ty {
    /// Wire byte size.
    pub fn size(self) -> usize {
        match self {
            Ty::I8 | Ty::U8 => 1,
            Ty::I16 | Ty::U16 => 2,
            Ty::I32 | Ty::U32 | Ty::F32 => 4,
            Ty::F64 => 8,
        }
    }
    fn is_float(self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
    }
    fn is_signed(self) -> bool {
        matches!(self, Ty::I8 | Ty::I16 | Ty::I32)
    }
}

fn ty_of(s: &[u8]) -> Option<Ty> {
    Some(match s {
        b"char" | b"int8" => Ty::I8,
        b"uchar" | b"uint8" => Ty::U8,
        b"short" | b"int16" => Ty::I16,
        b"ushort" | b"uint16" => Ty::U16,
        b"int" | b"int32" => Ty::I32,
        b"uint" | b"uint32" => Ty::U32,
        b"float" | b"float32" => Ty::F32,
        b"double" | b"float64" => Ty::F64,
        _ => return None,
    })
}

/// One `property` declaration — scalar or list.
#[derive(Clone, Debug)]
pub struct Prop {
    /// Property name.
    pub name: std::string::String,
    /// Scalar type (for a list: element type).
    pub ty: Ty,
    /// `Some(count_ty)` when this is a list — `count_ty` is the
    /// length-prefix type, `ty` the element type.
    pub list_count_ty: Option<Ty>,
}

/// One `element` block.
#[derive(Clone, Debug)]
pub struct Element {
    /// Element name (`vertex`, `face`, …).
    pub name: std::string::String,
    /// Declared row count.
    pub count: usize,
    /// Properties in column order.
    pub props: Vec<Prop>,
}

/// File-level surface.
#[derive(Clone, Debug)]
pub struct Ply {
    /// 0 = ascii, 1 = binary_little_endian, 2 = binary_big_endian.
    pub format: u8,
    /// `comment`/`obj_info` payload lines, trimmed.
    pub comments: Vec<std::string::String>,
    /// Element blocks in declaration order.
    pub elements: Vec<Element>,
    /// Where the body begins (right after `end_header\n`).
    pub data_start: usize,
}

/// A decoded binary cell.
#[derive(Clone, Debug, PartialEq)]
pub enum PlyVal {
    /// Signed integer.
    I(i64),
    /// Unsigned integer.
    U(u64),
    /// Raw IEEE bits (`u64` so F64 fits; F32 uses the low 32).
    Bits(u64),
    /// List property: element values.
    List(Vec<PlyVal>),
}

/// Parses the header (the body may be absent or truncated —
/// row access validates).
pub fn parse(d: &[u8]) -> Option<Ply> {
    if !d.starts_with(b"ply") {
        return None;
    }
    let mut pos = 0usize;
    let mut format: Option<u8> = None;
    let mut comments = Vec::new();
    let mut elements: Vec<Element> = Vec::new();
    loop {
        let nl = d.get(pos..)?.iter().position(|&b| b == b'\n')?;
        let mut line = &d[pos..pos + nl];
        if line.ends_with(b"\r") {
            line = &line[..line.len() - 1];
        }
        let toks: Vec<&[u8]> = line
            .split(|&b| b == b' ' || b == b'\t')
            .filter(|t| !t.is_empty())
            .collect();
        pos += nl + 1;
        match toks.first().copied() {
            Some(b"ply") => {}
            Some(b"format") => {
                format = Some(match toks.get(1).copied()? {
                    b"ascii" => 0,
                    b"binary_little_endian" => 1,
                    b"binary_big_endian" => 2,
                    _ => return None,
                });
            }
            Some(b"comment") | Some(b"obj_info") => {
                let kw = toks[0].len();
                comments.push(
                    std::string::String::from_utf8_lossy(line.get(kw..).unwrap_or(&[]))
                        .trim()
                        .to_string(),
                );
            }
            Some(b"element") => {
                let name = toks.get(1)?;
                let count: usize = std::str::from_utf8(toks.get(2)?).ok()?.parse().ok()?;
                elements.push(Element {
                    name: std::string::String::from_utf8_lossy(name).into_owned(),
                    count,
                    props: Vec::new(),
                });
            }
            Some(b"property") => {
                let e = elements.last_mut()?;
                if toks.get(1).copied() == Some(b"list") {
                    let ct = ty_of(toks.get(2).copied()?)?;
                    let et = ty_of(toks.get(3).copied()?)?;
                    e.props.push(Prop {
                        name: std::string::String::from_utf8_lossy(toks.get(4)?).into_owned(),
                        ty: et,
                        list_count_ty: Some(ct),
                    });
                } else {
                    e.props.push(Prop {
                        name: std::string::String::from_utf8_lossy(toks.get(2)?).into_owned(),
                        ty: ty_of(toks.get(1).copied()?)?,
                        list_count_ty: None,
                    });
                }
            }
            Some(b"end_header") => break,
            _ => {}
        }
    }
    if elements.is_empty() {
        return None;
    }
    Some(Ply {
        format: format?,
        comments,
        elements,
        data_start: pos,
    })
}

fn int_at(d: &[u8], ty: Ty, at: usize, big: bool) -> Option<PlyVal> {
    let n = ty.size();
    let b = d.get(at..at + n)?;
    let v = b.iter().enumerate().fold(0u64, |a, (i, &x)| {
        if big {
            a | ((x as u64) << ((n - 1 - i) * 8))
        } else {
            a | ((x as u64) << (i * 8))
        }
    });
    if ty.is_float() {
        return Some(PlyVal::Bits(v));
    }
    if ty.is_signed() {
        return Some(PlyVal::I(match n {
            1 => v as u8 as i8 as i64,
            2 => v as u16 as i16 as i64,
            _ => v as u32 as i32 as i64,
        }));
    }
    Some(PlyVal::U(v))
}

/// Bytes a binary property occupies starting at `at` (reads the
/// count prefix for lists).
fn prop_bytes(d: &[u8], p: &Prop, at: usize, big: bool) -> Option<usize> {
    match p.list_count_ty {
        Some(ct) => {
            let cn = ct.size();
            let count = match int_at(d, ct, at, big)? {
                PlyVal::I(v) if v >= 0 => v as usize,
                PlyVal::U(v) => v as usize,
                _ => return None,
            };
            cn.checked_add(count.checked_mul(p.ty.size())?)
        }
        None => Some(p.ty.size()),
    }
}

fn elem_start(d: &[u8], ply: &Ply, ei: usize) -> Option<usize> {
    let big = ply.format == 2;
    let mut at = ply.data_start;
    for e in &ply.elements[..ei.min(ply.elements.len())] {
        for _ in 0..e.count {
            for p in &e.props {
                at = at.checked_add(prop_bytes(d, p, at, big)?)?;
            }
        }
    }
    Some(at)
}

/// Decodes binary cell `(row, prop_index)` of element `ei`.
/// `None` for ascii files, out-of-range rows, or truncation.
pub fn cell(d: &[u8], ply: &Ply, ei: usize, row: usize, j: usize) -> Option<PlyVal> {
    if ply.format == 0 {
        return None;
    }
    let e = ply.elements.get(ei)?;
    if row >= e.count {
        return None;
    }
    let p = e.props.get(j)?;
    let big = ply.format == 2;
    let mut at = elem_start(d, ply, ei)?;
    for _ in 0..row {
        for q in &e.props {
            at = at.checked_add(prop_bytes(d, q, at, big)?)?;
        }
    }
    for q in &e.props[..j] {
        at = at.checked_add(prop_bytes(d, q, at, big)?)?;
    }
    match p.list_count_ty {
        Some(ct) => {
            let cn = ct.size();
            let count = match int_at(d, ct, at, big)? {
                PlyVal::I(v) if v >= 0 => v as usize,
                PlyVal::U(v) => v as usize,
                _ => return None,
            };
            let mut vals = Vec::with_capacity(count);
            for i in 0..count {
                vals.push(int_at(d, p.ty, at + cn + i * p.ty.size(), big)?);
            }
            Some(PlyVal::List(vals))
        }
        None => int_at(d, p.ty, at, big),
    }
}

/// Splits ascii row `row` of element `ei` into its raw field
/// tokens — list properties appear as `count` followed by the
/// elements (the caller knows the property list). `None` for
/// binary files, out-of-range rows, or missing lines.
pub fn tokens<'a>(d: &'a [u8], ply: &Ply, ei: usize, row: usize) -> Option<Vec<&'a [u8]>> {
    if ply.format != 0 {
        return None;
    }
    let e = ply.elements.get(ei)?;
    if row >= e.count {
        return None;
    }
    let mut at = ply.data_start;
    for prior in &ply.elements[..ei] {
        for _ in 0..prior.count {
            at += d.get(at..)?.iter().position(|&b| b == b'\n')? + 1;
        }
    }
    for _ in 0..row {
        at += d.get(at..)?.iter().position(|&b| b == b'\n')? + 1;
    }
    let line_end = match d.get(at..)?.iter().position(|&b| b == b'\n') {
        Some(nl) => at + nl,
        None => d.len(),
    };
    let mut line = d.get(at..line_end)?;
    if line.ends_with(b"\r") {
        line = &line[..line.len() - 1];
    }
    Some(
        line.split(|&b| b == b' ' || b == b'\t')
            .filter(|t| !t.is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASCII: &[u8] = b"ply\nformat ascii 1.0\ncomment demo\nelement vertex 2\nproperty float x\nproperty float y\nproperty uchar red\nelement face 1\nproperty list uchar int verts\nend_header\n1.5 2.5 255\n3.5 4.5 0\n3 0 1 2\n";

    #[test]
    fn ascii_header_and_tokens() {
        let p = parse(ASCII).unwrap();
        assert_eq!(p.format, 0);
        assert_eq!(p.comments, vec!["demo"]);
        assert_eq!(p.elements.len(), 2);
        let v = &p.elements[0];
        assert_eq!(v.count, 2);
        assert_eq!(v.props.len(), 3);
        assert_eq!(v.props[2].ty, Ty::U8);
        let f = &p.elements[1];
        assert_eq!(f.props[0].list_count_ty, Some(Ty::U8));
        assert_eq!(f.props[0].ty, Ty::I32);
        let r0 = tokens(ASCII, &p, 0, 0).unwrap();
        assert_eq!(r0, vec![b"1.5", b"2.5", b"255"]);
        let r1 = tokens(ASCII, &p, 0, 1).unwrap();
        assert_eq!(r1[0], b"3.5");
        let face = tokens(ASCII, &p, 1, 0).unwrap();
        assert_eq!(face, vec![b"3", b"0", b"1", b"2"]);
        assert!(tokens(ASCII, &p, 1, 1).is_none()); // only 1 face
    }

    #[test]
    fn binary_le_and_be() {
        // vertex 1: float x, int y ; face 1: list uchar int verts
        let mut d = Vec::new();
        d.extend_from_slice(b"ply\nformat binary_little_endian 1.0\nelement vertex 1\nproperty float x\nproperty int y\nelement face 1\nproperty list uchar int verts\nend_header\n");
        d.extend_from_slice(&0x3FC0_0000u32.to_le_bytes()); // 1.5f
        d.extend_from_slice(&(-7i32).to_le_bytes());
        d.push(3);
        d.extend_from_slice(&0i32.to_le_bytes());
        d.extend_from_slice(&1i32.to_le_bytes());
        d.extend_from_slice(&2i32.to_le_bytes());
        let p = parse(&d).unwrap();
        assert_eq!(p.format, 1);
        assert_eq!(cell(&d, &p, 0, 0, 0), Some(PlyVal::Bits(0x3FC0_0000)));
        assert_eq!(cell(&d, &p, 0, 0, 1), Some(PlyVal::I(-7)));
        assert_eq!(
            cell(&d, &p, 1, 0, 0),
            Some(PlyVal::List(vec![PlyVal::I(0), PlyVal::I(1), PlyVal::I(2)]))
        );
        assert!(cell(&d, &p, 0, 1, 0).is_none()); // count=1
        assert!(tokens(&d, &p, 0, 0).is_none()); // ascii-only
    }

    #[test]
    fn binary_be_and_walk() {
        let mut d = Vec::new();
        d.extend_from_slice(
            b"ply\nformat binary_big_endian 1.0\nelement a 2\nproperty ushort v\nend_header\n",
        );
        d.extend_from_slice(&[0x12, 0x34]); // BE u16
        d.extend_from_slice(&[0xBE, 0xEF]);
        let p = parse(&d).unwrap();
        assert_eq!(cell(&d, &p, 0, 0, 0), Some(PlyVal::U(0x1234)));
        assert_eq!(cell(&d, &p, 0, 1, 0), Some(PlyVal::U(0xBEEF)));
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"ply\nformat potato 1.0\nend_header\n").is_none());
        assert!(parse(b"ply\nformat ascii 1.0\nend_header\n").is_none());
        assert!(parse(b"ply\n").is_none()); // unterminated header
                                            // truncated binary body
        let mut d = Vec::new();
        d.extend_from_slice(
            b"ply\nformat binary_little_endian 1.0\nelement v 1\nproperty int x\nend_header\n",
        );
        d.extend_from_slice(&[1, 2]); // short
        let p = parse(&d).unwrap();
        assert!(cell(&d, &p, 0, 0, 0).is_none());
    }
}

//! Minecraft NBT (Named Binary Tag) — the binary tree format of
//! level.dat and friends.
//!
//! Big-endian throughout (assembled by shifts — byte-order builtins are
//! banned for endian independence). [`parse`] expects the standard
//! root: one `TAG_Compound` with a UTF-8 name. `Float`/`Double` carry
//! their raw IEEE bits — the kit has no float type, so bits are kept
//! verbatim for a caller to convert. [`encode`] emits the canonical
//! form; `parse∘encode` is the identity. Recursion is depth-limited,
//! every length field is checked.
//!
//! ```
//! use izanagi_kit::nbt::{Tag, parse, encode};
//! let mut root: Vec<(String, Tag)> = Vec::new();
//! root.push(("name".into(), Tag::String("Bananrama".into())));
//! let doc = Tag::Compound(root);
//! let bytes = encode("hello world", &doc);
//! let (name, tag, _) = parse(&bytes).unwrap();
//! assert_eq!(name, "hello world");
//! assert_eq!(tag, doc);
//! ```

/// An NBT tag payload.
#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    /// `TAG_Byte` (1)
    Byte(i8),
    /// `TAG_Short` (2)
    Short(i16),
    /// `TAG_Int` (3)
    Int(i32),
    /// `TAG_Long` (4)
    Long(i64),
    /// `TAG_Float` (5) — raw IEEE-754 bits.
    Float(u32),
    /// `TAG_Double` (6) — raw IEEE-754 bits.
    Double(u64),
    /// `TAG_Byte_Array` (7)
    ByteArray(Vec<i8>),
    /// `TAG_String` (8) — decoded with `from_utf8_lossy`.
    String(String),
    /// `TAG_List` (9) — `(element_tag_id, items)`.
    List(u8, Vec<Tag>),
    /// `TAG_Compound` (10) — ordered named children.
    Compound(Vec<(String, Tag)>),
    /// `TAG_Int_Array` (11)
    IntArray(Vec<i32>),
    /// `TAG_Long_Array` (12)
    LongArray(Vec<i64>),
}

impl Tag {
    /// The tag's numeric id.
    pub fn id(&self) -> u8 {
        match self {
            Tag::Byte(_) => 1,
            Tag::Short(_) => 2,
            Tag::Int(_) => 3,
            Tag::Long(_) => 4,
            Tag::Float(_) => 5,
            Tag::Double(_) => 6,
            Tag::ByteArray(_) => 7,
            Tag::String(_) => 8,
            Tag::List(_, _) => 9,
            Tag::Compound(_) => 10,
            Tag::IntArray(_) => 11,
            Tag::LongArray(_) => 12,
        }
    }

    /// Named-child lookup on a `Compound`; `None` otherwise.
    pub fn get(&self, name: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(items) => items.iter().find(|(k, _)| k == name).map(|(_, v)| v),
            _ => None,
        }
    }
}

const MAX_DEPTH: usize = 512;

fn be16(d: &[u8], i: usize) -> Option<u16> {
    Some(((*d.get(i)? as u16) << 8) | *d.get(i + 1)? as u16)
}

fn be32(d: &[u8], i: usize) -> Option<u32> {
    Some(((be16(d, i)? as u32) << 16) | be16(d, i + 2)? as u32)
}

fn be64(d: &[u8], i: usize) -> Option<u64> {
    Some(((be32(d, i)? as u64) << 32) | be32(d, i + 4)? as u64)
}

fn name(d: &[u8], i: usize) -> Option<(String, usize)> {
    let n = be16(d, i)? as usize;
    let s = d.get(i + 2..i + 2 + n)?;
    Some((String::from_utf8_lossy(s).into_owned(), i + 2 + n))
}

fn payload(d: &[u8], i: usize, ty: u8, depth: usize) -> Option<(Tag, usize)> {
    if depth > MAX_DEPTH {
        return None;
    }
    match ty {
        1 => Some((Tag::Byte(*d.get(i)? as i8), i + 1)),
        2 => Some((Tag::Short(be16(d, i)? as i16), i + 2)),
        3 => Some((Tag::Int(be32(d, i)? as i32), i + 4)),
        4 => Some((Tag::Long(be64(d, i)? as i64), i + 8)),
        5 => Some((Tag::Float(be32(d, i)?), i + 4)),
        6 => Some((Tag::Double(be64(d, i)?), i + 8)),
        7 => {
            let n = be32(d, i)? as usize;
            let end = (i + 4).checked_add(n)?;
            let s = d.get(i + 4..end)?;
            Some((Tag::ByteArray(s.iter().map(|&b| b as i8).collect()), end))
        }
        8 => {
            let (s, n) = name(d, i)?;
            Some((Tag::String(s), n))
        }
        9 => {
            let ety = *d.get(i)?;
            let n = be32(d, i + 1)? as i32;
            if n < 0 {
                return None;
            }
            let mut items = Vec::new();
            let mut j = i + 5;
            for _ in 0..n {
                let (t, n2) = payload(d, j, ety, depth + 1)?;
                items.push(t);
                j = n2;
            }
            Some((Tag::List(ety, items), j))
        }
        10 => {
            let mut items = Vec::new();
            let mut j = i;
            loop {
                let t = *d.get(j)?;
                j += 1;
                if t == 0 {
                    return Some((Tag::Compound(items), j));
                }
                if t > 12 {
                    return None;
                }
                let (nm, n) = name(d, j)?;
                let (v, n2) = payload(d, n, t, depth + 1)?;
                items.push((nm, v));
                j = n2;
            }
        }
        11 => {
            let n = be32(d, i)? as usize;
            let mut v = Vec::with_capacity(n.min(1024));
            let mut j = i + 4;
            for _ in 0..n {
                v.push(be32(d, j)? as i32);
                j += 4;
            }
            Some((Tag::IntArray(v), j))
        }
        12 => {
            let n = be32(d, i)? as usize;
            let mut v = Vec::with_capacity(n.min(1024));
            let mut j = i + 4;
            for _ in 0..n {
                v.push(be64(d, j)? as i64);
                j += 8;
            }
            Some((Tag::LongArray(v), j))
        }
        _ => None,
    }
}

/// Parse a buffer: `(root_name, root_compound, bytes_used)`.
/// `None` unless the root is a `TAG_Compound`.
pub fn parse(d: &[u8]) -> Option<(String, Tag, usize)> {
    if d.first().copied() != Some(10) {
        return None;
    }
    let (nm, i) = name(d, 1)?;
    let (t, used) = payload(d, i, 10, 0)?;
    Some((nm, t, used))
}

fn w16(out: &mut Vec<u8>, v: u16) {
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

fn w32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&[(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]);
}

fn w64(out: &mut Vec<u8>, v: u64) {
    w32(out, (v >> 32) as u32);
    w32(out, v as u32);
}

fn wname(out: &mut Vec<u8>, s: &str) {
    w16(out, s.len().min(0xFFFF) as u16);
    out.extend_from_slice(&s.as_bytes()[..s.len().min(0xFFFF)]);
}

fn emit_payload(t: &Tag, out: &mut Vec<u8>) {
    match t {
        Tag::Byte(v) => out.push(*v as u8),
        Tag::Short(v) => w16(out, *v as u16),
        Tag::Int(v) => w32(out, *v as u32),
        Tag::Long(v) => w64(out, *v as u64),
        Tag::Float(v) => w32(out, *v),
        Tag::Double(v) => w64(out, *v),
        Tag::ByteArray(v) => {
            w32(out, v.len() as u32);
            out.extend(v.iter().map(|&b| b as u8));
        }
        Tag::String(s) => wname(out, s),
        Tag::List(ety, items) => {
            out.push(*ety);
            w32(out, items.len() as u32);
            for it in items {
                emit_payload(it, out);
            }
        }
        Tag::Compound(items) => {
            for (nm, v) in items {
                out.push(v.id());
                wname(out, nm);
                emit_payload(v, out);
            }
            out.push(0);
        }
        Tag::IntArray(v) => {
            w32(out, v.len() as u32);
            for x in v {
                w32(out, *x as u32);
            }
        }
        Tag::LongArray(v) => {
            w32(out, v.len() as u32);
            for x in v {
                w64(out, *x as u64);
            }
        }
    }
}

/// Encode `tag` as a named document (`TAG_Compound("name"){...}`).
/// Non-compound roots are emitted as-is — the spec's root is a named
/// compound, and [`parse`] accepts only that.
pub fn encode(name: &str, tag: &Tag) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(tag.id());
    wname(&mut out, name);
    emit_payload(tag, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_hello_world_vector() {
        // The spec's canonical document: Compound("hello world"){
        //   String("name") = "Bananrama" }.
        let want: Vec<u8> = vec![
            0x0a, 0x00, 0x0b, b'h', b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd',
            0x08, 0x00, 0x04, b'n', b'a', b'm', b'e', 0x00, 0x09, b'B', b'a', b'n', b'a', b'n',
            b'r', b'a', b'm', b'a', 0x00,
        ];
        let (nm, t, used) = parse(&want).unwrap();
        assert_eq!(nm, "hello world");
        assert_eq!(used, want.len());
        assert_eq!(t.get("name"), Some(&Tag::String("Bananrama".into())));
        assert_eq!(encode("hello world", &t), want);
    }

    #[test]
    fn every_tag_roundtrips() {
        let doc = Tag::Compound(vec![
            ("b".into(), Tag::Byte(-5)),
            ("s".into(), Tag::Short(-300)),
            ("i".into(), Tag::Int(-70_000)),
            ("l".into(), Tag::Long(-9_000_000_000)),
            ("f".into(), Tag::Float(0x3FC0_0000)),
            ("d".into(), Tag::Double(0xC002_0000_0000_0000)),
            ("ba".into(), Tag::ByteArray(vec![1, -2, 3])),
            ("str".into(), Tag::String("hi".into())),
            ("list".into(), Tag::List(3, vec![Tag::Int(1), Tag::Int(2)])),
            ("empty".into(), Tag::List(9, vec![])),
            ("c".into(), Tag::Compound(vec![("x".into(), Tag::Byte(7))])),
            ("ia".into(), Tag::IntArray(vec![4, -5])),
            ("la".into(), Tag::LongArray(vec![6, -7])),
        ]);
        let bytes = encode("root", &doc);
        let (_, got, used) = parse(&bytes).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(got, doc);
    }

    #[test]
    fn malformed_inputs_rejected() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[8]).is_none()); // root not compound
        assert!(parse(&[10, 0, 1, b'a']).is_none()); // no children, no end
                                                     // List of tag id 99 → invalid.
        assert!(parse(&[10, 0, 0, 9, 0, 1, b'x', 99, 0, 0, 0, 1, 0]).is_none());
        // ByteArray claiming more bytes than present.
        assert!(parse(&[10, 0, 0, 7, 0, 1, b'x', 0, 0, 0, 5, 1, 0]).is_none());
        // Depth bomb: 600 nested compounds — depth > MAX_DEPTH rejects.
        let mut t = Tag::Compound(vec![]);
        for _ in 0..600 {
            t = Tag::Compound(vec![("n".into(), t)]);
        }
        assert!(parse(&encode("r", &t)).is_none());
    }
}

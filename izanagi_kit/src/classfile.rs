//! JVM `.class` file (JVMS §4): `0xCAFEBABE` magic, version,
//! the constant pool, access flags, this/super class and the
//! interface table. Members and attributes are walked by count but
//! kept as offsets — full method dissection is out of scope.
//!
//! Float/double constants keep their raw `u32`/`u64` IEEE bits —
//! this crate does not use float types.
//!
//! ```
//! use izanagi_kit::classfile;
//! let mut c = vec![0xCA, 0xFE, 0xBA, 0xBE];
//! c.extend_from_slice(&[0, 0]);       // minor
//! c.extend_from_slice(&[0, 61]);      // major (Java 17)
//! c.extend_from_slice(&[0, 4]);       // cp_count = 4 (slots 1..3)
//! c.extend_from_slice(&[1, 0, 5]);    // Utf8 len 5
//! c.extend_from_slice(b"Hello");
//! c.extend_from_slice(&[7, 0, 1]);    // Class → cp[1]
//! c.extend_from_slice(&[1, 0, 5]);
//! c.extend_from_slice(b"World");
//! c.extend_from_slice(&[0x00, 0x21]); // access public|super
//! c.extend_from_slice(&[0, 2]);       // this_class
//! c.extend_from_slice(&[0, 0]);       // super = 0 (Object)
//! c.extend_from_slice(&[0, 0]);       // interfaces
//! c.extend_from_slice(&[0, 0]);       // fields
//! c.extend_from_slice(&[0, 0]);       // methods
//! c.extend_from_slice(&[0, 0]);       // attributes
//! let cf = classfile::parse(&c).unwrap();
//! assert_eq!(cf.major, 61);
//! assert_eq!(classfile::class_name(&cf, cf.this_class), Some("Hello"));
//! ```

use std::vec::Vec;

fn r16(d: &[u8], at: usize) -> Option<u32> {
    Some((*d.get(at)? as u32) << 8 | *d.get(at + 1)? as u32)
}
fn r32(d: &[u8], at: usize) -> Option<u32> {
    Some(r16(d, at)? << 16 | r16(d, at + 2)?)
}

/// A constant-pool entry (JVMS §4.4). `Float`/`Double`/`Long` keep
/// raw bits; references keep raw indices.
#[derive(Clone, Debug, PartialEq)]
pub enum Cp {
    /// `CONSTANT_Utf8` (modified UTF-8 kept verbatim).
    Utf8(Vec<u8>),
    /// `CONSTANT_Integer` bits.
    Int(u32),
    /// `CONSTANT_Float` raw IEEE bits.
    FloatBits(u32),
    /// `CONSTANT_Long` bits.
    Long(u64),
    /// `CONSTANT_Double` raw IEEE bits.
    DoubleBits(u64),
    /// `CONSTANT_Class` → Utf8 index.
    Class(u16),
    /// `CONSTANT_String` → Utf8 index.
    Str(u16),
    /// Field/Method/InterfaceMethod ref (tag 9/10/11) → indices.
    Ref(u8, u16, u16),
    /// `CONSTANT_NameAndType` → (name, descriptor) indices.
    NameType(u16, u16),
    /// `CONSTANT_MethodHandle` → (kind, ref index).
    MethodHandle(u8, u16),
    /// `CONSTANT_MethodType` → descriptor index.
    MethodType(u16),
    /// `CONSTANT_InvokeDynamic`/`Dynamic` → (bootstrap, nat) indices.
    Dynamic(u16, u16),
    /// `CONSTANT_Module`/`Package` → Utf8 index.
    Named(u8, u16),
    /// Unknown tag kept verbatim (payload bytes).
    Unknown(u8, Vec<u8>),
}

/// A parsed `.class` file.
#[derive(Clone, Debug)]
pub struct Class {
    /// Major version (45, 52, 61 …).
    pub major: u16,
    /// Minor version.
    pub minor: u16,
    /// Constant pool, 1-indexed like the spec: `cp[0]` is a hole.
    /// Long/Double occupy two slots; the second is `Cp::Unknown(0,…)`
    /// padding.
    pub cp: Vec<Cp>,
    /// Access flags (ACC_PUBLIC 0x0001, ACC_SUPER 0x0020, …).
    pub access: u16,
    /// `this_class` — index of a `Cp::Class`.
    pub this_class: u16,
    /// `super_class` — index, or 0 only for `java.lang.Object`.
    pub super_class: u16,
    /// Interface Class-index list.
    pub interfaces: Vec<u16>,
    /// Declared field count (bodies not parsed).
    pub field_count: u16,
    /// Declared method count.
    pub method_count: u16,
    /// Top-level attribute count.
    pub attr_count: u16,
}

fn cp_at(d: &[u8], at: usize, cp: &mut Vec<Cp>) -> Option<usize> {
    let tag = *d.get(at)?;
    let body = at + 1;
    let (next, e) = match tag {
        1 => {
            let n = r16(d, body)? as usize;
            let end = body + 2 + n;
            (end, Cp::Utf8(d.get(body + 2..end)?.to_vec()))
        }
        3 => (body + 4, Cp::Int(r32(d, body)?)),
        4 => (body + 4, Cp::FloatBits(r32(d, body)?)),
        5 => {
            let v = (r32(d, body)? as u64) << 32 | r32(d, body + 4)? as u64;
            cp.push(Cp::Long(v));
            (body + 8, Cp::Unknown(0, Vec::new())) // second slot
        }
        6 => {
            let v = (r32(d, body)? as u64) << 32 | r32(d, body + 4)? as u64;
            cp.push(Cp::DoubleBits(v));
            (body + 8, Cp::Unknown(0, Vec::new()))
        }
        7 => (body + 2, Cp::Class(r16(d, body)? as u16)),
        8 => (body + 2, Cp::Str(r16(d, body)? as u16)),
        9..=11 => (
            body + 4,
            Cp::Ref(tag, r16(d, body)? as u16, r16(d, body + 2)? as u16),
        ),
        12 => (
            body + 4,
            Cp::NameType(r16(d, body)? as u16, r16(d, body + 2)? as u16),
        ),
        15 => (
            body + 3,
            Cp::MethodHandle(*d.get(body)?, r16(d, body + 1)? as u16),
        ),
        16 => (body + 2, Cp::MethodType(r16(d, body)? as u16)),
        17 | 18 => (
            body + 4,
            Cp::Dynamic(r16(d, body)? as u16, r16(d, body + 2)? as u16),
        ),
        19 | 20 => (body + 2, Cp::Named(tag, r16(d, body)? as u16)),
        _ => return None, // unknown tag — corrupt
    };
    if next > d.len() {
        return None;
    }
    cp.push(e);
    Some(next)
}

/// Skips a `u16`-count × record table where each record is a fixed
/// `head` bytes followed by `u32 len + bytes` attribute lists.
/// `attr` flag = record is exactly one `u32` count of
/// (u16,u32,u32)+bytes items.
fn skip_members(d: &[u8], at: usize) -> Option<(usize, u16)> {
    let count = r16(d, at)? as u16;
    let mut i = at + 2;
    for _ in 0..count {
        i = i.checked_add(6)?; // access/name/desc
        let n = r16(d, i)? as usize;
        i += 2;
        for _ in 0..n {
            let sz = r32(d, i + 2)? as usize; // name idx then len
            i = i.checked_add(6)?.checked_add(sz)?;
        }
    }
    Some((i, count))
}

/// Parses a `.class` file.
pub fn parse(d: &[u8]) -> Option<Class> {
    if d.get(0..4)? != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let minor = r16(d, 4)? as u16;
    let major = r16(d, 6)? as u16;
    let cp_count = r16(d, 8)? as usize;
    if cp_count == 0 {
        return None;
    }
    let mut cp = Vec::with_capacity(cp_count.min(1 << 16));
    cp.push(Cp::Unknown(0, Vec::new())); // index 0 unused
    let mut i = 10usize;
    while cp.len() < cp_count {
        i = cp_at(d, i, &mut cp)?;
    }
    let access = r16(d, i)? as u16;
    let this_class = r16(d, i + 2)? as u16;
    let super_class = r16(d, i + 4)? as u16;
    i += 6;
    let ni = r16(d, i)? as usize;
    i += 2;
    let mut interfaces = Vec::with_capacity(ni.min(4096));
    for _ in 0..ni {
        interfaces.push(r16(d, i)? as u16);
        i += 2;
    }
    let (i2, field_count) = skip_members(d, i)?;
    i = i2;
    let (i3, method_count) = skip_members(d, i)?;
    i = i3;
    // top-level attributes
    let attr_count = r16(d, i)? as u16;
    i += 2;
    for _ in 0..attr_count {
        let sz = r32(d, i + 2)? as usize;
        i = i.checked_add(6)?.checked_add(sz)?;
    }
    if i > d.len() {
        return None;
    }
    Some(Class {
        major,
        minor,
        cp,
        access,
        this_class,
        super_class,
        interfaces,
        field_count,
        method_count,
        attr_count,
    })
}

/// `Cp::Utf8` text at `idx` (lossy).
pub fn utf8(cf: &Class, idx: u16) -> Option<&str> {
    match cf.cp.get(idx as usize)? {
        Cp::Utf8(b) => std::str::from_utf8(b).ok(),
        _ => None,
    }
}

/// Resolves a `Cp::Class` index to its Utf8 name.
pub fn class_name(cf: &Class, idx: u16) -> Option<&str> {
    match cf.cp.get(idx as usize)? {
        Cp::Class(u) => utf8(cf, *u),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_class() -> Vec<u8> {
        let mut c = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 61];
        c.extend_from_slice(&[0, 4]);
        c.extend_from_slice(&[1, 0, 5]);
        c.extend_from_slice(b"Hello");
        c.extend_from_slice(&[7, 0, 1]);
        c.extend_from_slice(&[1, 0, 5]);
        c.extend_from_slice(b"World");
        c.extend_from_slice(&[0x00, 0x21, 0, 2, 0, 0]);
        c.extend_from_slice(&[0, 0]);
        c.extend_from_slice(&[0, 0]);
        c.extend_from_slice(&[0, 0]);
        c.extend_from_slice(&[0, 0]);
        c
    }

    #[test]
    fn parses_hello() {
        let cf = parse(&hello_class()).unwrap();
        assert_eq!(cf.major, 61);
        assert_eq!(cf.access, 0x21);
        assert_eq!(class_name(&cf, cf.this_class), Some("Hello"));
        assert_eq!(utf8(&cf, 3), Some("World"));
        assert_eq!(cf.cp.len(), 4);
    }

    #[test]
    fn long_double_take_two_slots() {
        let mut c = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 55];
        c.extend_from_slice(&[0, 3]);
        c.extend_from_slice(&[5]); // Long
        c.extend_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
        c.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // access/this/super
        c.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
        let cf = parse(&c).unwrap();
        assert_eq!(cf.cp.len(), 3);
        match &cf.cp[1] {
            Cp::Long(v) => assert_eq!(*v, 0x1122_3344_5566_7788),
            _ => panic!(),
        }
        match &cf.cp[2] {
            Cp::Unknown(..) => {}
            _ => panic!(),
        }
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0xCA, 0xFE, 0xBA, 0xBE]).is_none());
        let mut bad = hello_class();
        bad[9] = 99; // cp_count 99 → runs out of pool
        assert!(parse(&bad).is_none());
        let mut bad2 = hello_class();
        bad2.truncate(12); // mid-utf8
        assert!(parse(&bad2).is_none());
    }

    #[test]
    fn member_tables_walked() {
        // fields=1 (no attrs), methods=0, attrs=1 SourceFile
        let mut c = hello_class();
        c.truncate(c.len() - 6); // drop field/method/attr counts
        c.extend_from_slice(&[0, 1]); // fields = 1
        c.extend_from_slice(&[0, 9, 0, 1, 0, 3]); // access/name/desc
        c.extend_from_slice(&[0, 0]); // field attrs = 0
        c.extend_from_slice(&[0, 0]); // methods = 0
        c.extend_from_slice(&[0, 1]); // attrs = 1
        c.extend_from_slice(&[0, 1, 0, 0, 0, 2, 0xAA, 0xBB]); // SourceFile len 2
        let cf = parse(&c).unwrap();
        assert_eq!(cf.field_count, 1);
        assert_eq!(cf.attr_count, 1);
    }
}

//! Apple Property List — the binary `bplist00` form plus the XML
//! `<plist>` form. Binary objects carry a 4-bit class nibble; the
//! offset/ref tables live in the 32-byte trailer. `Real` and
//! `Date` keep raw IEEE bits — this crate has no float types.
//!
//! ```
//! use izanagi_kit::plist;
//! let p = plist::parse_xml(
//!     br#"<?xml version="1\x2e0"?><plist version="1\x2e0"><dict>
//!        <key>Name</key><string>Izanami</string>
//!        <key>Count</key><integer>3</integer></dict></plist>"#,
//! )
//! .unwrap();
//! match plist::get(&p, "Name") {
//!     Some(plist::Val::Str(s)) => assert_eq!(s, "Izanami"),
//!     _ => panic!(),
//! }
//! ```

use std::string::String;
use std::vec::Vec;

/// A plist value (both forms normalize here).
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    /// Binary `0x00` marker (no XML form).
    Null,
    /// `<true/>`/`<false/>` or a binary bool.
    Bool(bool),
    /// `<integer>` — up to 128-bit big-endian.
    Int(i128),
    /// Raw IEEE real bits.
    Real(u64),
    /// `<date>` as text, or binary 8-byte raw bits.
    Date(String),
    /// `<data>` bytes.
    Data(Vec<u8>),
    /// `<string>` (XML) / ASCII+UTF-16 (binary).
    Str(String),
    /// Binary UID object.
    Uid(u64),
    /// `<array>`.
    Arr(Vec<Val>),
    /// `<dict>` — keys in file order.
    Dict(Vec<(String, Val)>),
}

/// `dict[key]`.
pub fn get<'a>(v: &'a Val, key: &str) -> Option<&'a Val> {
    match v {
        Val::Dict(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/* ------------------------------ binary ------------------------------ */

/// `bplist00` object table, decoded.
///
/// Containers hold **object indices**, not values — call
/// [`resolve`] to walk the tree into a [`Val`].
#[derive(Clone, Debug)]
pub struct Bplist {
    /// Objects in table order.
    pub objects: Vec<Obj>,
    /// Index of the root object.
    pub top: usize,
}

/// A raw binary-plist object: containers carry ref indices.
#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    /// `0x00` null marker.
    Null,
    /// `0x08`/`0x09` bool.
    Bool(bool),
    /// `0x1_` int, up to 16 bytes BE.
    Int(i128),
    /// `0x2_` real, raw IEEE bits.
    Real(u64),
    /// `0x33` date, raw 8-byte bits.
    Date(u64),
    /// `0x4_` data.
    Data(Vec<u8>),
    /// `0x5_` ASCII or `0x6_` UTF-16BE string.
    Str(String),
    /// `0x8_` uid.
    Uid(u64),
    /// `0xA_` array — element object indices.
    Arr(Vec<u64>),
    /// `0xD_` dict — (key ref, value ref) pairs.
    Dict(Vec<(u64, u64)>),
}

fn be(d: &[u8], at: usize, n: usize) -> Option<u128> {
    let s = d.get(at..at.checked_add(n)?)?;
    let mut v = 0u128;
    for &b in s {
        v = (v << 8) | b as u128;
    }
    Some(v)
}

fn u16s_to_string(d: &[u8]) -> String {
    let mut s = String::new();
    let mut i = 0;
    while i + 1 < d.len() {
        let c = ((d[i] as u16) << 8) | d[i + 1] as u16;
        if (0xD800..0xDC00).contains(&c) && i + 3 < d.len() {
            let lo = ((d[i + 2] as u16) << 8) | d[i + 3] as u16;
            if (0xDC00..0xE000).contains(&lo) {
                let cp = 0x1_0000u32 + (((c as u32 - 0xD800) << 10) | (lo as u32 - 0xDC00));
                if let Some(ch) = char::from_u32(cp) {
                    s.push(ch);
                }
                i += 4;
                continue;
            }
        }
        s.push(char::from_u32(c as u32).unwrap_or('\u{FFFD}'));
        i += 2;
    }
    s
}

/// Decodes one object at `off`; `refsz` is the trailer's
/// reference size. Returns (object, end offset).
fn obj_at(d: &[u8], off: usize, refsz: usize) -> Option<(Obj, usize)> {
    let mark = *d.get(off)?;
    let class = mark >> 4;
    let mut at = off + 1;
    let mut n = (mark & 0x0F) as u64;
    if n == 0x0F {
        let m2 = *d.get(at)?;
        if m2 >> 4 != 0x01 {
            return None;
        }
        let nbytes = 1usize << (m2 & 0x0F);
        n = be(d, at + 1, nbytes)? as u64;
        at += 1 + nbytes;
    }
    let n = n as usize;
    let body_end = match class {
        0x0 => match mark & 0x0F {
            0x00 | 0x08 | 0x09 | 0x0F => at,
            _ => return None,
        },
        0x1 | 0x2 => at.checked_add(1usize << (mark & 0x0F))?,
        0x3 => {
            if mark & 0x0F != 0x03 {
                return None;
            }
            at + 8
        }
        0x4 | 0x5 => at.checked_add(n)?,
        0x6 => at.checked_add(n.checked_mul(2)?)?,
        0x8 => at.checked_add(n + 1)?,
        0xA => at.checked_add(n.checked_mul(refsz)?)?,
        0xD => at.checked_add(n.checked_mul(refsz)?.checked_mul(2)?)?,
        _ => return None,
    };
    let v = match class {
        0x0 => match mark & 0x0F {
            0x00 => Obj::Null,
            0x08 => Obj::Bool(false),
            0x09 => Obj::Bool(true),
            _ => return None, // 0x0F fill — treated as noise
        },
        0x1 => Obj::Int(be(d, at, 1usize << (mark & 0x0F))? as i128),
        0x2 => Obj::Real(be(d, at, 1usize << (mark & 0x0F))? as u64),
        0x3 => Obj::Date(be(d, at, 8)? as u64),
        0x4 => Obj::Data(d.get(at..body_end)?.to_vec()),
        0x5 => Obj::Str(String::from_utf8_lossy(d.get(at..body_end)?).into_owned()),
        0x6 => Obj::Str(u16s_to_string(d.get(at..body_end)?)),
        0x8 => Obj::Uid(be(d, at, n + 1)? as u64),
        0xA => {
            let mut items = Vec::with_capacity(n.min(1 << 20));
            for i in 0..n {
                items.push(be(d, at + i * refsz, refsz)? as u64);
            }
            Obj::Arr(items)
        }
        0xD => {
            let mut kv = Vec::with_capacity(n.min(1 << 20));
            for i in 0..n {
                let k = be(d, at + i * refsz, refsz)? as u64;
                let v = be(d, at + n * refsz + i * refsz, refsz)? as u64;
                kv.push((k, v));
            }
            Obj::Dict(kv)
        }
        _ => return None,
    };
    Some((v, body_end))
}

/// Parses a `bplist00` file: magic + object table + trailer.
pub fn parse_bin(d: &[u8]) -> Option<Bplist> {
    if d.get(0..8)? != b"bplist00" {
        return None;
    }
    if d.len() < 40 {
        return None;
    }
    let t = d.len() - 32;
    let off_size = d[t + 6] as usize;
    let ref_size = d[t + 7] as usize;
    if off_size == 0 || off_size > 8 || ref_size == 0 || ref_size > 8 {
        return None;
    }
    let count = be(d, t + 8, 8)? as usize;
    let top = be(d, t + 16, 8)? as usize;
    let off_tab = be(d, t + 24, 8)? as usize;
    if count > (1 << 20) || top >= count {
        return None;
    }
    let off_end = off_tab.checked_add(count.checked_mul(off_size)?)?;
    if off_end > t {
        return None;
    }
    let mut objects = Vec::with_capacity(count);
    for i in 0..count {
        let off = be(d, off_tab + i * off_size, off_size)? as usize;
        if off >= off_tab || off < 8 {
            return None;
        }
        let (v, _) = obj_at(d, off, ref_size)?;
        objects.push(v);
    }
    Some(Bplist { objects, top })
}

/// Deep-resolves the root object to a `Val` (refs → values).
/// Depth-limited; `None` on a bad ref.
pub fn resolve(b: &Bplist) -> Option<Val> {
    resolve_at(b, b.top, 0)
}

fn resolve_at(b: &Bplist, i: usize, depth: usize) -> Option<Val> {
    if depth > 64 {
        return None;
    }
    let o = b.objects.get(i)?;
    Some(match o {
        Obj::Null => Val::Null,
        Obj::Bool(bo) => Val::Bool(*bo),
        Obj::Int(v) => Val::Int(*v),
        Obj::Real(v) => Val::Real(*v),
        Obj::Date(v) => Val::Date(format!("bits:{v:016x}")),
        Obj::Data(d2) => Val::Data(d2.clone()),
        Obj::Str(s) => Val::Str(s.clone()),
        Obj::Uid(u) => Val::Uid(*u),
        Obj::Arr(items) => {
            let mut out = Vec::with_capacity(items.len());
            for &r in items {
                out.push(resolve_at(b, r as usize, depth + 1)?);
            }
            Val::Arr(out)
        }
        Obj::Dict(kv) => {
            let mut out = Vec::with_capacity(kv.len());
            for &(k, v) in kv {
                let key = match resolve_at(b, k as usize, depth + 1)? {
                    Val::Str(s) => s,
                    _ => return None, // keys must be strings
                };
                out.push((key, resolve_at(b, v as usize, depth + 1)?));
            }
            Val::Dict(out)
        }
    })
}

/* ------------------------------- XML -------------------------------- */

fn tag_text(s: &str, at: &mut usize, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let rest = s.get(*at..)?;
    let body_start = rest.find(&open)? + open.len();
    let body_end = rest[body_start..].find(&close)? + body_start;
    *at += body_end + close.len();
    Some(rest[body_start..body_end].to_string())
}

fn xml_val(s: &str, at: &mut usize) -> Option<Val> {
    let rest = s.get(*at..)?.trim_start();
    *at = s.len() - rest.len();
    let rest = s.get(*at..)?;
    if rest.starts_with("<string>") {
        return Some(Val::Str(tag_text(s, at, "string")?));
    }
    if rest.starts_with("<integer>") {
        let t = tag_text(s, at, "integer")?;
        let t = t.trim();
        let neg = t.starts_with('-');
        let t = if neg { &t[1..] } else { t };
        if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let mut v = 0i128;
        for b in t.bytes() {
            v = v.checked_mul(10)?.checked_add((b - b'0') as i128)?;
        }
        return Some(Val::Int(if neg { -v } else { v }));
    }
    if rest.starts_with("<true") {
        *at += "<true/>".len();
        return Some(Val::Bool(true));
    }
    if rest.starts_with("<false") {
        *at += "<false/>".len();
        return Some(Val::Bool(false));
    }
    if rest.starts_with("<date>") {
        return Some(Val::Date(tag_text(s, at, "date")?));
    }
    if rest.starts_with("<real>") {
        // decimal kept as text — no float types in this crate
        return Some(Val::Str(tag_text(s, at, "real")?));
    }
    if rest.starts_with("<data>") {
        let t = tag_text(s, at, "data")?;
        let mut bytes = Vec::new();
        let mut acc = 0u32;
        let mut nbits = 0usize;
        for b in t.bytes() {
            let v = match b {
                b'A'..=b'Z' => b - b'A',
                b'a'..=b'z' => b - b'a' + 26,
                b'0'..=b'9' => b - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => break,
                _ => continue,
            };
            acc = (acc << 6) | v as u32;
            nbits += 6;
            if nbits >= 8 {
                bytes.push((acc >> (nbits - 8)) as u8);
                acc &= (1 << (nbits - 8)) - 1;
                nbits -= 8;
            }
        }
        return Some(Val::Data(bytes));
    }
    if rest.starts_with("<array>") {
        *at += "<array>".len();
        let mut items = Vec::new();
        loop {
            let rest = s.get(*at..)?.trim_start();
            *at = s.len() - rest.len();
            let rest = s.get(*at..)?;
            if rest.starts_with("</array>") {
                *at += "</array>".len();
                return Some(Val::Arr(items));
            }
            items.push(xml_val(s, at)?);
        }
    }
    if rest.starts_with("<dict>") {
        *at += "<dict>".len();
        let mut items = Vec::new();
        loop {
            let rest = s.get(*at..)?.trim_start();
            *at = s.len() - rest.len();
            let rest = s.get(*at..)?;
            if rest.starts_with("</dict>") {
                *at += "</dict>".len();
                return Some(Val::Dict(items));
            }
            let key = tag_text(s, at, "key")?;
            items.push((key, xml_val(s, at)?));
        }
    }
    None
}

/// Parses an XML `<plist>` document into the root value.
pub fn parse_xml(d: &[u8]) -> Option<Val> {
    let s = std::str::from_utf8(d).ok()?;
    let ps = s.find("<plist")?;
    let close = s[ps..].find('>')? + ps;
    let mut at = close + 1;
    let v = xml_val(s, &mut at)?;
    if !s.get(at..)?.contains("</plist>") {
        return None;
    }
    Some(v)
}

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Serializes a value back to canonical XML plist text.
pub fn emit_xml(v: &Val) -> String {
    let mut s = String::from("<?xml version=\"1\x2e0\"?><plist version=\"1\x2e0\">");
    emit_val(&mut s, v);
    s.push_str("</plist>");
    s
}

fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '&' => o.push_str("&amp;"),
            _ => o.push(c),
        }
    }
    o
}

fn emit_val(s: &mut String, v: &Val) {
    match v {
        Val::Null => s.push_str("<string></string>"),
        Val::Bool(true) => s.push_str("<true/>"),
        Val::Bool(false) => s.push_str("<false/>"),
        Val::Int(i) => {
            s.push_str("<integer>");
            s.push_str(&i.to_string());
            s.push_str("</integer>");
        }
        Val::Real(bits) => {
            s.push_str("<real>");
            s.push_str(&format!("bits:{bits:016x}"));
            s.push_str("</real>");
        }
        Val::Date(d2) => {
            s.push_str("<date>");
            s.push_str(&esc(d2));
            s.push_str("</date>");
        }
        Val::Data(b) => {
            s.push_str("<data>");
            for ch in b.chunks(3) {
                let acc = (ch[0] as u32) << 16
                    | (*ch.get(1).unwrap_or(&0) as u32) << 8
                    | *ch.get(2).unwrap_or(&0) as u32;
                s.push(B64[(acc >> 18) as usize & 63] as char);
                s.push(B64[(acc >> 12) as usize & 63] as char);
                s.push(if ch.len() > 1 {
                    B64[(acc >> 6) as usize & 63] as char
                } else {
                    '='
                });
                s.push(if ch.len() > 2 {
                    B64[acc as usize & 63] as char
                } else {
                    '='
                });
            }
            s.push_str("</data>");
        }
        Val::Str(t) => {
            s.push_str("<string>");
            s.push_str(&esc(t));
            s.push_str("</string>");
        }
        Val::Uid(u) => {
            s.push_str("<integer>");
            s.push_str(&u.to_string());
            s.push_str("</integer>");
        }
        Val::Arr(items) => {
            s.push_str("<array>");
            for v in items {
                emit_val(s, v);
            }
            s.push_str("</array>");
        }
        Val::Dict(items) => {
            s.push_str("<dict>");
            for (k, v) in items {
                s.push_str("<key>");
                s.push_str(&esc(k));
                s.push_str("</key>");
                emit_val(s, v);
            }
            s.push_str("</dict>");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_dict_roundtrip() {
        let x = br#"<?xml version="1\x2e0"?><plist version="1\x2e0"><dict><key>A</key><integer>7</integer><key>B</key><string>hi</string><key>C</key><true/></dict></plist>"#;
        let v = parse_xml(x).unwrap();
        assert_eq!(get(&v, "A"), Some(&Val::Int(7)));
        assert_eq!(get(&v, "B"), Some(&Val::Str("hi".into())));
        assert_eq!(get(&v, "C"), Some(&Val::Bool(true)));
        assert_eq!(get(&v, "missing"), None);
        let back = emit_xml(&v);
        let v2 = parse_xml(back.as_bytes()).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn xml_nested_array() {
        let x = br#"<plist><array><integer>1</integer><array><string>x</string></array></array></plist>"#;
        let v = parse_xml(x).unwrap();
        match &v {
            Val::Arr(a) => {
                assert_eq!(a.len(), 2);
                assert_eq!(a[0], Val::Int(1));
                match &a[1] {
                    Val::Arr(inner) => assert_eq!(inner[0], Val::Str("x".into())),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn xml_data_base64() {
        let x = br#"<plist><dict><key>d</key><data>aGk=</data></dict></plist>"#;
        let v = parse_xml(x).unwrap();
        assert_eq!(get(&v, "d"), Some(&Val::Data(b"hi".to_vec())));
    }

    #[test]
    fn bin_dict() {
        // bplist with objects: [0]="Name" ascii, [1]=dict{0→0}
        let mut d = b"bplist00".to_vec();
        // obj0 @8: ascii "Nm" — class 0x5 len 2
        d.extend_from_slice(&[0x52, b'N', b'm']);
        // obj1 @11: dict of 1 pair: key ref 0, val ref 0 — 0xD1
        d.extend_from_slice(&[0xD1, 0x00, 0x00]);
        let obj_end = d.len();
        // trailer: offset_size=1, ref_size=1, count=2, top=1, off_tab=obj_end
        let mut tr = vec![0u8; 32];
        tr[6] = 1;
        tr[7] = 1;
        tr[15] = 2; // count
        tr[23] = 1; // top
        tr[31] = obj_end as u8; // offset table loc
        let mut file = d;
        file.extend_from_slice(&[8, 11]); // offset table
        file.extend_from_slice(&tr);
        let b = parse_bin(&file).unwrap();
        assert_eq!(b.objects.len(), 2);
        assert_eq!(b.objects[0], Obj::Str("Nm".into()));
        assert_eq!(b.objects[1], Obj::Dict(vec![(0, 0)]));
        let v = resolve(&b).unwrap();
        assert_eq!(get(&v, "Nm"), Some(&Val::Str("Nm".into())));
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse_xml(&[]).is_none());
        assert!(parse_xml(b"garbage").is_none());
        assert!(parse_bin(&[]).is_none());
        assert!(parse_bin(b"bplist01").is_none());
        let mut t = b"bplist00".to_vec();
        t.extend_from_slice(&[0x51, b'x']); // obj0 = "x" 1-char
        let mut tr = vec![0u8; 32];
        tr[6] = 1;
        tr[7] = 1;
        tr[15] = 5; // count 5 but only 1 obj
        tr[31] = t.len() as u8;
        t.extend_from_slice(&[8]); // off table
        t.extend_from_slice(&tr);
        assert!(parse_bin(&t).is_none()); // count overruns offset table
    }
}

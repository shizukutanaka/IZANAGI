//! PDF (ISO 32000) — minimal reader: header version, `startxref` →
//! classic `xref` table, trailer dictionary, indirect-object lookup,
//! and the `/Root` → `/Pages` → `/Kids`/`/Count` walk for page count.
//!
//! Scope: uncompressed object streams and classic xref tables only —
//! a cross-reference *stream* (PDF ≥1.5) degrades to `None`. Reals are
//! stored as exact decimal `(mantissa × 10^exp)` pairs — no floats.
//!
//! ```
//! let doc = b"%PDF-1.4\n\
//! 1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n\
//! 2 0 obj\n<</Type/Pages/Kids[3 0 R 4 0 R]/Count 2>>\nendobj\n\
//! 3 0 obj\n<</Type/Page/Parent 2 0 R>>\nendobj\n\
//! 4 0 obj\n<</Type/Page/Parent 2 0 R>>\nendobj\n\
//! xref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000054 00000 n \n\
//! 0000000111 00000 n \n0000000154 00000 n \n\
//! trailer\n<</Size 5/Root 1 0 R>>\nstartxref\n197\n%%EOF\n";
//! let p = izanagi_kit::pdf::parse(doc).unwrap();
//! assert_eq!(izanagi_kit::pdf::page_count(doc, &p), Some(2));
//! assert_eq!(izanagi_kit::pdf::page_ids(doc, &p).unwrap(), vec![3, 4]);
//! ```

use std::collections::BTreeMap;

/// Minimal PDF object tree — enough for structure walking.
#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// Integer token.
    Int(i64),
    /// Real token as exact decimal `mant × 10^exp` (no float types).
    Real(i64, i32),
    /// `/Name` (without the slash, UTF-8-lossy).
    Name(String),
    /// Literal string `(…)` decoded (escapes processed, UTF-8-lossy).
    Str(String),
    /// Hex string `<…>` decoded to bytes.
    Hex(Vec<u8>),
    /// `[ … ]`
    Arr(Vec<Obj>),
    /// `<< … >>`
    Dict(BTreeMap<String, Obj>),
    /// Indirect reference `n g R`.
    Ref(u32, u16),
}

/// A parsed xref table + trailer.
#[derive(Debug)]
pub struct Pdf {
    /// Version × 10 (`%PDF-1.4` → 14).
    pub version: u32,
    /// Byte offset of the `xref` section.
    pub xref_at: usize,
    /// `object number → (byte offset, generation)`.
    pub map: BTreeMap<u32, (u64, u16)>,
    /// Trailer dictionary.
    pub trailer: BTreeMap<String, Obj>,
}

fn ws(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}

/// Skips whitespace and `%` comments; returns the next non-ws offset.
fn skip_ws(d: &[u8], mut i: usize) -> usize {
    while i < d.len() {
        if d[i] == b'%' {
            while i < d.len() && d[i] != b'\n' && d[i] != b'\r' {
                i += 1;
            }
        } else if ws(d[i]) {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn name_end(d: &[u8], i: usize) -> usize {
    let mut j = i;
    while j < d.len()
        && !ws(d[j])
        && !matches!(
            d[j],
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    {
        j += 1;
    }
    j
}

/// Parses one value starting at `i` (already ws-skipped); returns
/// `(obj, next_offset)`. Depth-capped.
fn parse_obj(d: &[u8], i: usize, depth: usize) -> Option<(Obj, usize)> {
    if depth > 64 {
        return None;
    }
    let i = skip_ws(d, i);
    match *d.get(i)? {
        b'/' => {
            let e = name_end(d, i + 1);
            Some((Obj::Name(decode_name(&d[i + 1..e])), e))
        }
        b'<' => {
            if *d.get(i + 1)? == b'<' {
                // dict
                let mut m = BTreeMap::new();
                let mut at = i + 2;
                loop {
                    at = skip_ws(d, at);
                    if at + 2 <= d.len() && &d[at..at + 2] == b">>" {
                        return Some((Obj::Dict(m), at + 2));
                    }
                    if *d.get(at)? != b'/' {
                        return None;
                    }
                    let ke = name_end(d, at + 1);
                    let key = decode_name(&d[at + 1..ke]);
                    let (v, nx) = parse_obj(d, ke, depth + 1)?;
                    m.insert(key, v);
                    at = nx;
                }
            }
            // hex string
            let e = i + 1;
            let mut j = e;
            while j < d.len() && d[j] != b'>' {
                j += 1;
            }
            if j >= d.len() {
                return None;
            }
            let hexs = &d[e..j];
            let mut bytes = Vec::with_capacity(hexs.len() / 2);
            let mut hi: Option<u8> = None;
            for &c in hexs {
                if ws(c) {
                    continue;
                }
                let v = (c as char).to_digit(16)? as u8;
                match hi {
                    None => hi = Some(v),
                    Some(h) => {
                        bytes.push((h << 4) | v);
                        hi = None;
                    }
                }
            }
            if let Some(h) = hi {
                bytes.push(h << 4);
            }
            Some((Obj::Hex(bytes), j + 1))
        }
        b'[' => {
            let mut v = Vec::new();
            let mut at = i + 1;
            loop {
                at = skip_ws(d, at);
                if *d.get(at)? == b']' {
                    return Some((Obj::Arr(v), at + 1));
                }
                let (o, nx) = parse_obj(d, at, depth + 1)?;
                v.push(o);
                at = nx;
            }
        }
        b'(' => {
            // literal string with nesting + escapes
            let mut depth_n = 1usize;
            let mut j = i + 1;
            let mut s = Vec::new();
            while j < d.len() && depth_n > 0 {
                let c = d[j];
                if c == b'\\' {
                    j += 1;
                    if j >= d.len() {
                        return None;
                    }
                    let e = d[j];
                    match e {
                        b'n' => s.push(b'\n'),
                        b'r' => s.push(b'\r'),
                        b't' => s.push(b'\t'),
                        b'b' => s.push(8),
                        b'f' => s.push(12),
                        b'0'..=b'7' => {
                            let mut v = (e - b'0') as u32;
                            let mut k = 0;
                            while k < 2 && j + 1 < d.len() && (b'0'..=b'7').contains(&d[j + 1]) {
                                j += 1;
                                v = v * 8 + (d[j] - b'0') as u32;
                                k += 1;
                            }
                            s.push(v as u8);
                        }
                        b'\n' | b'\r' => {} // line continuation
                        _ => s.push(e),
                    }
                    j += 1;
                    continue;
                }
                if c == b'(' {
                    depth_n += 1;
                    if depth_n > 64 {
                        return None;
                    }
                    s.push(c);
                    j += 1;
                    continue;
                }
                if c == b')' {
                    depth_n -= 1;
                    if depth_n == 0 {
                        j += 1;
                        break;
                    }
                    s.push(c);
                    j += 1;
                    continue;
                }
                s.push(c);
                j += 1;
            }
            if depth_n != 0 {
                return None;
            }
            Some((Obj::Str(String::from_utf8_lossy(&s).into_owned()), j))
        }
        b'>' | b']' => None,
        _ => {
            // number / keyword / `n g R` indirect ref
            let e = name_end(d, i);
            let tok = &d[i..e];
            if tok.is_empty() {
                return None;
            }
            match tok {
                b"true" => return Some((Obj::Bool(true), e)),
                b"false" => return Some((Obj::Bool(false), e)),
                b"null" => return Some((Obj::Null, e)),
                _ => {}
            }
            let t = core::str::from_utf8(tok).ok()?;
            let (mant, exp) = parse_num(t)?;
            // indirect ref lookahead: int ws int ws R
            if exp == 0 {
                let j = skip_ws(d, e);
                if let Some((g, ge)) = take_int(d, j) {
                    let k = skip_ws(d, ge);
                    if *d.get(k)? == b'R' {
                        return Some((Obj::Ref(mant as u32, g as u16), k + 1));
                    }
                }
            }
            let o = if exp == 0 {
                Obj::Int(mant)
            } else {
                Obj::Real(mant, exp)
            };
            Some((o, e))
        }
    }
}

/// `#xx` escapes inside names.
fn decode_name(raw: &[u8]) -> String {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'#' && i + 2 < raw.len() {
            if let Ok(v) =
                u8::from_str_radix(core::str::from_utf8(&raw[i + 1..i + 3]).unwrap_or("zz"), 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parses an exact decimal: `12` → (12,0), `1.5` → (15,-1),
/// `-0.02` → (-2,-2). `None` on non-numeric input.
fn parse_num(t: &str) -> Option<(i64, i32)> {
    if t.is_empty()
        || t.bytes()
            .any(|b| !(b.is_ascii_digit() || b == b'+' || b == b'-' || b == b'.'))
    {
        return None;
    }
    let neg = t.starts_with('-');
    let t = t.trim_start_matches(['+', '-']);
    let mut mant: i64 = 0;
    let mut exp: i32 = 0;
    let mut seen_dot = false;
    let mut seen_any = false;
    for b in t.bytes() {
        if b == b'.' {
            if seen_dot {
                return None;
            }
            seen_dot = true;
            continue;
        }
        if !b.is_ascii_digit() {
            return None;
        }
        mant = mant.checked_mul(10)?.checked_add((b - b'0') as i64)?;
        if seen_dot {
            exp -= 1;
        }
        seen_any = true;
    }
    if !seen_any {
        return None;
    }
    Some((if neg { -mant } else { mant }, exp))
}

/// Consumes an integer token at `i`; returns `(value, end_offset)`.
fn take_int(d: &[u8], i: usize) -> Option<(i64, usize)> {
    let i = skip_ws(d, i);
    let e = name_end(d, i);
    if e == i {
        return None;
    }
    let t = core::str::from_utf8(&d[i..e]).ok()?;
    if t.bytes()
        .any(|b| !(b.is_ascii_digit() || b == b'-' || b == b'+'))
    {
        return None;
    }
    t.parse().ok().map(|v| (v, e))
}

/// Parses `%PDF-x.y`, `startxref`, the xref table, and the trailer.
/// `None` on a missing header, missing xref, or malformed trailer.
pub fn parse(d: &[u8]) -> Option<Pdf> {
    // header: %PDF-a.b within the first 1024 bytes
    let head = d.get(..d.len().min(1024))?;
    let hp = head.windows(5).position(|w| w == b"%PDF-")?;
    let vline_end = {
        let mut e = hp + 5;
        while e < d.len() && d[e] != b'\n' && d[e] != b'\r' {
            e += 1;
        }
        e
    };
    let vs = core::str::from_utf8(d.get(hp + 5..vline_end)?).ok()?;
    let mut version = 0u32;
    if let Some((a, b)) = vs.split_once('.') {
        version = a.trim().parse::<u32>().ok()?.checked_mul(10)? + b.trim().parse::<u32>().ok()?;
    }

    // startxref — last occurrence wins (incremental updates append)
    let needle = b"startxref";
    let sx = d.windows(needle.len()).rposition(|w| w == needle)?;
    let xref_at = take_int(d, sx + needle.len())?.0 as usize;
    if xref_at + 4 > d.len() || &d[xref_at..xref_at + 4] != b"xref" {
        return None; // xref stream — out of scope
    }

    // xref subsections
    let mut map = BTreeMap::new();
    let mut at = skip_ws(d, xref_at + 4);
    loop {
        at = skip_ws(d, at);
        if at + 7 <= d.len() && &d[at..at + 7] == b"trailer" {
            at += 7;
            break;
        }
        let (first, e1) = take_int(d, at)?;
        let (count, e2) = take_int(d, e1)?;
        at = skip_ws(d, e2);
        for k in 0..count {
            // "nnnnnnnnnn ggggg t " (20 bytes incl trailing ws, per spec)
            let line_end = at + 20.min(d.len() - at);
            let ent = d.get(at..line_end)?;
            if ent.len() < 18 {
                return None;
            }
            let off = core::str::from_utf8(&ent[0..10])
                .ok()?
                .parse::<u64>()
                .ok()?;
            let genn = core::str::from_utf8(&ent[11..16])
                .ok()?
                .parse::<u16>()
                .ok()?;
            let ty = ent[17];
            if ty == b'n' {
                map.insert((first + k) as u32, (off, genn));
            }
            at += if at + 20 <= d.len() { 20 } else { 18 };
            // consume the line ending exactly
        }
    }

    let (trailer, _nx) = parse_obj(d, at, 0)?;
    let trailer = match trailer {
        Obj::Dict(m) => m,
        _ => return None,
    };
    Some(Pdf {
        version,
        xref_at,
        map,
        trailer,
    })
}

/// Byte offset of object `n`'s body per the xref table.
pub fn obj_at(p: &Pdf, n: u32) -> Option<usize> {
    p.map.get(&n).map(|&(o, _)| o as usize)
}

/// Parses the object at `n`'s xref offset: `n g obj <value> endobj`.
/// Returns the *value* (stream contents not included).
pub fn obj(d: &[u8], p: &Pdf, n: u32) -> Option<Obj> {
    let off = obj_at(p, n)?;
    let mut at = skip_ws(d, off);
    let (num, e) = take_int(d, at)?;
    if num != n as i64 {
        return None;
    }
    let (_gen, e2) = take_int(d, e)?;
    at = skip_ws(d, e2);
    if at + 3 > d.len() || &d[at..at + 3] != b"obj" {
        return None;
    }
    parse_obj(d, at + 3, 0).map(|(o, _)| o)
}

/// Resolves an [`Obj::Ref`] or returns the object unchanged.
fn resolve(d: &[u8], p: &Pdf, o: &Obj, depth: usize) -> Option<Obj> {
    if depth > 32 {
        return None;
    }
    match o {
        Obj::Ref(n, _) => {
            let inner = obj(d, p, *n)?;
            resolve(d, p, &inner, depth + 1).or(Some(inner))
        }
        _ => Some(o.clone()),
    }
}

/// Dictionary helper: `dict_get(&catalog, "Pages")`.
pub fn dict_get<'a>(m: &'a BTreeMap<String, Obj>, key: &str) -> Option<&'a Obj> {
    m.get(key)
}

/// The `/Root` catalog dictionary.
pub fn root(d: &[u8], p: &Pdf) -> Option<BTreeMap<String, Obj>> {
    match resolve(d, p, p.trailer.get("Root")?, 0)? {
        Obj::Dict(m) => Some(m),
        _ => None,
    }
}

/// The `/Pages` node dictionary.
pub fn pages(d: &[u8], p: &Pdf) -> Option<BTreeMap<String, Obj>> {
    let r = root(d, p)?;
    match resolve(d, p, r.get("Pages")?, 0)? {
        Obj::Dict(m) => Some(m),
        _ => None,
    }
}

/// `/Count` of the `/Pages` node — total page count.
pub fn page_count(d: &[u8], p: &Pdf) -> Option<i64> {
    match pages(d, p)?.get("Count")? {
        Obj::Int(n) => Some(*n),
        _ => None,
    }
}

/// Object numbers of the direct `/Kids` of `/Pages` (first level only;
/// intermediate page-tree nodes are returned unresolved).
pub fn page_ids(d: &[u8], p: &Pdf) -> Option<Vec<u32>> {
    let pd = pages(d, p)?;
    let kids = match pd.get("Kids")? {
        Obj::Arr(v) => v,
        _ => return None,
    };
    let mut out = Vec::with_capacity(kids.len());
    for k in kids {
        match k {
            Obj::Ref(n, _) => out.push(*n),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a 4-object doc computing real offsets for each `N 0 obj`.
    fn fixture() -> Vec<u8> {
        let objs = [
            b"1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n" as &[u8],
            b"2 0 obj\n<</Type/Pages/Kids[3 0 R 4 0 R]/Count 2>>\nendobj\n",
            b"3 0 obj\n<</Type/Page/Parent 2 0 R>>\nendobj\n",
            b"4 0 obj\n<</Type/Page/Parent 2 0 R>>\nendobj\n",
        ];
        let mut d = Vec::new();
        d.extend_from_slice(b"%PDF-1.4\n");
        let mut offs = Vec::new();
        for o in objs {
            offs.push(d.len());
            d.extend_from_slice(o);
        }
        let xref_at = d.len();
        d.extend_from_slice(b"xref\n0 5\n");
        d.extend_from_slice(b"0000000000 65535 f \n");
        for o in offs {
            d.extend_from_slice(format!("{:010} 00000 n \n", o).as_bytes());
        }
        d.extend_from_slice(b"trailer\n<</Size 5/Root 1 0 R>>\n");
        d.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_at).as_bytes());
        d
    }

    #[test]
    fn parses_xref_and_trailer() {
        let d = fixture();
        let p = parse(&d).unwrap();
        assert_eq!(p.version, 14);
        assert_eq!(p.map.len(), 4);
        match p.trailer.get("Root") {
            Some(Obj::Ref(1, _)) => {}
            other => panic!("bad trailer root: {other:?}"),
        }
    }

    #[test]
    fn resolves_objects() {
        let d = fixture();
        let p = parse(&d).unwrap();
        assert_eq!(obj_at(&p, 1), Some(9));
        match obj(&d, &p, 1).unwrap() {
            Obj::Dict(m) => assert!(m.contains_key("Pages")),
            other => panic!("obj1 not dict: {other:?}"),
        }
        assert_eq!(page_count(&d, &p), Some(2));
        assert_eq!(page_ids(&d, &p).unwrap(), vec![3, 4]);
    }

    #[test]
    fn object_tree_pieces() {
        assert_eq!(parse_num("0"), Some((0, 0)));
        assert_eq!(parse_num("-12"), Some((-12, 0)));
        assert_eq!(parse_num("3.50"), Some((350, -2)));
        assert_eq!(parse_num(".5"), Some((5, -1)));
        assert_eq!(parse_num("x"), None);
        // literal string escapes
        let (o, _) = parse_obj(b"(a\\(b\\) \\123)", 0, 0).unwrap();
        assert_eq!(o, Obj::Str("a(b) S".to_string()));
        // name # escape
        let (o2, _) = parse_obj(b"/A#20B", 0, 0).unwrap();
        assert_eq!(o2, Obj::Name("A B".to_string()));
        // array + ref
        let (o3, _) = parse_obj(b"[1 0 R /X]", 0, 0).unwrap();
        match o3 {
            Obj::Arr(v) => {
                assert_eq!(v[0], Obj::Ref(1, 0));
                assert_eq!(v[1], Obj::Name("X".to_string()));
            }
            _ => panic!(),
        }
        // dict + dict_get
        let (o4, _) = parse_obj(b"<</K 7/N null>>", 0, 0).unwrap();
        match &o4 {
            Obj::Dict(m) => assert_eq!(dict_get(m, "K"), Some(&Obj::Int(7))),
            _ => panic!(),
        }
        // real vs int
        let (o5, _) = parse_obj(b"1.5", 0, 0).unwrap();
        assert_eq!(o5, Obj::Real(15, -1));
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"no pdf here").is_none());
        let mut bad = fixture();
        let n = bad.len();
        bad.truncate(n - 6); // cut %%EOF — startxref still parses…
                             // actually cut the whole startxref line
        let mut bad = fixture();
        let pos = bad.windows(9).rposition(|w| w == b"startxref").unwrap();
        bad.truncate(pos);
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture();
        // point startxref at garbage offset
        let pos2 = bad2.windows(9).rposition(|w| w == b"startxref").unwrap();
        for i in 0..3 {
            bad2[pos2 + 10 + i] = b'9';
        }
        assert!(parse(&bad2).is_none());
    }
}

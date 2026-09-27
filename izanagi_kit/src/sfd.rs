//! FontForge SplineFont Database (`.sfd`) header/glyph scanning.
//!
//! Text format: `SplineFontDB: <ver>` on line one, `Key: value` header
//! fields (`FontName`, `FullName`, `FamilyName`, `Weight`, `Version`,
//! `Copyright`, `Encoding`, `UComments`...), then `BeginChars: <enc> <n>`
//! and per-glyph blocks `StartChar: <name>` … `EndChar` holding
//! `Encoding:`, `Width:`, `Flags:`, `Fore`/`SplineSet`/`Back` outline
//! payloads. Only the skeleton is decoded; spline data stays verbatim.
//!
//! ```
//! use izanagi_kit::sfd;
//! let d = b"SplineFontDB: 3.0\nFontName: Demo\nBeginChars: 65536 2\n\
//!           StartChar: .notdef\nEncoding: 0 -1 0\nWidth: 500\nEndChar\n\
//!           StartChar: A\nEncoding: 32 65 1\nEndChar\nEndSplineFont\n";
//! let f = sfd::parse(d).unwrap();
//! assert_eq!(f.chars.len(), 2);
//! assert_eq!(sfd::get(&f, b"FontName"), Some(b"Demo".as_ref()));
//! ```

use std::vec::Vec;

/// A glyph block between `StartChar:` and `EndChar`.
#[derive(Clone, Debug, PartialEq)]
pub struct Char {
    /// `StartChar:` name.
    pub name: Vec<u8>,
    /// `Encoding:`'s third number (glyph order), if present.
    pub gid: Option<i64>,
    /// `Encoding:`'s second number (code point), if present.
    pub code: Option<i64>,
    /// `Width:` value, if present.
    pub width: Option<i64>,
}

/// A parsed `.sfd` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Sfd {
    /// `SplineFontDB:` version text.
    pub version: Vec<u8>,
    /// Header `Key: value` pairs (file order, before `BeginChars`).
    pub header: Vec<(Vec<u8>, Vec<u8>)>,
    /// `BeginChars:` declared glyph count, if present.
    pub declared: Option<i64>,
    /// Parsed `StartChar` blocks.
    pub chars: Vec<Char>,
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && (s[a] == b' ' || s[a] == b'\t') {
        a += 1;
    }
    while b > a && (s[b - 1] == b' ' || s[b - 1] == b'\t' || s[b - 1] == b'\r') {
        b -= 1;
    }
    &s[a..b]
}

fn int(s: &[u8]) -> Option<i64> {
    let s = trim(s);
    let (neg, s) = if s.first() == Some(&b'-') {
        (true, &s[1..])
    } else {
        (false, s)
    };
    if s.is_empty() || !s.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut v: i64 = 0;
    for &b in s {
        v = v.checked_mul(10)?.checked_add((b - b'0') as i64)?;
    }
    Some(if neg { -v } else { v })
}

fn colon(s: &[u8]) -> Option<(&[u8], &[u8])> {
    let e = s.iter().position(|&b| b == b':')?;
    Some((trim(&s[..e]), trim(&s[e + 1..])))
}

/// Parses an SFD file; requires the `SplineFontDB:` first line.
pub fn parse(d: &[u8]) -> Option<Sfd> {
    let mut it = d.split(|&b| b == b'\n');
    let first = trim(it.next()?);
    if !first.starts_with(b"SplineFontDB") {
        return None;
    }
    let (_, ver) = colon(first)?;
    let mut header = Vec::new();
    let mut chars: Vec<Char> = Vec::new();
    let mut declared = None;
    let mut cur: Option<Char> = None;
    for raw in it {
        let line = trim(raw);
        if line.is_empty() {
            continue;
        }
        if line.starts_with(b"StartChar") {
            let (_, v) = colon(line)?;
            cur = Some(Char {
                name: v.to_vec(),
                gid: None,
                code: None,
                width: None,
            });
            continue;
        }
        if line.starts_with(b"EndChar") {
            if let Some(c) = cur.take() {
                chars.push(c);
            }
            continue;
        }
        if line.starts_with(b"EndSplineFont") {
            break;
        }
        if let Some(c) = cur.as_mut() {
            let (k, v) = match colon(line) {
                Some(kv) => kv,
                None => continue,
            };
            match k {
                b"Encoding" => {
                    let nums: Vec<i64> = v
                        .split(|&b| b == b' ')
                        .filter(|w| !w.is_empty())
                        .map(int)
                        .collect::<Option<Vec<i64>>>()?;
                    if nums.len() >= 3 {
                        c.code = Some(nums[1]);
                        c.gid = Some(nums[2]);
                    }
                }
                b"Width" | b"VWidth" => c.width = int(v),
                _ => {}
            }
            continue;
        }
        if line.starts_with(b"BeginChars") {
            let (_, v) = colon(line)?;
            declared = v
                .split(|&b| b == b' ')
                .filter(|w| !w.is_empty())
                .map(int)
                .collect::<Option<Vec<i64>>>()
                .and_then(|ns| ns.last().copied());
            continue;
        }
        // Other header fields and nested sections (BeginPrivate, etc.):
        // keep top-level `Key: value` pairs only.
        if line.starts_with(b"Begin") && !line.starts_with(b"BeginChars") {
            continue;
        }
        if line.starts_with(b"End") {
            continue;
        }
        if let Some((k, v)) = colon(line) {
            header.push((k.to_vec(), v.to_vec()));
        }
    }
    Some(Sfd {
        version: ver.to_vec(),
        header,
        declared,
        chars,
    })
}

/// First header `Key:` value.
pub fn get<'a>(f: &'a Sfd, key: &[u8]) -> Option<&'a [u8]> {
    f.header
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_slice())
}

/// First glyph named `name`.
pub fn find_char<'a>(f: &'a Sfd, name: &[u8]) -> Option<&'a Char> {
    f.chars.iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_glyphs() {
        let d = b"SplineFontDB: 3.2\nFontName: Demo\nFamilyName: Demo\n\
                  BeginChars: 65536 2\n\
                  StartChar: .notdef\nEncoding: 0 -1 0\nWidth: 250\nSplineSet\n1 2 m 0\nEndSplineSet\nEndChar\n\
                  StartChar: A\nEncoding: 32 65 1\nWidth: 600\nEndChar\nEndSplineFont\n";
        let f = parse(d).unwrap();
        assert_eq!(f.version, b"3.2".to_vec());
        assert_eq!(get(&f, b"FamilyName"), Some(b"Demo".as_ref()));
        assert_eq!(f.declared, Some(2));
        assert_eq!(f.chars.len(), 2);
        let a = find_char(&f, b"A").unwrap();
        assert_eq!(a.code, Some(65));
        assert_eq!(a.width, Some(600));
        let nd = find_char(&f, b".notdef").unwrap();
        assert_eq!(nd.code, Some(-1));
        assert!(find_char(&f, b"Z").is_none());
    }

    #[test]
    fn rejects_non_sfd() {
        assert!(parse(b"FontName: X\n").is_none());
        assert!(parse(b"").is_none());
    }
}

//! Apple `.strings` (OpenStep-style): `"key" = "value";` entries with
//! `//` and `/* */` comments. Escapes `\\` `\"` `\n` `\t` `\r`.
//!
//! ```
//! use izanagi_kit::strings::parse;
//!
//! let d = b"/* c */\n\"ok\" = \"OK\";\n\"cancel\" = \"\\u201cNo\\u201d\";\n";
//! let s = parse(d).unwrap();
//! assert_eq!(s.entries.len(), 2);
//! ```

/// Parsed `.strings` table.
#[derive(Debug, Clone)]
pub struct Strings {
    /// `(key, value)` pairs in file order.
    pub entries: Vec<(String, String)>,
}

fn hex4(d: &[u8], i: usize) -> Option<(u32, usize)> {
    let mut v = 0u32;
    for k in 0..4 {
        let c = *d.get(i + k)?;
        v = v * 16 + (c as char).to_digit(16)?;
    }
    Some((v, i + 4))
}

fn quoted(d: &[u8], mut i: usize) -> Option<(String, usize)> {
    if d.get(i) != Some(&b'"') {
        return None;
    }
    i += 1;
    let mut out: Vec<u8> = Vec::new();
    while i < d.len() {
        match d[i] {
            b'"' => return Some((String::from_utf8(out).ok()?, i + 1)),
            b'\\' => {
                i += 1;
                match *d.get(i)? {
                    b'n' => out.push(b'\n'),
                    b't' => out.push(b'\t'),
                    b'r' => out.push(b'\r'),
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'u' | b'U' => {
                        let (v, j) = hex4(d, i + 1)?;
                        let mut buf = [0u8; 4];
                        out.extend_from_slice(char::from_u32(v)?.encode_utf8(&mut buf).as_bytes());
                        i = j - 1;
                    }
                    _ => return None,
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    None
}

fn skip_ws(d: &[u8], mut i: usize) -> Option<usize> {
    loop {
        while i < d.len() && d[i].is_ascii_whitespace() {
            i += 1;
        }
        if d[i..].starts_with(b"//") {
            while i < d.len() && d[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if d[i..].starts_with(b"/*") {
            let e = d[i + 2..]
                .windows(2)
                .position(|w| w == b"*/")
                .map(|p| p + i + 4)?;
            i = e;
            continue;
        }
        return Some(i);
    }
}

/// Parse a `.strings` file. `None` on malformed tokens.
pub fn parse(data: &[u8]) -> Option<Strings> {
    let mut entries = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        i = skip_ws(data, i)?;
        if i >= data.len() {
            break;
        }
        let (k, j) = quoted(data, i)?;
        i = skip_ws(data, j)?;
        if data.get(i) != Some(&b'=') {
            return None;
        }
        i = skip_ws(data, i + 1)?;
        let (v, j) = quoted(data, i)?;
        i = skip_ws(data, j)?;
        if data.get(i) != Some(&b';') {
            return None;
        }
        i += 1;
        entries.push((k, v));
    }
    if entries.is_empty() {
        return None;
    }
    Some(Strings { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"// c\n\"a\"=\"1\"; /* x */ \"b\" = \"two\\n\";\n\"c\" = \"\\U00e9\";\n";
        let s = parse(d).unwrap();
        assert_eq!(s.entries.len(), 3);
        assert_eq!(s.entries[1].1, "two\n");
        assert_eq!(s.entries[2].1, "é");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"\"a\" = \"1\"").is_none()); // missing ';'
        assert!(parse(b"a = \"1\";").is_none());
        assert!(parse(b"/* unclosed").is_none());
    }
}

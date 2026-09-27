//! BibTeX `.bib` databases: `@type{key, field = value, ...}` entries plus
//! `@string` and `@preamble` (BibTeX file format).
//!
//! ```
//! use izanagi_kit::bibtex::parse;
//!
//! let d = b"@article{knuth84,\n  author = {Donald Knuth},\n  year = 1984\n}\n";
//! let b = parse(d).unwrap();
//! assert_eq!(b.entries[0].kind, "article");
//! assert_eq!(b.entries[0].get("author"), Some("Donald Knuth"));
//! ```

/// One `@kind{...}` record.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Entry kind lowercased (`article`, `book`, `string`, `preamble`, ...).
    pub kind: String,
    /// Citation key (absent for `string`/`preamble`).
    pub key: Option<String>,
    /// Field assignments in written order.
    pub fields: Vec<(String, String)>,
}

impl Entry {
    /// Case-insensitive field lookup.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Parsed `.bib` file.
#[derive(Debug, Clone)]
pub struct Bib {
    /// Entries in file order.
    pub entries: Vec<Entry>,
}

fn skip_ws(d: &[u8], mut i: usize) -> usize {
    while i < d.len() && (d[i] as char).is_whitespace() {
        i += 1;
    }
    i
}

fn name(d: &[u8], mut i: usize) -> Option<(String, usize)> {
    let s = i;
    while i < d.len() && (d[i].is_ascii_alphanumeric() || matches!(d[i], b'_' | b'-' | b':' | b'/'))
    {
        i += 1;
    }
    if i == s {
        return None;
    }
    Some((String::from_utf8_lossy(&d[s..i]).into_owned(), i))
}

/// Read a value: `{...}` balanced, `"..."` quoted, or a bare word; `i` points
/// past the value on success.
fn value(d: &[u8], i0: usize) -> Option<(String, usize)> {
    let i = skip_ws(d, i0);
    match *d.get(i)? {
        b'{' => {
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < d.len() && depth > 0 {
                if d[j] == b'{' {
                    depth += 1;
                } else if d[j] == b'}' {
                    depth -= 1;
                }
                j += 1;
            }
            if depth != 0 {
                return None;
            }
            Some((
                String::from_utf8_lossy(&d[i + 1..j - 1]).trim().to_string(),
                j,
            ))
        }
        b'"' => {
            let mut j = i + 1;
            while j < d.len() && d[j] != b'"' {
                j += 1;
            }
            if j >= d.len() {
                return None;
            }
            Some((String::from_utf8_lossy(&d[i + 1..j]).to_string(), j + 1))
        }
        _ => {
            let (w, j) = name(d, i)?;
            Some((w, j))
        }
    }
}

/// Parse the database.
pub fn parse(d: &[u8]) -> Option<Bib> {
    let mut entries = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        // find '@' — content outside entries is a BibTeX comment
        let Some(rel) = d[i..].iter().position(|&b| b == b'@') else {
            break;
        };
        i += rel + 1;
        let (kind, j) = name(d, i)?;
        let j = skip_ws(d, j);
        if d.get(j) != Some(&b'{') && d.get(j) != Some(&b'(') {
            return None;
        }
        let close = if d[j] == b'{' { b'}' } else { b')' };
        let mut p = j + 1;
        let p = &mut p;
        let mut key = None;
        let mut fields = Vec::new();
        // read until close: first token may be the key or a field name
        loop {
            *p = skip_ws(d, *p);
            match *d.get(*p)? {
                b'}' | b')' if d[*p] == close => {
                    *p += 1;
                    break;
                }
                _ => {}
            }
            let saved = *p;
            match name(d, *p) {
                Some((w, j2)) => {
                    *p = skip_ws(d, j2);
                    if d.get(*p) == Some(&b'=') {
                        *p += 1;
                        let (v, j3) = value(d, *p)?;
                        *p = j3;
                        fields.push((w, v));
                    } else if key.is_none() {
                        key = Some(w);
                    } else {
                        return None;
                    }
                }
                None => {
                    // @preamble/@string open straight with a value
                    if key.is_some() {
                        return None;
                    }
                    let (v, j3) = value(d, saved)?;
                    key = Some(v);
                    *p = j3;
                }
            }
            *p = skip_ws(d, *p);
            if d.get(*p) == Some(&b',') {
                *p += 1;
            }
        }
        entries.push(Entry {
            kind: kind.to_lowercase(),
            key,
            fields,
        });
    }
    Some(Bib { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"@string{j = {Journ}}\n@book{a, title = \"T\", year = 2001}\n@preamble{\"x\"}\n";
        let b = parse(d).unwrap();
        assert_eq!(b.entries.len(), 3);
        assert_eq!(b.entries[0].kind, "string");
        assert_eq!(b.entries[1].key.as_deref(), Some("a"));
        assert_eq!(b.entries[1].get("TITLE"), Some("T"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"@article").is_none());
        assert!(parse(b"@x{k, f = {unclosed\n").is_none());
    }
}

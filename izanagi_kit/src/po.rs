//! Gettext PO (Portable Object) catalogues: `#` comments, `#,` flag
//! lines, `msgid`/`msgid_plural`/`msgstr`/`msgstr[n]`/`msgctxt` C-string
//! literals with multiline `"..."` continuation (gettext manual §3.4).
//!
//! ```
//! use izanagi_kit::po::parse;
//!
//! let d = b"#, fuzzy\nmsgid \"hi\"\nmsgstr \"bonjour\\n\"\n";
//! let p = parse(d).unwrap();
//! assert_eq!(p.entries[0].flags, vec!["fuzzy"]);
//! assert_eq!(p.entries[0].msgid.as_deref(), Some("hi"));
//! ```

/// One catalogue entry (context, ids, translations, flags).
#[derive(Debug, Clone, Default)]
pub struct Entry {
    /// `# ` translator comments.
    pub comments: Vec<String>,
    /// `#,` flag list (`fuzzy`, `c-format`, ...).
    pub flags: Vec<String>,
    /// `msgctxt` context if present.
    pub ctxt: Option<String>,
    /// `msgid` (the source string; `None` until it appears).
    pub msgid: Option<String>,
    /// `msgid_plural` if present.
    pub msgid_plural: Option<String>,
    /// `msgstr` for singular entries.
    pub msgstr: Option<String>,
    /// `msgstr[n]` plural translations, `n`-indexed order.
    pub msgstr_plural: Vec<(usize, String)>,
}

/// A parsed catalogue.
#[derive(Debug, Clone)]
pub struct Po {
    /// The header entry (first entry with empty `msgid`), unparsed.
    pub header: Option<Entry>,
    /// Non-header entries in file order.
    pub entries: Vec<Entry>,
}

/// Which string field a `"..."` continuation line appends to.
#[derive(Clone, Copy)]
enum Field {
    Id,
    IdPlural,
    Str,
    StrPlural(usize),
    Ctxt,
}

fn unquote(t: &str) -> Option<String> {
    let mut out = String::new();
    let mut it = t.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                _ => return None,
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn quoted(t: &str) -> Option<String> {
    let t = t.trim();
    if t.len() < 2 || !t.starts_with('"') || !t.ends_with('"') {
        return None;
    }
    unquote(&t[1..t.len() - 1])
}

fn append(e: &mut Entry, f: Field, s: &str) -> Option<()> {
    match f {
        Field::Id => e.msgid.as_mut()?.push_str(s),
        Field::IdPlural => e.msgid_plural.as_mut()?.push_str(s),
        Field::Str => e.msgstr.as_mut()?.push_str(s),
        Field::StrPlural(n) => {
            let last = e.msgstr_plural.last_mut()?;
            if last.0 != n {
                return None;
            }
            last.1.push_str(s);
        }
        Field::Ctxt => e.ctxt.as_mut()?.push_str(s),
    }
    Some(())
}

/// Parse a PO file. `None` on malformed directive or unterminated quote.
pub fn parse(data: &[u8]) -> Option<Po> {
    let text = std::str::from_utf8(data).ok()?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut cur = Entry::default();
    let mut open = false; // a directive has been seen for `cur`
    let mut field: Option<Field> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            if open {
                entries.push(std::mem::take(&mut cur));
                open = false;
                field = None;
            }
            continue;
        }
        if let Some(c) = t.strip_prefix("#,") {
            cur.flags.extend(
                c.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            );
            continue;
        }
        if let Some(c) = t.strip_prefix('#') {
            cur.comments.push(c.trim().to_string());
            continue;
        }
        if t.starts_with('"') {
            let f = field?;
            let s = quoted(t)?;
            append(&mut cur, f, &s)?;
            continue;
        }
        let (kw, arg) = t.split_once(' ').unwrap_or((t, ""));
        let arg = arg.trim();
        let v = quoted(arg)?;
        match kw {
            "msgctxt" => {
                cur.ctxt = Some(v);
                field = Some(Field::Ctxt);
            }
            "msgid" => {
                cur.msgid = Some(v);
                field = Some(Field::Id);
            }
            "msgid_plural" => {
                cur.msgid_plural = Some(v);
                field = Some(Field::IdPlural);
            }
            "msgstr" => {
                cur.msgstr = Some(v);
                field = Some(Field::Str);
            }
            _ => {
                let n = kw
                    .strip_prefix("msgstr[")?
                    .strip_suffix(']')?
                    .parse::<usize>()
                    .ok()?;
                cur.msgstr_plural.push((n, v));
                field = Some(Field::StrPlural(n));
            }
        }
        open = true;
    }
    if open {
        entries.push(cur);
    }
    if entries.is_empty() {
        return None;
    }
    let header = if entries.first()?.msgid.as_deref() == Some("") {
        Some(entries.remove(0))
    } else {
        None
    };
    Some(Po { header, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"msgid \"\"\nmsgstr \"Project-Id: x\\n\"\n\n# note\nmsgctxt \"ui\"\nmsgid \"a\"\nmsgid_plural \"as\"\nmsgstr[0] \"b\"\nmsgstr[1] \"c\"\n";
        let p = parse(d).unwrap();
        assert!(p.header.is_some());
        assert_eq!(p.entries.len(), 1);
        assert_eq!(p.entries[0].ctxt.as_deref(), Some("ui"));
        assert_eq!(p.entries[0].msgstr_plural.len(), 2);
    }

    #[test]
    fn multiline() {
        let d = b"msgid \"a\"\n\"b\"\nmsgstr \"x\"\n\"y\"\n";
        let p = parse(d).unwrap();
        assert_eq!(p.entries[0].msgid.as_deref(), Some("ab"));
        assert_eq!(p.entries[0].msgstr.as_deref(), Some("xy"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"msgid \"unterminated\n").is_none());
        assert!(parse(b"msgid a\n").is_none());
    }
}

//! Mozilla Project Fluent (`.ftl`): `id = value` messages, `-id = v`
//! terms, `-attr = v` attributes on the preceding message, `#`/`##`/`###`
//! comments, indented continuation lines.
//!
//! ```
//! use izanagi_kit::ftl::parse;
//!
//! let d = b"# c\nhello = Hello { $name }\n    cont\n    .title = T\n-term = T2\nbye = Bye\n";
//! let f = parse(d).unwrap();
//! assert_eq!(f.messages.len(), 2);
//! assert_eq!(f.messages[0].value.as_deref(), Some("Hello { $name } cont"));
//! assert_eq!(f.messages[0].attrs[0].0, "title");
//! assert_eq!(f.terms[0].id, "term");
//! ```

/// One message or term (terms have `-` prefixed ids).
#[derive(Debug, Clone)]
pub struct Message {
    /// Identifier (no leading `-` for terms).
    pub id: String,
    /// Joined value lines (`None` when the entry has attributes only).
    pub value: Option<String>,
    /// Indented `.attr = v` attributes attached to this entry.
    pub attrs: Vec<(String, String)>,
}

/// Parsed `.ftl` resource.
#[derive(Debug, Clone)]
pub struct Ftl {
    /// Messages in order.
    pub messages: Vec<Message>,
    /// Terms in order.
    pub terms: Vec<Message>,
    /// Comment lines (`#`, `##`, `###`) verbatim after the `#`s.
    pub comments: Vec<String>,
}

fn ident(t: &str) -> Option<&str> {
    if !t.as_bytes().first()?.is_ascii_alphabetic() {
        return None;
    }
    let i = t
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(t.len());
    Some(&t[..i])
}

/// Parse a Fluent resource. `None` on a non-continuation junk line or a
/// stray `.attr`/indented line with no preceding entry.
pub fn parse(data: &[u8]) -> Option<Ftl> {
    let text = std::str::from_utf8(data).ok()?;
    let mut f = Ftl {
        messages: Vec::new(),
        terms: Vec::new(),
        comments: Vec::new(),
    };
    // `cur` names which vec (messages/terms) + index the current entry
    // lives in.
    enum Which {
        Msg,
        Term,
    }
    let mut cur: Option<(Which, usize)> = None;
    let mut cur_attr: Option<usize> = None; // continuation targets last attr
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with('#') {
            let t = line.trim_start_matches('#');
            f.comments.push(t.trim().to_string());
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            let (w, mi) = cur.as_ref().map(|(w, i)| (w, *i))?;
            let v = line.trim();
            // indented `.attr = value` attaches an attribute; anything else
            // is a continuation of the current value or last attribute
            if let Some(rest) = v.strip_prefix('.') {
                let id_end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                    .unwrap_or(rest.len());
                if id_end > 0 {
                    let after = rest[id_end..].trim_start();
                    if let Some(av) = after.strip_prefix('=') {
                        let m = match w {
                            Which::Msg => &mut f.messages[mi],
                            Which::Term => &mut f.terms[mi],
                        };
                        m.attrs
                            .push((rest[..id_end].to_string(), av.trim().to_string()));
                        cur_attr = Some(m.attrs.len() - 1);
                        continue;
                    }
                }
            }
            let m = match w {
                Which::Msg => &mut f.messages[mi],
                Which::Term => &mut f.terms[mi],
            };
            if let Some(ai) = cur_attr {
                m.attrs[ai].1.push(' ');
                m.attrs[ai].1.push_str(v);
            } else if let Some(val) = &mut m.value {
                val.push(' ');
                val.push_str(v);
            } else {
                m.value = Some(v.to_string());
            }
            continue;
        }
        cur_attr = None;
        if let Some(rest) = line.strip_prefix('-') {
            // top-level `-id = value` is a term
            let id_end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .unwrap_or(rest.len());
            if id_end == 0 {
                return None;
            }
            let after = rest[id_end..].trim_start();
            let v = after.strip_prefix('=')?.trim();
            f.terms.push(Message {
                id: rest[..id_end].to_string(),
                value: if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                },
                attrs: Vec::new(),
            });
            cur = Some((Which::Term, f.terms.len() - 1));
            continue;
        }
        // `id = value` message
        let id = ident(line)?.to_string();
        let after = line[id.len()..].trim_start();
        let v = after.strip_prefix('=')?.trim();
        f.messages.push(Message {
            id,
            value: if v.is_empty() {
                None
            } else {
                Some(v.to_string())
            },
            attrs: Vec::new(),
        });
        cur = Some((Which::Msg, f.messages.len() - 1));
    }
    if f.messages.is_empty() && f.terms.is_empty() {
        return None;
    }
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"hello = Hi\n  cont\n  more\n  .attr = a\n  attrc\n-term = T\nbye =\n  only\n";
        let f = parse(d).unwrap();
        assert_eq!(f.messages.len(), 2);
        assert_eq!(f.messages[0].value.as_deref(), Some("Hi cont more"));
        assert_eq!(f.messages[0].attrs[0].1, "a attrc");
        assert_eq!(f.terms[0].id, "term");
        assert_eq!(f.messages[1].value.as_deref(), Some("only"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"  orphan\n").is_none());
        assert!(parse(b"junk\n").is_none());
        assert!(parse(b"").is_none());
    }
}

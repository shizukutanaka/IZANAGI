//! W3C XML 1.0 well-formedness parser: a strict event-stream reader for
//! elements, attributes, text, CDATA, comments, processing instructions,
//! the XML declaration, and DOCTYPE. [`parse`] returns the whole event
//! list or `None` on any malformedness — mismatched tags, multiple
//! roots, unexpanded entities, `]]>` in text, duplicate attributes —
//! which is the guarantee `rss`, `gpx`, `svg`-style consumers in this
//! kit were re-implementing ad hoc. [`escape`] and [`unescape`] cover
//! the five predefined entities plus `&#…;`/`&#x…;` character refs.
//!
//! The name grammar follows the common ASCII core of the spec
//! (`NameStartChar` letters, `_`, `:`; `NameChar` adds `-`, `.`, digits)
//! plus any non-ASCII scalar — combining/surrogate edge cases are out of
//! scope, matching what real lightweight parsers accept.
//!
//! ```
//! use izanagi_kit::xml::{parse, Ev};
//! let d = parse("<a x='1&amp;2'>hi<b/></a>").unwrap();
//! match &d[0] {
//!     Ev::Start { name, attrs, empty } => {
//!         assert_eq!(name, "a");
//!         assert_eq!(attrs[0], ("x".to_string(), "1&2".to_string()));
//!         assert!(!empty);
//!     }
//!     _ => panic!(),
//! }
//! ```

use std::string::String;
use std::vec::Vec;

/// One XML event from [`parse`].
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// `<name attr="v">` or `<name …/>` (`empty` marks the self-closed form;
    /// no matching `End` follows for it).
    Start {
        /// Tag name.
        name: String,
        /// `(name, value)` pairs in document order; values are entity-decoded.
        attrs: Vec<(String, String)>,
        /// True for `<tag/>`.
        empty: bool,
    },
    /// `</name>`.
    End {
        /// Tag name.
        name: String,
    },
    /// Character data outside markup (entity- and char-ref-decoded).
    Text(String),
    /// `<![CDATA[…]]>` contents, verbatim.
    CData(String),
    /// `<!-- … -->` contents.
    Comment(String),
    /// `<?target body?>` (excluding the XML declaration, which is [`Ev::Decl`]).
    Pi {
        /// PI target.
        target: String,
        /// PI body text.
        body: String,
    },
    /// `<?xml version="1.0" …?>` declaration body.
    Decl(String),
    /// `<!DOCTYPE …>` raw contents (internal subset kept verbatim).
    Doctype(String),
}

fn name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == ':' || !c.is_ascii()
}

fn name_char(c: char) -> bool {
    name_start(c) || c.is_ascii_digit() || c == '-' || c == '.'
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

fn ws(s: &str, i: &mut usize) {
    while s[*i..].chars().next().is_some_and(is_ws) {
        *i += s[*i..].chars().next().map_or(1, |c| c.len_utf8());
    }
}

fn name<'a>(s: &'a str, i: &mut usize) -> Option<&'a str> {
    let start = *i;
    let mut it = s[start..].char_indices();
    let (_, c0) = it.next()?;
    if !name_start(c0) {
        return None;
    }
    let mut end = start + c0.len_utf8();
    for (off, c) in it {
        if !name_char(c) {
            break;
        }
        end = start + off + c.len_utf8();
    }
    *i = end;
    Some(&s[start..end])
}

/// Decode the five predefined entities and numeric character refs
/// (`&#65;`, `&#x41;`). `None` on an unknown or malformed reference —
/// bare `&` is never passed through, and the result is the spec's
/// replacement rule rather than leniency.
pub fn unescape(s: &str) -> Option<String> {
    if !s.contains('&') {
        return Some(s.to_string());
    }
    let mut out = String::new();
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        let tail = &rest[p..];
        let semi = tail.find(';')?;
        let ent = &tail[1..semi];
        let decoded: String = match ent {
            "amp" => "&".into(),
            "lt" => "<".into(),
            "gt" => ">".into(),
            "apos" => "'".into(),
            "quot" => "\"".into(),
            _ => {
                let hex = ent.strip_prefix("#x").or_else(|| ent.strip_prefix("#X"));
                let v = match hex {
                    Some(h) => u32::from_str_radix(h, 16).ok()?,
                    None => ent.strip_prefix('#').and_then(|d| d.parse::<u32>().ok())?,
                };
                char::from_u32(v)?.to_string()
            }
        };
        out.push_str(&decoded);
        rest = &tail[semi + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Escape `&`, `<`, `>`, `"`, `'` for attribute or text content.
pub fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Parse a document into its event list, or `None` if the document is
/// not well-formed: every `Start` must be closed (or `empty`), there is
/// exactly one root element, names are balanced, no `]]>` in text.
/// Text is delivered exactly as written (whitespace preserved —
/// normalization is the consumer's business).
pub fn parse(d: &str) -> Option<Vec<Ev>> {
    let mut evs = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut i = 0usize;
    let mut seen_root = false;
    let mut done_root = false;
    let b = d.as_bytes();
    while i < d.len() {
        if d[i..].starts_with("<!--") {
            let end = d[i + 4..].find("-->").map(|p| i + 4 + p)?;
            let body = &d[i + 4..end];
            if body.contains("--") || body.ends_with('-') {
                return None; // "--" illegal inside comments
            }
            evs.push(Ev::Comment(body.to_string()));
            i = end + 3;
        } else if d[i..].starts_with("<![CDATA[") {
            let end = d[i + 9..].find("]]>").map(|p| i + 9 + p)?;
            if stack.is_empty() {
                return None; // CDATA must live inside the root element
            }
            evs.push(Ev::CData(d[i + 9..end].to_string()));
            i = end + 3;
        } else if d[i..].starts_with("<?") {
            let end = d[i + 2..].find("?>").map(|p| i + 2 + p)?;
            let body = &d[i + 2..end];
            let mut j = 0usize;
            let target = name(body, &mut j)?.to_string();
            let rest = body[j..].trim_start().to_string();
            if target.eq_ignore_ascii_case("xml") {
                if evs.is_empty() && !seen_root {
                    evs.push(Ev::Decl(rest));
                } else {
                    return None; // 'xml' target reserved for the declaration
                }
            } else {
                evs.push(Ev::Pi { target, body: rest });
            }
            i = end + 2;
        } else if d[i..].starts_with("<!") {
            // DOCTYPE (possibly with an internal subset) — keep verbatim
            if done_root || seen_root || !d[i..].starts_with("<!DOCTYPE") {
                return None;
            }
            // DOCTYPE ends at `>` outside `[…]` internal subset and
            // outside quoted strings.
            let mut j = i + 2;
            let mut depth = 0usize;
            let mut quote = 0u8;
            loop {
                let c = *b.get(j)?;
                if quote != 0 {
                    if c == quote {
                        quote = 0;
                    }
                } else {
                    match c {
                        b'"' | b'\'' => quote = c,
                        b'[' => depth += 1,
                        b']' => depth = depth.saturating_sub(1),
                        b'>' if depth == 0 => break,
                        _ => {}
                    }
                }
                j += 1;
            }
            evs.push(Ev::Doctype(d[i + 9..j].trim().to_string()));
            i = j + 1;
        } else if d[i..].starts_with("</") {
            i += 2;
            let n = name(d, &mut i)?.to_string();
            ws(d, &mut i);
            if *b.get(i)? != b'>' {
                return None;
            }
            i += 1;
            if stack.pop().as_deref() != Some(n.as_str()) {
                return None;
            }
            evs.push(Ev::End { name: n });
            if stack.is_empty() {
                done_root = true;
            }
        } else if d[i..].starts_with('<') {
            i += 1;
            let n = name(d, &mut i)?.to_string();
            let mut attrs: Vec<(String, String)> = Vec::new();
            let empty = loop {
                ws(d, &mut i);
                match *b.get(i)? {
                    b'>' => {
                        i += 1;
                        break false;
                    }
                    b'/' => {
                        i += 1;
                        if *b.get(i)? != b'>' {
                            return None;
                        }
                        i += 1;
                        break true;
                    }
                    _ => {
                        let an = name(d, &mut i)?.to_string();
                        ws(d, &mut i);
                        if *b.get(i)? != b'=' {
                            return None;
                        }
                        i += 1;
                        ws(d, &mut i);
                        let q = *b.get(i)?;
                        if q != b'"' && q != b'\'' {
                            return None;
                        }
                        i += 1;
                        let start = i;
                        while *b.get(i)? != q {
                            if d.as_bytes()[i] == b'<' {
                                return None;
                            }
                            i += 1;
                        }
                        let av = unescape(&d[start..i])?;
                        i += 1;
                        if attrs.iter().any(|(x, _)| *x == an) {
                            return None; // duplicate attribute
                        }
                        attrs.push((an, av));
                    }
                }
            };
            if done_root {
                return None; // second root element
            }
            seen_root = true;
            if !empty {
                stack.push(n.clone());
            } else {
                done_root = stack.is_empty();
            }
            evs.push(Ev::Start {
                name: n,
                attrs,
                empty,
            });
        } else {
            // text run up to '<'
            let start = i;
            while *b.get(i)? != b'<' {
                i += 1;
                if i >= d.len() {
                    break;
                }
            }
            let t = &d[start..i];
            if t.contains("]]>") {
                return None;
            }
            let t = unescape(t)?;
            if stack.is_empty() {
                if !t.trim().is_empty() {
                    return None; // character data outside the root element
                }
            } else {
                evs.push(Ev::Text(t));
            }
        }
    }
    if !stack.is_empty() || !seen_root {
        return None;
    }
    Some(evs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_doc() {
        let e = parse("<?xml version=\"1.0\"?><root a=\"1\" b=\"x &lt; y\"><item/>hi<![CDATA[<raw>]]><!-- c --><?pi body?></root>").unwrap();
        assert_eq!(e[0], Ev::Decl("version=\"1.0\"".to_string()));
        match &e[1] {
            Ev::Start { name, attrs, empty } => {
                assert_eq!(name, "root");
                assert_eq!(attrs[1].1, "x < y");
                assert!(!empty);
            }
            _ => panic!(),
        }
        assert!(matches!(&e[2], Ev::Start { name, empty: true, .. } if name == "item"));
        assert_eq!(e[3], Ev::Text("hi".into()));
        assert_eq!(e[4], Ev::CData("<raw>".into()));
        assert_eq!(e[5], Ev::Comment(" c ".into()));
        assert!(matches!(&e[6], Ev::Pi { target, .. } if target == "pi"));
        assert!(matches!(&e[7], Ev::End { name } if name == "root"));
    }

    #[test]
    fn doctype_and_entities() {
        let e = parse("<!DOCTYPE r [ <!ENTITY x \"y\"> ]><r>&amp;&#65;&#x42;</r>").unwrap();
        assert!(matches!(&e[0], Ev::Doctype(_)));
        assert_eq!(e[2], Ev::Text("&AB".into()));
    }

    #[test]
    fn malformed_rejected() {
        assert!(parse("").is_none());
        assert!(parse("<a>").is_none()); // unclosed
        assert!(parse("<a></b>").is_none()); // mismatched
        assert!(parse("<a/><b/>").is_none()); // two roots
        assert!(parse("<a>&bogus;</a>").is_none()); // unknown entity
        assert!(parse("<a>]]></a>").is_none()); // ']]>' in text
        assert!(parse("<a x=\"1\" x=\"2\"/>").is_none()); // dup attr
        assert!(parse("<a x=1/>").is_none()); // unquoted attr
        assert!(parse("<a><b></a></b>").is_none()); // crossed tags
        assert!(parse("text<a/>").is_none()); // text outside root
        assert!(parse("<a/><!-- -- -->").is_none()); // '--' in comment
        assert!(parse("<a/><?xml v?>").is_none()); // xml PI after decl position
        assert!(parse("<a b=\"x<y\"/>").is_none()); // '<' in attr value
    }

    #[test]
    fn escape_unescape() {
        assert_eq!(escape("<a&\"b\">"), "&lt;a&amp;&quot;b&quot;&gt;");
        assert_eq!(
            unescape("&lt;a&amp;&quot;b&quot;&gt;&#x21;").unwrap(),
            "<a&\"b\">!"
        );
        assert_eq!(unescape("plain").unwrap(), "plain");
        assert!(unescape("&#xD800;").is_none()); // surrogate isn't a char
        assert!(unescape("&").is_none());
    }
}

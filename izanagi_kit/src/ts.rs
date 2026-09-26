//! Qt Linguist `.ts` files: `<TS version language>` over `<context>` /
//! `<message>` / `<source>` / `<translation>` (+ optional `<numerusform>`
//! plurals). Minimal tag scanning; entities `&lt; &gt; &amp; &quot;
//! &apos;` are unescaped in text.
//!
//! ```
//! use izanagi_kit::ts::parse;
//!
//! let d = b"<TS version=\"2.1\" language=\"en\"><context><name>C</name>\
//! <message><source>Hi</source><translation type=\"unfinished\"/></message></context></TS>";
//! let t = parse(d).unwrap();
//! assert_eq!(t.contexts[0].messages[0].source, "Hi");
//! assert!(t.contexts[0].messages[0].unfinished);
//! ```

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Attribute value for `key="v"` (or `'v'`); `None` if absent.
fn attr(tag: &str, key: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(i) = rest.find(key) {
        // `key` must start an attribute name, not end a longer one
        if i > 0
            && (rest.as_bytes()[i - 1].is_ascii_alphanumeric() || rest.as_bytes()[i - 1] == b'-')
        {
            rest = &rest[i + key.len()..];
            continue;
        }
        let after = &rest[i + key.len()..];
        let after = after.trim_start();
        if let Some(v) = after.strip_prefix('=') {
            let v = v.trim_start();
            let q = v.as_bytes().first()?;
            if *q != b'"' && *q != b'\'' {
                return None;
            }
            let end = v[1..].find(*q as char)?;
            return Some(v[1..1 + end].to_string());
        }
        rest = &rest[key.len()..];
    }
    None
}

/// Text inside `<tag ..>...</tag>` (self-closing yields "").
fn element(d: &str, tag: &str, from: usize) -> Option<(String, String, usize)> {
    let open = d[from..].find(&['<'][..])? + from;
    let gt = d[open..].find('>')? + open;
    if !d[open + 1..].starts_with(tag) {
        return None;
    }
    let head = &d[open + 1..gt];
    if head.ends_with('/') {
        return Some((head.to_string(), String::new(), gt + 1));
    }
    let close = d[gt..].find(&["</", tag, ">"].concat())? + gt;
    Some((
        head.to_string(),
        unescape(&d[gt + 1..close]),
        close + tag.len() + 3,
    ))
}

/// One `<message>` record.
#[derive(Debug, Clone)]
pub struct Message {
    /// `<source>` text.
    pub source: String,
    /// `<translation>` text ("" when absent/self-closed).
    pub translation: Option<String>,
    /// `type="unfinished"` on the translation.
    pub unfinished: bool,
    /// `<numerusform>` plurals.
    pub plurals: Vec<String>,
}

/// One `<context>` group.
#[derive(Debug, Clone)]
pub struct Context {
    /// `<name>` of the context.
    pub name: String,
    /// Messages in order.
    pub messages: Vec<Message>,
}

/// Root `<TS>`.
#[derive(Debug, Clone)]
pub struct Ts {
    /// `version` attribute.
    pub version: String,
    /// `language` attribute ("" when absent).
    pub language: String,
    /// Contexts in order.
    pub contexts: Vec<Context>,
}

fn parse_message(d: &str, mut i: usize, end: usize) -> Option<(Message, usize)> {
    let mut source = None;
    let mut translation = None;
    let mut unfinished = false;
    let mut plurals = Vec::new();
    while i < end {
        let open = d[i..end].find('<')? + i;
        if d[open..].starts_with("</message") {
            i = open + 9;
            break;
        }
        match element(d, "source", open)
            .or_else(|| element(d, "translation", open))
            .or_else(|| element(d, "numerusform", open))
        {
            Some((head, text, j)) => {
                if head.starts_with("source") {
                    source = Some(text);
                } else if head.starts_with("translation") {
                    unfinished = attr(&head, "type").as_deref() == Some("unfinished");
                    translation = Some(text);
                } else {
                    plurals.push(text);
                }
                i = j;
            }
            // <location>, <comment>, ... — skip the tag
            None => i = d[open..end].find('>')? + open + 1,
        }
    }
    Some((
        Message {
            source: source?,
            translation,
            unfinished,
            plurals,
        },
        i,
    ))
}

/// Parse a `.ts` document. `None` when `<TS>` is absent.
pub fn parse(d: &[u8]) -> Option<Ts> {
    let d = std::str::from_utf8(d).ok()?;
    let open = d.find("<TS")?;
    let gt = d[open..].find('>')? + open;
    let head = &d[open..gt];
    let version = attr(head, "version").unwrap_or_default();
    let language = attr(head, "language").unwrap_or_default();
    let mut contexts = Vec::new();
    let mut i = gt + 1;
    while let Some(rel) = d[i..].find("<context") {
        let mut j = i + rel + 8;
        let close_tag = "</context>";
        let end = d[j..].find(close_tag)? + j;
        let (h, name, nj) = element(d, "name", j)?;
        if !h.starts_with("name") {
            return None;
        }
        j = nj;
        let mut messages = Vec::new();
        while j < end {
            let Some(m) = d[j..end].find("<message") else {
                break;
            };
            let (msg, nj) = parse_message(d, j + m + 8, end)?;
            messages.push(msg);
            j = nj;
        }
        contexts.push(Context { name, messages });
        i = end + close_tag.len();
    }
    Some(Ts {
        version,
        language,
        contexts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"<TS version=\"2.0\"><context><name>Main</name>\
        <message><source>A &lt; B</source><translation>a b</translation></message>\
        <message><source>one</source><numerusform>one</numerusform><numerusform>many</numerusform></message>\
        </context></TS>";
        let t = parse(d).unwrap();
        assert_eq!(t.version, "2.0");
        assert_eq!(t.contexts[0].name, "Main");
        assert_eq!(t.contexts[0].messages[0].source, "A < B");
        assert_eq!(
            t.contexts[0].messages[0].translation.as_deref(),
            Some("a b")
        );
        assert_eq!(t.contexts[0].messages[1].plurals.len(), 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"<notts/>").is_none());
    }
}

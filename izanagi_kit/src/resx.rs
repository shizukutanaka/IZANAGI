//! .NET `.resx` resource files: `<root>` over `<data name="k"
//! xml:space="preserve"><value>v</value><comment>c</comment></data>`
//! entries (plus `<metadata>` companions).
//!
//! ```
//! use izanagi_kit::resx::parse;
//!
//! let d = b"<root><data name=\"ok\" xml:space=\"preserve\"><value>OK</value>\
//! <comment>button</comment></data></root>";
//! let r = parse(d).unwrap();
//! assert_eq!(r.entries[0].value.as_deref(), Some("OK"));
//! ```

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(i) = rest.find(key) {
        if i > 0
            && (rest.as_bytes()[i - 1].is_ascii_alphanumeric() || rest.as_bytes()[i - 1] == b'-')
        {
            rest = &rest[i + key.len()..];
            continue;
        }
        let after = rest[i + key.len()..].trim_start();
        if let Some(v) = after.strip_prefix('=') {
            let v = v.trim_start();
            let q = *v.as_bytes().first()?;
            if q != b'"' && q != b'\'' {
                return None;
            }
            let end = v[1..].find(q as char)?;
            return Some(v[1..1 + end].to_string());
        }
        rest = &rest[key.len()..];
    }
    None
}

/// Text inside `<tag>` — `Ok(None)` for a self-closing element.
fn text_of(d: &str, tag: &str, from: usize, end: usize) -> Option<(Option<String>, usize)> {
    let open = d[from..end].find(&["<", tag].concat())? + from;
    let gt = d[open..end].find('>')? + open;
    if d[open..gt].ends_with('/') {
        return Some((None, gt + 1));
    }
    let close = d[gt..end].find(&["</", tag, ">"].concat())? + gt;
    Some((Some(unescape(&d[gt + 1..close])), close + tag.len() + 3))
}

/// One `<data>` entry.
#[derive(Debug, Clone)]
pub struct Entry {
    /// `name` attribute.
    pub name: String,
    /// `type` / `mimetype` attributes ("" when absent).
    pub mime: String,
    /// `<value>` text (`None` when the element is absent/self-closed).
    pub value: Option<String>,
    /// `<comment>` text if present.
    pub comment: Option<String>,
}

/// Parsed resx document.
#[derive(Debug, Clone)]
pub struct Resx {
    /// Entries in file order.
    pub entries: Vec<Entry>,
}

/// Parse a `.resx` document. `None` when `<root>` is absent.
pub fn parse(d: &[u8]) -> Option<Resx> {
    let d = std::str::from_utf8(d).ok()?;
    if !d.contains("<root") {
        return None;
    }
    let mut entries = Vec::new();
    let mut i = 0usize;
    while let Some(rel) = d[i..].find("<data") {
        let open = i + rel;
        let gt = d[open..].find('>')? + open;
        let head = &d[open..gt];
        let dend = d[gt..].find("</data>")? + gt;
        let value = text_of(d, "value", gt + 1, dend)
            .map(|r| r.0)
            .unwrap_or(None);
        let comment = text_of(d, "comment", gt + 1, dend)
            .map(|r| r.0)
            .unwrap_or(None);
        entries.push(Entry {
            name: attr(head, "name").unwrap_or_default(),
            mime: attr(head, "mimetype")
                .or_else(|| attr(head, "type"))
                .unwrap_or_default(),
            value,
            comment,
        });
        i = dend + 7;
    }
    Some(Resx { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"<root><resheader name=\"version\"><value>2</value></resheader>\
        <data name=\"a\"><value>1</value></data>\
        <data name=\"b\" mimetype=\"x\"><value/></data></root>";
        let r = parse(d).unwrap();
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.entries[0].value.as_deref(), Some("1"));
        assert_eq!(r.entries[1].mime, "x");
        assert!(r.entries[1].value.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"<noroot/>").is_none());
    }
}

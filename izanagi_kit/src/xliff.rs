//! XLIFF 1.2 localisation interchange: `<xliff version>` over
//! `<file source-language target-language>` with `<trans-unit id>`
//! containing `<source>`/`<target>`/`<note>`.
//!
//! ```
//! use izanagi_kit::xliff::parse;
//!
//! let d = b"<xliff version=\"1.2\"><file source-language=\"en\" datatype=\"plaintext\">\
//! <body><trans-unit id=\"greet\"><source>Hi</source><target>Salut</target></trans-unit>\
//! </body></file></xliff>";
//! let x = parse(d).unwrap();
//! assert_eq!(x.files[0].units[0].target.as_deref(), Some("Salut"));
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

/// Text between `<tag>` and `</tag>` starting the search at `from`.
fn text_of(d: &str, tag: &str, from: usize, end: usize) -> Option<(Option<String>, usize)> {
    let open = d[from..end].find(&["<", tag].concat())? + from;
    let gt = d[open..end].find('>')? + open;
    if d[gt - 1..gt].starts_with('/') || d[open..gt].ends_with('/') {
        return Some((None, gt + 1));
    }
    let close = d[gt..end].find(&["</", tag, ">"].concat())? + gt;
    Some((Some(unescape(&d[gt + 1..close])), close + tag.len() + 3))
}

/// One `<trans-unit>`.
#[derive(Debug, Clone)]
pub struct Unit {
    /// `id` attribute.
    pub id: String,
    /// `<source>` text.
    pub source: String,
    /// `<target>` text if present.
    pub target: Option<String>,
    /// `<note>` text if present.
    pub note: Option<String>,
}

/// One `<file>`.
#[derive(Debug, Clone)]
pub struct File {
    /// `source-language` attribute ("" when absent).
    pub source_language: String,
    /// `target-language` attribute ("" when absent).
    pub target_language: String,
    /// `datatype` attribute ("" when absent).
    pub datatype: String,
    /// Translation units in order.
    pub units: Vec<Unit>,
}

/// Root `<xliff>` document.
#[derive(Debug, Clone)]
pub struct Xliff {
    /// `version` attribute.
    pub version: String,
    /// Files in order.
    pub files: Vec<File>,
}

/// Parse an XLIFF document. `None` when `<xliff` is absent.
pub fn parse(d: &[u8]) -> Option<Xliff> {
    let d = std::str::from_utf8(d).ok()?;
    let open = d.find("<xliff")?;
    let gt = d[open..].find('>')? + open;
    let version = attr(&d[open..gt], "version").unwrap_or_default();
    let mut files = Vec::new();
    let mut i = gt + 1;
    while let Some(rel) = d[i..].find("<file") {
        let fhead_end = d[i + rel..].find('>')? + i + rel;
        let fhead = &d[i + rel..fhead_end];
        let fend = d[fhead_end..].find("</file>")? + fhead_end;
        let mut units = Vec::new();
        let mut j = fhead_end + 1;
        while let Some(urel) = d[j..fend].find("<trans-unit") {
            let uo = j + urel;
            let ugt = d[uo..fend].find('>')? + uo;
            let id = attr(&d[uo..ugt], "id").unwrap_or_default();
            let uend = d[ugt..fend].find("</trans-unit>")? + ugt;
            let (source, _) = text_of(d, "source", ugt + 1, uend)?;
            let target = text_of(d, "target", ugt + 1, uend)
                .map(|r| r.0)
                .unwrap_or(None);
            let note = text_of(d, "note", ugt + 1, uend)
                .map(|r| r.0)
                .unwrap_or(None);
            units.push(Unit {
                id,
                source: source?,
                target,
                note,
            });
            j = uend + 13;
        }
        files.push(File {
            source_language: attr(fhead, "source-language").unwrap_or_default(),
            target_language: attr(fhead, "target-language").unwrap_or_default(),
            datatype: attr(fhead, "datatype").unwrap_or_default(),
            units,
        });
        i = fend + 7;
    }
    Some(Xliff { version, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"<xliff version=\"1.2\"><file source-language=\"en\" target-language=\"fr\">\
        <body><trans-unit id=\"a\"><source>A</source><target>B</target><note>n</note></trans-unit>\
        <trans-unit id=\"b\"><source>only</source></trans-unit></body></file></xliff>";
        let x = parse(d).unwrap();
        assert_eq!(x.version, "1.2");
        assert_eq!(x.files[0].target_language, "fr");
        assert_eq!(x.files[0].units[0].note.as_deref(), Some("n"));
        assert!(x.files[0].units[1].target.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"<notxliff/>").is_none());
    }
}

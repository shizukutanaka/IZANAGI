//! RFC 1524 mailcap file parsing.
//!
//! Each line maps a MIME type to a view command:
//! `type/subtype; command; flag; name=value`. A bare `*` subtype (or a
//! missing subtype) is a wildcard entry. `#` comments and `\` line
//! continuations are supported; fields are `;`-separated with the first
//! two being the type and the command.
//!
//! ```
//! use izanagi_kit::mailcap;
//! let m = mailcap::parse(b"text/html; lynx %s; needsterminal\ntext/*; less %s\n").unwrap();
//! assert_eq!(m.len(), 2);
//! assert!(m[1].subtype_wildcard);
//! ```

use std::vec::Vec;

/// One mailcap line.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Full media type verbatim (`b"text/html"`, `b"text/*"`, `b"audio"`).
    pub mime_type: Vec<u8>,
    /// True when the subtype is `*` or absent.
    pub subtype_wildcard: bool,
    /// The view command (second `;` field).
    pub view: Vec<u8>,
    /// Bare flag words (`needsterminal`, `copiousoutput`, `x-new-bitmap`, ...).
    pub flags: Vec<Vec<u8>>,
    /// `name=value` fields (`test=...`, `description=...`, ...).
    pub fields: Vec<(Vec<u8>, Vec<u8>)>,
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && (s[a] == b' ' || s[a] == b'\t') {
        a += 1;
    }
    while b > a && (s[b - 1] == b' ' || s[b - 1] == b'\t') {
        b -= 1;
    }
    &s[a..b]
}

/// Parses a whole mailcap file; comment and malformed lines are
/// skipped. `None` when a non-empty input has no non-comment content
/// line — an all-comment or empty file yields `Some(vec![])`.
pub fn parse(d: &[u8]) -> Option<Vec<Entry>> {
    // Join `\`-continued physical lines into logical lines first.
    let mut logical: Vec<u8> = Vec::with_capacity(d.len());
    let mut i = 0;
    while i < d.len() {
        let b = d[i];
        if b == b'\\' && i + 1 < d.len() && d[i + 1] == b'\n' {
            i += 2;
            continue;
        }
        logical.push(b);
        i += 1;
    }
    let mut out = Vec::new();
    let mut content_seen = false;
    for raw in logical.split(|&b| b == b'\n') {
        let line = trim(raw);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        content_seen = true;
        let mut it = line.split(|&b| b == b';');
        let mt = match it.next() {
            Some(t) => trim(t),
            None => continue,
        };
        if mt.is_empty() {
            continue;
        }
        let view = match it.next() {
            Some(v) => trim(v).to_vec(),
            None => continue,
        };
        let wildcard = match mt.iter().position(|&b| b == b'/') {
            Some(p) => trim(&mt[p + 1..]) == b"*",
            None => true, // bare "type" means any subtype
        };
        let mut flags = Vec::new();
        let mut fields = Vec::new();
        for f in it {
            let f = trim(f);
            if f.is_empty() {
                continue;
            }
            match f.iter().position(|&b| b == b'=') {
                Some(e) => fields.push((trim(&f[..e]).to_vec(), trim(&f[e + 1..]).to_vec())),
                None => flags.push(f.to_vec()),
            }
        }
        out.push(Entry {
            mime_type: mt.to_vec(),
            subtype_wildcard: wildcard,
            view,
            flags,
            fields,
        });
    }
    if out.is_empty() && content_seen {
        None
    } else {
        Some(out)
    }
}

/// First entry matching `mime_type` (e.g. `b"text/html"`); wildcard
/// entries match any subtype of their type.
pub fn lookup<'a>(entries: &'a [Entry], mime_type: &[u8]) -> Option<&'a Entry> {
    let ty = mime_type
        .iter()
        .position(|&b| b == b'/')
        .map(|p| &mime_type[..p])
        .unwrap_or(mime_type);
    entries.iter().find(|e| {
        if e.mime_type == mime_type {
            return true;
        }
        if !e.subtype_wildcard {
            return false;
        }
        match e.mime_type.iter().position(|&b| b == b'/') {
            Some(p) => &e.mime_type[..p] == ty,
            None => e.mime_type == ty,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_flags() {
        let e =
            parse(b"image/png; display %s; test=test -n %f; needsterminal; description=PNG img\n")
                .unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].flags, vec![b"needsterminal".to_vec()]);
        assert_eq!(e[0].fields.len(), 2);
        assert_eq!(e[0].fields[0].0, b"test".to_vec());
        assert!(!e[0].subtype_wildcard);
    }

    #[test]
    fn comments_blank_and_continuation() {
        let m = parse(b"# c\n\ntext/plain; less \\\n %s\n").unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].view, b"less  %s".to_vec());
    }

    #[test]
    fn wildcard_lookup() {
        let m = parse(b"text/*; less %s\nimage/png; display %s\n").unwrap();
        assert_eq!(
            lookup(&m, b"text/html").unwrap().mime_type,
            b"text/*".to_vec()
        );
        assert_eq!(
            lookup(&m, b"image/png").unwrap().view,
            b"display %s".to_vec()
        );
        assert!(lookup(&m, b"audio/ogg").is_none());
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a mailcap file\n").is_none());
        // Empty and comment-only inputs are still valid files.
        assert_eq!(parse(b""), Some(Vec::new()));
        assert_eq!(parse(b"# just a comment\n"), Some(Vec::new()));
    }
}

//! LDIF (RFC 2849) — `dn:`-separated LDAP entry blocks of
//! `attr: value` / `attr:: base64` / `attr:< url` lines with
//! `changetype:` records and `version:`/`#` comment lines.
//!
//! ```
//! let d = b"version: 1\ndn: cn=admin,dc=ex,dc=com\ncn: admin\nobjectClass: top\n";
//! let l = izanagi_kit::ldif::parse(d).unwrap();
//! assert_eq!(l.entries, 1);
//! assert_eq!(l.attrs, 2);
//! assert!(izanagi_kit::ldif::detect(d));
//! ```

/// Census of an LDIF stream.
#[derive(Debug, Clone)]
pub struct Ldif {
    /// `dn:` entry starts.
    pub entries: usize,
    /// `version:` header present.
    pub version: bool,
    /// Plain `attr: value` lines.
    pub attrs: usize,
    /// `attr:: base64` lines.
    pub base64: usize,
    /// `attr:< url` reference lines.
    pub urls: usize,
    /// `changetype:` records (add/modify/delete/moddn).
    pub changes: usize,
    /// `add:`/`replace:`/`delete:`/`modify:` operation lines.
    pub ops: usize,
    /// `-` change separators.
    pub separators: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// Folded continuation lines.
    pub folded: usize,
    /// Distinct attribute names.
    pub attr_names: usize,
}

const CHANGE_TYPES: &[&str] = &["add", "modify", "delete", "moddn", "modrdn"];

/// Detects LDIF: a `dn:` line plus at least one `attr:` line.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    t.lines()
        .any(|l| l.starts_with("dn:") || l.starts_with("dn::"))
        && t.lines()
            .any(|l| l.contains(": ") && l.split(':').next().is_some_and(|h| !h.is_empty()))
}

/// Parses an LDIF stream; `None` when no `dn:` record exists.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ldif> {
    if !detect(b) {
        return None;
    }
    let t = String::from_utf8_lossy(b);
    let mut l = Ldif {
        entries: 0,
        version: false,
        attrs: 0,
        base64: 0,
        urls: 0,
        changes: 0,
        ops: 0,
        separators: 0,
        comments: 0,
        folded: 0,
        attr_names: 0,
    };
    let mut names = Vec::<String>::new();
    for line in t.lines() {
        if line.starts_with(' ') {
            l.folded += 1;
            continue;
        }
        if line.starts_with('#') {
            l.comments += 1;
            continue;
        }
        if line == "-" {
            l.separators += 1;
            continue;
        }
        if let Some(v) = line.strip_prefix("version:") {
            let _ = v;
            l.version = true;
            continue;
        }
        if line.starts_with("dn:") || line.starts_with("dn::") {
            l.entries += 1;
            continue;
        }
        if let Some(ct) = line.strip_prefix("changetype:") {
            let _ = ct.trim();
            l.changes += 1;
            continue;
        }
        let head = line.split(':').next().unwrap_or("");
        if CHANGE_TYPES.contains(&head) && line.contains(": ") {
            l.ops += 1;
            continue;
        }
        if line.contains("::") {
            l.base64 += 1;
        } else if line.contains(":<") {
            l.urls += 1;
        } else if line.contains(": ") {
            l.attrs += 1;
        } else {
            continue;
        }
        let name = head.split(';').next().unwrap_or(head);
        if !names.iter().any(|n| n == name) {
            names.push(name.to_string());
            l.attr_names = names.len();
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"version: 1\n\ndn: cn=admin,dc=ex,dc=com\ncn: admin\nobjectClass: top\nobjectClass: person\n\ndn: cn=j,dc=ex,dc=com\nchangetype: modify\nadd: mail\nmail: j@x.y\n-\n";

    #[test]
    fn parses() {
        let l = parse(D).unwrap();
        assert_eq!(l.entries, 2);
        assert!(l.version);
        assert_eq!(l.changes, 1);
        assert_eq!(l.ops, 1);
        assert_eq!(l.separators, 1);
        assert!(l.attr_names >= 3);
    }

    #[test]
    fn encodings() {
        let d = b"dn: cn=x\njpegPhoto:: aGk=\nref:< file:///x\n# note\n";
        let l = parse(d).unwrap();
        assert_eq!(l.base64, 1);
        assert_eq!(l.urls, 1);
        assert_eq!(l.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"no dn here"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
    }
}

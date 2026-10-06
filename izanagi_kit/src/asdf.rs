//! ASDF (Advanced Scientific Data Format, `*.asdf`) census.
//!
//! `#ASDF`/`#ASDF_STANDARD` headers, a `%YAML` document between `---` and
//! `...`, then optional binary `BLK` blocks. Counts tree keys, datatype/
//! shape/source entries, comments and anchors.
//!
//! ```
//! let s = b"#ASDF 1\n#ASDF_STANDARD 1\n%YAML 1\n--- !core/asdf\ndata:\n  datatype: int32\n  shape: [4]\n  source: 0\nwcs: {}\n...\nBLK\n";
//! assert!(izanagi_kit::asdf::detect(s));
//! let a = izanagi_kit::asdf::Asdf::parse(s).unwrap();
//! assert_eq!(a.version, 1);
//! assert_eq!(a.tree_keys, 2);
//! assert_eq!(a.blocks, 1);
//! assert_eq!(a.datatypes, 1);
//! assert_eq!(a.shapes, 1);
//! ```

/// Parsed census of an ASDF file.
#[derive(Debug, Clone)]
pub struct Asdf {
    /// `#ASDF` version digits, or 0.
    pub version: usize,
    /// `#ASDF_STANDARD` version digits, or 0.
    pub standard: usize,
    /// Indent-0 `key:` lines inside the YAML tree.
    pub tree_keys: usize,
    /// `BLK` binary block markers.
    pub blocks: usize,
    /// `datatype:` entries.
    pub datatypes: usize,
    /// `shape:` entries.
    pub shapes: usize,
    /// `source:` entries.
    pub sources: usize,
    /// `%` directives.
    pub directives: usize,
    /// `---`/`...` document markers.
    pub markers: usize,
    /// `!` tag tokens.
    pub tags: usize,
    /// `&name` anchors.
    pub anchors: usize,
    /// `*name` aliases.
    pub aliases: usize,
    /// Comment lines (`#` not `#ASDF`).
    pub comments: usize,
    /// `http://`/`https://` references.
    pub uris: usize,
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

fn is_key(s: &str, key: &str) -> bool {
    // `key :` (コロン前の空白)も YAML では合法。
    s.strip_prefix(key)
        .is_some_and(|r| r.trim_start().starts_with(':'))
}

/// Reports whether `b` looks like an ASDF file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("#ASDF") && (t.contains("%YAML") || t.contains("---"))
}

impl Asdf {
    /// Parses `b` as an ASDF file, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut a = Asdf {
            version: 0,
            standard: 0,
            tree_keys: 0,
            blocks: 0,
            datatypes: 0,
            shapes: 0,
            sources: 0,
            directives: 0,
            markers: 0,
            tags: 0,
            anchors: 0,
            aliases: 0,
            comments: 0,
            uris: 0,
        };
        let mut in_tree = false;
        let mut tree_done = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("#ASDF_STANDARD") {
                a.standard = tr
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0);
                continue;
            }
            if tr.starts_with("#ASDF") {
                a.version = tr
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0);
                continue;
            }
            if tr.starts_with('#') {
                a.comments += 1;
                continue;
            }
            if tr.starts_with('%') {
                a.directives += 1;
                continue;
            }
            if tr.starts_with("---") {
                in_tree = true;
                a.markers += 1;
                continue;
            }
            if tr == "..." {
                in_tree = false;
                tree_done = true;
                a.markers += 1;
                continue;
            }
            if tr.starts_with("BLK") {
                a.blocks += 1;
                continue;
            }
            if in_tree && !tree_done {
                if indent(l) == 0 && tr.contains(':') {
                    a.tree_keys += 1;
                }
                if is_key(tr, "datatype") {
                    a.datatypes += 1;
                }
                if is_key(tr, "shape") {
                    a.shapes += 1;
                }
                if is_key(tr, "source") {
                    a.sources += 1;
                }
                if tr.contains('!') {
                    a.tags += 1;
                }
                for c in tr.split_whitespace() {
                    if let Some(v) = c.strip_prefix('&') {
                        if !v.is_empty() {
                            a.anchors += 1;
                        }
                    } else if let Some(v) = c.strip_prefix('*') {
                        if !v.is_empty() {
                            a.aliases += 1;
                        }
                    }
                }
            }
        }
        a.uris = t.matches("http://").count() + t.matches("https://").count();
        Some(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_space_before_colon() {
        // YAML tree では `key :` も合法。
        let v = String::from_utf8_lossy(S).replace("datatype:", "datatype :");
        let a = Asdf::parse(v.as_bytes()).unwrap();
        assert_eq!(a.datatypes, 1);
    }

    const S: &[u8] = b"#ASDF 1\n#ASDF_STANDARD 1\n%YAML 1\n--- !core/asdf\ndata:\n  datatype: int32\n  shape: [4]\n  source: 0\nwcs: {}\n...\nBLK\n";

    #[test]
    fn parses_asdf() {
        assert!(detect(S));
        let a = Asdf::parse(S).unwrap();
        assert_eq!(a.version, 1);
        assert_eq!(a.tree_keys, 2);
        assert_eq!(a.blocks, 1);
        assert_eq!(a.datatypes, 1);
        assert_eq!(a.shapes, 1);
    }

    #[test]
    fn rejects_non_asdf() {
        assert!(!detect(b"a: 1"));
        assert!(Asdf::parse(b"{}").is_none());
    }
}

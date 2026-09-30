//! Philips PAR/REC `.PAR` header (MRI/PET reconstruction parameter
//! file): free text with `#` comment lines, `# === SECTION ===`
//! section headers, and `.   KEY  :  VALUE` parameter lines (the key
//! begins after a leading `.`); the image-info table rows are lines
//! beginning with digits/`.` columns — exposed only as `params` and
//! `sections` plus the image row count.
//!
//! ```
//! let d = b"# === DATA DESCRIPTION FILE ===\n.   Patient name  :   S\n.   Study name :   T\n# === IMAGE INFORMATION ===\n";
//! let p = izanagi_kit::parrec::parse(d).unwrap();
//! assert_eq!(p.get("patient name"), Some("S"));
//! assert!(p.sections.iter().any(|s| s.contains("IMAGE")));
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed `.PAR` header.
#[derive(Clone, Debug)]
pub struct ParRec {
    /// `=== SECTION ===` header texts (without `# ===`/ `===`).
    pub sections: Vec<String>,
    /// Parameter `key : value` pairs in order.
    pub params: Vec<(String, String)>,
    /// Number of numeric column rows (the image information table).
    pub image_rows: usize,
}

impl ParRec {
    /// Look up a parameter by name (case-insensitive, last wins).
    pub fn get(&self, key: &str) -> Option<&str> {
        let want = key.to_lowercase();
        self.params
            .iter()
            .rev()
            .find(|(k, _)| k.to_lowercase() == want)
            .map(|(_, v)| v.as_str())
    }
}

fn is_section(t: &str) -> Option<String> {
    let inner = t.strip_prefix('#')?.trim();
    let inner = inner.strip_prefix('=')?;
    let (name, _) = inner.rsplit_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Parse a `.PAR` header; `None` without at least one `# ===` section
/// and one `.` parameter line.
pub fn parse(d: &[u8]) -> Option<ParRec> {
    let s = std::str::from_utf8(d).ok()?;
    let mut sections = Vec::new();
    let mut params = Vec::new();
    let mut image_rows = 0usize;
    for raw in s.lines() {
        let line = raw.trim_end();
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(name) = is_section(t) {
            sections.push(name);
            continue;
        }
        if let Some(rest) = t.strip_prefix('.') {
            if let Some((k, v)) = rest.split_once(':') {
                let k = k.trim();
                if !k.is_empty() {
                    params.push((k.to_string(), v.trim().to_string()));
                    continue;
                }
            }
            continue;
        }
        if t.starts_with('#') {
            continue;
        }
        // A row of the numeric image-information table starts with a
        // digit or `*`/`X` marker.
        let c = t.as_bytes()[0];
        if c.is_ascii_digit() || c == b'*' {
            image_rows += 1;
        }
    }
    if sections.is_empty() || params.is_empty() {
        return None;
    }
    Some(ParRec {
        sections,
        params,
        image_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAR: &[u8] = b"# === DATA DESCRIPTION FILE ===\n.   Patient name  :   Smith\n.   Study name :   CARDIAC\n# === GENERAL INFORMATION ===\n.   Reconstruction resolution :   256 256\n# === IMAGE INFORMATION DEFINITION ===\n# === IMAGE INFORMATION ===\n    1  256 256 90\n    2  256 256 95\n";

    #[test]
    fn basic() {
        let p = parse(PAR).unwrap();
        assert_eq!(p.sections.len(), 4);
        assert_eq!(p.get("patient name"), Some("Smith"));
        assert_eq!(p.get("reconstruction resolution"), Some("256 256"));
        assert_eq!(p.image_rows, 2);
        assert_eq!(p.get("missing"), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"# === S ===\n").is_none()); // no params
        assert!(parse(b". a : 1\n").is_none()); // no section
        assert!(parse(b"plain text").is_none());
    }
}

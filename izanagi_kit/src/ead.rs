//! EAD (Encoded Archival Description): `<ead>` or namespaced root
//! carrying `<eadheader>` + `<archdesc>` + hierarchical component
//! units `<c>`/`<c01>`-`<c12>` + `<did>`/`<unittitle>`/`unitid`.
//!
//! ```
//! use izanagi_kit::ead::{detect, parse};
//!
//! let d = b"<ead><eadheader><eadid>x</eadid></eadheader>\
//! <archdesc><did><unittitle>C</unittitle></did>\
//! <c01><did><unittitle>S</unittitle><unitid>1</unitid></did></c01>\
//! <c02/></archdesc></ead>";
//! assert!(detect(d));
//! let e = parse(d).unwrap();
//! assert_eq!(e.components, 2);
//! assert_eq!(e.unitids, 1);
//! ```

fn elem_count(s: &str, name: &str) -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    while let Some(off) = s[i..].find(&format!("<{name}")) {
        let p = i + off + name.len() + 1;
        match s[p..].chars().next() {
            None | Some(' ' | '>' | '/' | '\t' | '\n') => n += 1,
            _ => {}
        }
        i = p;
        if i >= s.len() {
            break;
        }
    }
    n
}

/// Parsed EAD census.
#[derive(Debug, Clone, PartialEq)]
pub struct Ead {
    /// `<eadheader>` block present.
    pub has_eadheader: bool,
    /// `eadid` value (collection identifier).
    pub eadid: Option<String>,
    /// `<archdesc>` sections.
    pub archdescs: u32,
    /// Component `<c>`/`<cNN>` elements total.
    pub components: u32,
    /// `<did>` descriptive blocks.
    pub dids: u32,
    /// `<unittitle>` elements.
    pub unittitles: u32,
    /// `<unitid>` identifiers.
    pub unitids: u32,
    /// `<controlaccess>` terms.
    pub control_access: u32,
    /// `level=` attribute values seen (`fonds`/`series`/`file`/`item`…).
    pub levels: Vec<String>,
}

fn tag_val(s: &str, tag: &str) -> Option<String> {
    let i = s.find(&format!("<{tag}>"))? + tag.len() + 2;
    let j = s[i..].find('<')? + i;
    Some(s[i..j].to_string())
}

/// `true` on an `<ead` root containing `eadheader`/`archdesc`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    (s.contains("<ead") || s.contains(":ead>"))
        && (s.contains("eadheader") || s.contains("archdesc"))
}

/// Census; `None` without an EAD root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ead> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    // `<c`, `<c01>`..`<c12>` component units — digit or boundary after `c`
    let mut components = 0u32;
    for (i, _) in s.match_indices('<') {
        if let Some(r) = s[i + 1..].strip_prefix('c') {
            if r.chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || c == ' ' || c == '>' || c == '/')
            {
                components += 1;
            }
        }
    }
    let mut levels = Vec::new();
    for (m, _) in s.match_indices("level=\"") {
        let start = m + 7;
        if let Some(end) = s[start..].find('"') {
            levels.push(s[start..start + end].to_string());
        }
    }
    Some(Ead {
        has_eadheader: s.contains("<eadheader"),
        eadid: tag_val(s, "eadid"),
        archdescs: elem_count(s, "archdesc"),
        components,
        dids: elem_count(s, "did"),
        unittitles: elem_count(s, "unittitle"),
        unitids: elem_count(s, "unitid"),
        control_access: elem_count(s, "controlaccess"),
        levels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<ead><eadheader><eadid>x</eadid></eadheader>\
<archdesc level=\"fonds\"><did><unittitle>C</unittitle></did>\
<c01 level=\"series\"><did><unittitle>S</unittitle><unitid>1</unitid></did></c01>\
<c02 level=\"item\"/></archdesc><controlaccess/></ead>";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<ead>empty</ead>"));
    }

    #[test]
    fn parses() {
        let e = parse(D).unwrap();
        assert!(e.has_eadheader);
        assert_eq!(e.eadid.as_deref(), Some("x"));
        assert_eq!(e.archdescs, 1);
        assert_eq!(e.components, 2);
        assert_eq!(e.dids, 2);
        assert_eq!(e.unittitles, 2);
        assert_eq!(e.unitids, 1);
        assert_eq!(e.control_access, 1);
        assert_eq!(e.levels, vec!["fonds", "series", "item"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

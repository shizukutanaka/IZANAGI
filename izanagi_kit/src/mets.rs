//! METS (Metadata Encoding & Transmission Standard, Library of
//! Congress): `<mets:mets` or `<mets` root with the `…/METS/` xmlns,
//! then `metsHdr` / `dmdSec` / `amdSec` / `fileSec` / `structMap` /
//! `structLink` sections and `file`/`FLocat` members.
//!
//! ```
//! use izanagi_kit::mets::{detect, parse};
//!
//! let d = b"<mets:mets xmlns:mets=\"x/METS/\"><mets:metsHdr/>\
//! <mets:dmdSec/><mets:fileSec><mets:file><mets:FLocat/></mets:file>\
//! </mets:fileSec><mets:structMap/><mets:structLink/></mets:mets>";
//! assert!(detect(d));
//! let m = parse(d).unwrap();
//! assert_eq!(m.dmd_secs, 1);
//! assert_eq!(m.files, 1);
//! assert_eq!(m.flocats, 1);
//! ```

fn elem_count(s: &str, name: &str) -> u32 {
    // matches `<name`, `<mets:name` / `m:name`, at a `<` boundary
    let mut n = 0u32;
    let mut i = 0usize;
    let bytes = s.as_bytes();
    while let Some(off) = s[i..].find('<') {
        let p = i + off + 1;
        let rest = &s[p..];
        let hit = rest.starts_with(name) || {
            rest.split_once(':').is_some_and(|(pre, post)| {
                !pre.contains('<')
                    && !pre.contains(' ')
                    && !pre.contains('/')
                    && pre.len() < 8
                    && post.starts_with(name)
            })
        };
        if hit {
            // skip the matched tag and require a boundary after the name
            let after = rest.find(name).map(|j| &rest[j + name.len()..]);
            if after.is_some_and(|a| {
                a.is_empty()
                    || a.chars()
                        .next()
                        .is_some_and(|c| c == ' ' || c == '>' || c == '/' || c == '\t' || c == '\n')
            }) {
                n += 1;
            }
        }
        i = p.max(i + 1);
        if i >= bytes.len() {
            break;
        }
    }
    n
}

/// Parsed METS census.
#[derive(Debug, Clone, PartialEq)]
pub struct Mets {
    /// The `…/METS/` namespace URI appears on the root.
    pub has_mets_ns: bool,
    /// `metsHdr` elements.
    pub mets_hdr: u32,
    /// `dmdSec` descriptive-metadata sections.
    pub dmd_secs: u32,
    /// `amdSec` administrative sections.
    pub amd_secs: u32,
    /// `fileSec` elements.
    pub file_secs: u32,
    /// `file` elements inside `fileSec`.
    pub files: u32,
    /// `FLocat` file locations.
    pub flocats: u32,
    /// `structMap` sections.
    pub struct_maps: u32,
    /// `structLink` sections.
    pub struct_links: u32,
    /// `agent` entries in `metsHdr`.
    pub agents: u32,
}

/// `true` on a `<mets`-prefixed root or `…/METS/` namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("/METS/") && (elem_count(s, "mets") > 0 || s.contains("<mets"))
}

/// Census; `None` without a METS root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mets> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    Some(Mets {
        has_mets_ns: s.contains("/METS/"),
        mets_hdr: elem_count(s, "metsHdr"),
        dmd_secs: elem_count(s, "dmdSec"),
        amd_secs: elem_count(s, "amdSec"),
        file_secs: elem_count(s, "fileSec"),
        files: elem_count(s, "file"),
        flocats: elem_count(s, "FLocat"),
        struct_maps: elem_count(s, "structMap"),
        struct_links: elem_count(s, "structLink"),
        agents: elem_count(s, "agent"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] =
        b"<mets:mets xmlns:mets=\"x/METS/\"><mets:metsHdr><mets:agent/></mets:metsHdr>\
<mets:dmdSec/><mets:amdSec/><mets:fileSec><mets:file><mets:FLocat/></mets:file></mets:fileSec>\
<mets:structMap/><mets:structLink/></mets:mets>";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<html><body>no</body></html>"));
    }

    #[test]
    fn parses() {
        let m = parse(D).unwrap();
        assert!(m.has_mets_ns);
        assert_eq!(m.mets_hdr, 1);
        assert_eq!(m.dmd_secs, 1);
        assert_eq!(m.amd_secs, 1);
        assert_eq!(m.file_secs, 1);
        assert_eq!(m.files, 1);
        assert_eq!(m.flocats, 1);
        assert_eq!(m.struct_maps, 1);
        assert_eq!(m.struct_links, 1);
        assert_eq!(m.agents, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

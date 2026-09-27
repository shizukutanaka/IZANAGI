//! IMS Common Cartridge / Content Packaging `imsmanifest.xml` —
//! `<manifest>` root with `<organizations>` item tree and
//! `<resources>` href table.
//!
//! ```
//! use izanagi_kit::imscc::parse;
//!
//! let x = br#"<manifest><organizations><organization><item identifierref="r1">
//!   <title>Intro</title></item></organization></organizations>
//!   <resources><resource identifier="r1" href="intro.html" type="webcontent"/>
//!   </resources></manifest>"#;
//! let m = parse(x).unwrap();
//! assert_eq!(m.items[0].title, "Intro");
//! assert_eq!(m.resources[0].1, "intro.html");
//! ```

use std::string::String;
use std::vec::Vec;

/// One `<item>` in the organization tree.
#[derive(Clone, Debug)]
pub struct Item {
    /// `<title>` text.
    pub title: String,
    /// `identifierref` when present.
    pub identifierref: Option<String>,
    /// Nesting depth (top-level items are 0).
    pub depth: usize,
}

/// Parsed manifest.
#[derive(Clone, Debug)]
pub struct Manifest {
    /// `<item>` entries in document order.
    pub items: Vec<Item>,
    /// `<resource>` entries `(identifier, href)`.
    pub resources: Vec<(String, String)>,
    /// `identifier` attribute of the root `<manifest>`.
    pub identifier: Option<String>,
}

fn attr(tag: &[u8], name: &[u8]) -> Option<String> {
    let mut pat = Vec::new();
    pat.extend_from_slice(name);
    pat.push(b'=');
    pat.push(b'"');
    let at = tag.windows(pat.len()).position(|w| w == pat.as_slice())?;
    let v0 = at + pat.len();
    let e = tag[v0..].iter().position(|c| *c == b'"')? + v0;
    Some(String::from_utf8_lossy(&tag[v0..e]).into_owned())
}

fn open_tag(d: &[u8], name: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut pat = Vec::with_capacity(name.len() + 1);
    pat.push(b'<');
    pat.extend_from_slice(name);
    let mut i = from;
    loop {
        let p = d[i..]
            .windows(pat.len())
            .position(|w| w == pat.as_slice())
            .map(|p| i + p)?;
        let after = p + pat.len();
        match d.get(after) {
            Some(c) if c.is_ascii_alphanumeric() || *c == b'_' || *c == b':' || *c == b'-' => {
                i = p + 1;
                continue;
            }
            None => return None,
            _ => {}
        }
        let gt = d[after..].iter().position(|c| *c == b'>')? + after;
        return Some((p, gt + 1));
    }
}

fn close_pos(d: &[u8], name: &[u8], from: usize) -> Option<usize> {
    let mut pat = Vec::with_capacity(name.len() + 3);
    pat.extend_from_slice(b"</");
    pat.extend_from_slice(name);
    d[from..]
        .windows(pat.len())
        .position(|w| w == pat.as_slice())
        .map(|p| from + p)
}

/// Parse an `imsmanifest.xml` document.
pub fn parse(d: &[u8]) -> Option<Manifest> {
    let (ms, mb) = open_tag(d, b"manifest", 0)?;
    let identifier = attr(&d[ms..mb], b"identifier");
    let me = close_pos(d, b"manifest", mb)?;

    let mut out = Manifest {
        items: Vec::new(),
        resources: Vec::new(),
        identifier,
    };

    // organization items
    let mut i = mb;
    let mut stack: usize = 0;
    let in_org = if let Some((_, tb)) = open_tag(d, b"organizations", i) {
        i = tb;
        true
    } else {
        false
    };
    if in_org {
        while i < me {
            if d[i..].starts_with(b"</item") {
                stack = stack.saturating_sub(1);
                i += 6;
                continue;
            }
            if let Some((ts, tb)) = open_tag(d, b"item", i) {
                let idref = attr(&d[ts..tb], b"identifierref");
                let te = close_pos(d, b"item", tb).unwrap_or(me);
                let title = open_tag(d, b"title", tb)
                    .filter(|(os, _)| *os < te)
                    .and_then(|(_, t0)| close_pos(d, b"title", t0))
                    .map(|t1| {
                        let t0 = open_tag(d, b"title", tb).map(|(_, x)| x).unwrap_or(tb);
                        String::from_utf8_lossy(&d[t0..t1]).trim().to_string()
                    })
                    .unwrap_or_default();
                out.items.push(Item {
                    title,
                    identifierref: idref,
                    depth: stack,
                });
                stack += 1;
                i = tb;
            } else {
                i += 1;
            }
        }
    }

    // resources
    let mut j = mb;
    while let Some((ts, tb)) = open_tag(d, b"resource", j) {
        if ts >= me {
            break;
        }
        let tag = &d[ts..tb];
        let id = attr(tag, b"identifier").unwrap_or_default();
        let href = attr(tag, b"href").unwrap_or_default();
        out.resources.push((id, href));
        j = tb;
    }

    if out.items.is_empty() && out.resources.is_empty() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest() {
        let x = br#"<manifest identifier="m1"><organizations><organization>
          <item identifierref="r1"><title>A</title><item identifierref="r2"><title>B</title></item></item>
          </organization></organizations>
          <resources><resource identifier="r1" href="a.html"/><resource identifier="r2" href="b.html"/></resources>
          </manifest>"#;
        let m = parse(x).unwrap();
        assert_eq!(m.identifier.as_deref(), Some("m1"));
        assert_eq!(m.items.len(), 2);
        assert_eq!(m.items[1].depth, 1);
        assert_eq!(m.resources.len(), 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<manifest/>").is_none());
        assert!(parse(b"not xml").is_none());
    }
}

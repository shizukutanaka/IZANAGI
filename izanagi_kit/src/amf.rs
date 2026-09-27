//! Minimal reader for AMF (Additive Manufacturing File Format,
//! ISO/ASTM 52915): XML `<amf unit="..."><object id="N"><mesh>` trees —
//! this module reports the unit, the object ids, and per-object
//! vertex/triangle counts (`<vertex>`/`<triangle>` tallies).
//!
//! ```
//! use izanagi_kit::amf::parse;
//!
//! let a = parse(
//!     b"<amf unit=\"millimeter\"><object id=\"1\"><mesh>\
//!        <vertices><vertex/><vertex/><vertex/></vertices>\
//!        <volume><triangle/><triangle/></volume></mesh></object></amf>",
//! )
//! .unwrap();
//! assert_eq!(a.unit.as_deref(), Some("millimeter"));
//! assert_eq!(a.objects[0].id, 1);
//! assert_eq!(a.objects[0].vertices, 3);
//! assert_eq!(a.objects[0].triangles, 2);
//! ```

/// One `<object>` mesh summary.
#[derive(Debug)]
pub struct Object {
    /// `id` attribute (0 when absent or non-numeric).
    pub id: u64,
    /// `<vertex>` element count inside `<vertices>`.
    pub vertices: usize,
    /// `<triangle>` element count inside `<volume>`(s).
    pub triangles: usize,
}

/// A parsed AMF document.
#[derive(Debug)]
pub struct Amf {
    /// `unit` attribute of `<amf>` (`millimeter`, `inch`, ...).
    pub unit: Option<String>,
    /// Objects in document order.
    pub objects: Vec<Object>,
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

/// Occurrences of `open` as an element start (next byte after the name is
/// a word boundary: ` `, `>`, `/`, tab, CR, LF).
fn count_elements(src: &str, open: &str) -> usize {
    let mut n = 0;
    let mut rest = src;
    while let Some(a) = rest.find(open) {
        let next = rest.as_bytes().get(a + open.len()).copied();
        if matches!(
            next,
            Some(b' ') | Some(b'>') | Some(b'/') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            n += 1;
        }
        rest = &rest[a + open.len()..];
    }
    n
}

/// Parse an AMF document. `None` without an `<amf` root.
pub fn parse(data: &[u8]) -> Option<Amf> {
    let src = std::str::from_utf8(data).ok()?;
    let a = src.find("<amf")?;
    let head_end = src[a..].find('>')? + a;
    let unit = attr(&src[a..head_end + 1], "unit");
    let mut objects = Vec::new();
    let mut rest = &src[head_end + 1..];
    while let Some(o) = rest.find("<object") {
        // `<object` vs `<objectid`-ish siblings: next byte must be a boundary
        let nb = rest.as_bytes().get(o + 7).copied();
        if !matches!(
            nb,
            Some(b' ') | Some(b'>') | Some(b'/') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            rest = &rest[o + 7..];
            continue;
        }
        let tag_end = rest[o..].find('>')? + o;
        let id = attr(&rest[o..tag_end + 1], "id")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let body = match rest[tag_end + 1..].find("</object>") {
            Some(e) => &rest[tag_end + 1..tag_end + 1 + e],
            None => break,
        };
        objects.push(Object {
            id,
            vertices: count_elements(body, "<vertex"),
            triangles: count_elements(body, "<triangle"),
        });
        rest = &rest[tag_end + 1 + body.len() + "</object>".len()..];
    }
    Some(Amf { unit, objects })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let a = parse(
            b"<?xml version=\"1.0\"?><amf unit=\"inch\">\
              <object id=\"5\"><mesh><vertices><vertex/></vertices>\
              <volume><triangle/></volume></mesh></object>\
              <object><mesh><vertices/></mesh></object></amf>",
        )
        .unwrap();
        assert_eq!(a.unit.as_deref(), Some("inch"));
        assert_eq!(a.objects.len(), 2);
        assert_eq!(a.objects[0].id, 5);
        assert_eq!(a.objects[0].vertices, 1);
        assert_eq!(a.objects[0].triangles, 1);
        assert_eq!(a.objects[1].id, 0); // no id attr
    }

    #[test]
    fn vertex_boundary() {
        // `<vertices>` must not be counted as a `<vertex>`
        let a = parse(b"<amf><object><vertices><vertex/></vertices></object></amf>").unwrap();
        assert_eq!(a.objects[0].vertices, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<xml/>").is_none());
        assert!(parse(&[0xFF]).is_none());
    }
}

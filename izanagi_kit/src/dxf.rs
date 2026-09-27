//! AutoCAD DXF ASCII structure: `(group code, value)` line pairs.
//!
//! A DXF file is alternating lines of an integer *group code* and a
//! raw value line. `0` opens entities/objects/sections, `2` carries a
//! name, `999` is a comment. The file is a `SECTION`/`ENDSEC` tree
//! ending in `0`/`EOF`. This module walks the pair stream and slices
//! out the named sections and ENTITIES entries — integer-only, no
//! geometric evaluation.
//!
//! ```
//! use izanagi_kit::dxf::{pairs, sections, entities, code, value};
//! let d = b"  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n  8\n0\n  0\nENDSEC\n  0\nEOF\n";
//! let ps = pairs(d);
//! assert_eq!(code(&ps[0]), 0);
//! assert_eq!(value(d, &ps[0]), b"SECTION");
//! let ss = sections(d);
//! assert_eq!(ss.len(), 1);
//! assert_eq!(&d[ss[0].name_at..ss[0].name_at + ss[0].name_len], b"ENTITIES");
//! let es = entities(d, &ss[0]);
//! assert_eq!(es.len(), 1);
//! assert_eq!(&d[es[0].kind_at..es[0].kind_at + es[0].kind_len], b"LINE");
//! ```

/// One `(code, value)` pair: `at`..`code_end` is the code line,
/// `val_at`..`val_end` the value line (end-exclusive, without the
/// line terminator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    /// Offset of the group-code line.
    pub at: usize,
    /// Offset of the value line.
    pub val_at: usize,
    /// End of the value line.
    pub val_end: usize,
    /// End of the whole pair (start of the next line).
    pub next: usize,
    /// Parsed group code (`-1` when the line is not an integer).
    pub code_at: i32,
}

/// A named `SECTION`/`ENDSEC` span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    /// Byte range of the section name (the `2` group's value).
    pub name_at: usize,
    /// Length of the name.
    pub name_len: usize,
    /// Offset just past `SECTION`'s value line — section body start.
    pub body_at: usize,
    /// Offset of the matching `0`/`ENDSEC` pair.
    pub end: usize,
}

/// One entity inside `ENTITIES`: kind is the value of the `0` group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity {
    /// Byte offset of the entity-kind value (e.g. `LINE`).
    pub kind_at: usize,
    /// Length of the kind name.
    pub kind_len: usize,
    /// Offset of the `0` group that opened this entity.
    pub at: usize,
}

fn line_end(d: &[u8], at: usize) -> usize {
    let mut i = at;
    while i < d.len() && d[i] != b'\n' {
        i += 1;
    }
    i
}

fn line_body_end(d: &[u8], at: usize) -> usize {
    let mut e = line_end(d, at);
    if e > at && d[e - 1] == b'\r' {
        e -= 1;
    }
    e
}

/// The pair's group code as parsed from the trimmed first line
/// (already stored on the pair — this just reads `code_at`).
pub fn code(p: &Pair) -> i32 {
    p.code_at
}

/// The raw value bytes of a pair's value line.
pub fn value<'a>(d: &'a [u8], p: &Pair) -> &'a [u8] {
    &d[p.val_at..p.val_end]
}

fn parse_code(d: &[u8], at: usize, end: usize) -> Option<i32> {
    let mut i = at;
    while i < end && (d[i] == b' ' || d[i] == b'\t') {
        i += 1;
    }
    let mut neg = false;
    if i < end && d[i] == b'-' {
        neg = true;
        i += 1;
    }
    if i >= end || !d[i].is_ascii_digit() {
        return None;
    }
    let mut v: i64 = 0;
    while i < end && d[i].is_ascii_digit() {
        v = v * 10 + (d[i] - b'0') as i64;
        if v > i32::MAX as i64 {
            return None;
        }
        i += 1;
    }
    while i < end && (d[i] == b' ' || d[i] == b'\t') {
        i += 1;
    }
    if i != end {
        return None;
    }
    Some(if neg { -(v as i32) } else { v as i32 })
}

/// Walk every `(code, value)` pair. Stops at the first malformed code
/// line or missing value line.
pub fn pairs(d: &[u8]) -> Vec<Pair> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < d.len() {
        let code_end = line_body_end(d, at);
        let c = match parse_code(d, at, code_end) {
            Some(c) => c,
            None => break,
        };
        let val_at = line_end(d, at) + 1;
        if val_at >= d.len() {
            break; // a code line with no value line is malformed
        }
        let val_end = line_body_end(d, val_at);
        let next = line_end(d, val_at) + 1;
        out.push(Pair {
            at,
            val_at,
            val_end,
            next,
            code_at: c,
        });
        at = next;
    }
    out
}

/// Top-level `SECTION`/`ENDSEC` spans (HEADER, TABLES, BLOCKS,
/// ENTITIES, OBJECTS, CLASSES, THUMBNAILIMAGE, …).
pub fn sections(d: &[u8]) -> Vec<Section> {
    let ps = pairs(d);
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 1 < ps.len() {
        if ps[i].code_at == 0 && value(d, &ps[i]) == b"SECTION" && ps[i + 1].code_at == 2 {
            let name_at = ps[i + 1].val_at;
            let name_len = ps[i + 1].val_end - ps[i + 1].val_at;
            let body_at = ps[i + 1].next;
            let mut j = i + 2;
            while j < ps.len() {
                if ps[j].code_at == 0 && value(d, &ps[j]) == b"ENDSEC" {
                    out.push(Section {
                        name_at,
                        name_len,
                        body_at,
                        end: ps[j].at,
                    });
                    i = j;
                    break;
                }
                j += 1;
            }
        }
        i += 1;
    }
    out
}

/// Entities of a section (every `0`-group between `body_at` and
/// `end`; the section should be the `ENTITIES` one, but any section
/// works — TABLE rows appear the same way).
pub fn entities(d: &[u8], s: &Section) -> Vec<Entity> {
    pairs(&d[s.body_at..s.end])
        .into_iter()
        .filter(|p| p.code_at == 0 && p.val_end > p.val_at)
        .map(|p| Entity {
            kind_at: s.body_at + p.val_at,
            kind_len: p.val_end - p.val_at,
            at: s.body_at + p.at,
        })
        .collect()
}

/// `true` when the pair stream reaches a `0`/`EOF` pair.
pub fn is_complete(d: &[u8]) -> bool {
    let ps = pairs(d);
    ps.iter().any(|p| p.code_at == 0 && value(d, p) == b"EOF")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        b"999\ncomment\n  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  0\nENDSEC\n  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n  8\n0\n 10\n  0\n 20\n  0\n  0\nCIRCLE\n  0\nENDSEC\n  0\nEOF\n".to_vec()
    }

    #[test]
    fn pairs_and_sections() {
        let d = fixture();
        let ps = pairs(&d);
        assert_eq!(ps[0].code_at, 999);
        assert_eq!(value(&d, &ps[0]), b"comment");
        let ss = sections(&d);
        assert_eq!(ss.len(), 2);
        assert_eq!(&d[ss[0].name_at..ss[0].name_at + ss[0].name_len], b"HEADER");
        assert_eq!(
            &d[ss[1].name_at..ss[1].name_at + ss[1].name_len],
            b"ENTITIES"
        );
        let es = entities(&d, &ss[1]);
        assert_eq!(es.len(), 2);
        assert_eq!(&d[es[0].kind_at..es[0].kind_at + es[0].kind_len], b"LINE");
        assert_eq!(&d[es[1].kind_at..es[1].kind_at + es[1].kind_len], b"CIRCLE");
        assert!(is_complete(&d));
        assert_eq!(code(&ps[1]), 0);
    }

    #[test]
    fn rejects_and_edge() {
        assert!(pairs(b"abc\nfoo\n").is_empty());
        // code without value line → pair dropped
        let ps = pairs(b"  0\n");
        assert!(ps.is_empty());
        let mut d = fixture();
        d.truncate(d.len() - 6); // drop EOF
        assert!(!is_complete(&d));
        assert_eq!(sections(b"  0\nEOF\n").len(), 0);
    }
}

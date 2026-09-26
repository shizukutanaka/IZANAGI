//! Encapsulated PostScript header via DSC comment conventions
//! (Adobe Document Structuring Conventions 3.0).
//!
//! The first line is `%!PS-Adobe-x.y` optionally suffixed
//! ` EPSF-x.y`. `%%` comment lines carry `%%Key: value` fields —
//! `%%BoundingBox: llx lly urx ury`, `%%HiResBoundingBox:` (kept as
//! raw text — floats live in strings only), `%%Pages: n`,
//! `%%Title:`/`%%Creator:`/`%%CreationDate:`, ending at
//! `%%EndComments`. Page markers `%%Page: label ordinal` and the
//! `%%Trailer`/`%%EOF` pair are walked as comments too.
//!
//! ```
//! use izanagi_kit::eps::{parse, comments, key, val};
//! let d = b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 50\n%%Pages: 1\n%%EndComments\n%%Page: 1 1\n%%Trailer\n%%EOF\n";
//! let e = parse(d).unwrap();
//! assert!(e.epsf);
//! assert_eq!(e.bounding_box, Some((0, 0, 100, 50)));
//! assert_eq!(e.pages, Some(1));
//! let cs = comments(d);
//! assert_eq!(key(d, &cs[0]), b"BoundingBox");
//! assert_eq!(val(d, &cs[0]), b" 0 0 100 50");
//! ```

/// One `%%Key:` comment line (leading `%%` not included in the key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comment {
    /// Offset of the key.
    pub key_at: usize,
    /// Key length (up to `:` or end of line).
    pub key_len: usize,
    /// Offset of the raw value (everything after `:` — leading space
    /// preserved).
    pub val_at: usize,
    /// Value end (line end, `\r` stripped).
    pub val_end: usize,
    /// Offset just past this line.
    pub next: usize,
}

/// Parsed EPS/DSC header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eps {
    /// `true` when the first line carries ` EPSF-`.
    pub epsf: bool,
    /// `%%BoundingBox:` as integers `(llx, lly, urx, ury)`, or `atend`
    /// when it defers to the trailer (`None`).
    pub bounding_box: Option<(i64, i64, i64, i64)>,
    /// `%%Pages:` count (`None` for `atend` or missing).
    pub pages: Option<i64>,
    /// Offset just past `%%EndComments` (or just past the first line
    /// when there are no `%%` comments).
    pub header_end: usize,
    /// Number of `%%Page:` markers seen.
    pub page_marks: usize,
    /// `true` when `%%EOF` terminates the file.
    pub has_eof: bool,
}

fn line_end(d: &[u8], at: usize) -> usize {
    let mut i = at;
    while i < d.len() && d[i] != b'\n' {
        i += 1;
    }
    i
}

fn body_end(d: &[u8], at: usize) -> usize {
    let mut e = line_end(d, at);
    if e > at && d[e - 1] == b'\r' {
        e -= 1;
    }
    e
}

/// Slice of a comment's key.
pub fn key<'a>(d: &'a [u8], c: &Comment) -> &'a [u8] {
    &d[c.key_at..c.key_at + c.key_len]
}

/// Slice of a comment's raw value (leading spaces kept).
pub fn val<'a>(d: &'a [u8], c: &Comment) -> &'a [u8] {
    &d[c.val_at..c.val_end]
}

/// Walk every `%%`-comment line. A `%!` first line is skipped — it is
/// a signature, not a comment.
pub fn comments(d: &[u8]) -> Vec<Comment> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < d.len() {
        if !d[at..].starts_with(b"%%") {
            at = line_end(d, at) + 1;
            continue;
        }
        let end = body_end(d, at);
        let mut colon = at + 2;
        while colon < end && d[colon] != b':' {
            colon += 1;
        }
        let (key_len, val_at) = if colon < end {
            (colon - (at + 2), colon + 1)
        } else {
            (end - (at + 2), end)
        };
        out.push(Comment {
            key_at: at + 2,
            key_len,
            val_at,
            val_end: end,
            next: line_end(d, at) + 1,
        });
        at = line_end(d, at) + 1;
    }
    out
}

fn int_fields(v: &[u8]) -> Option<(i64, i64, i64, i64)> {
    let mut out = [0i64; 4];
    let mut i = 0usize;
    let mut n = 0usize;
    while i <= v.len() {
        if i == v.len() || v[i] == b' ' || v[i] == b'\t' {
            i += 1;
            continue;
        }
        let start = i;
        if v[i] == b'-' || v[i] == b'+' {
            i += 1;
        }
        while i < v.len() && v[i].is_ascii_digit() {
            i += 1;
        }
        if n >= 4 || i == start || (i == start + 1 && (v[start] == b'-' || v[start] == b'+')) {
            return None;
        }
        let mut x: i64 = 0;
        for &b in &v[start..i] {
            if b.is_ascii_digit() {
                x = x.checked_mul(10)?.checked_add((b - b'0') as i64)?;
            }
        }
        if v[start] == b'-' {
            x = -x;
        }
        out[n] = x;
        n += 1;
    }
    if n == 4 {
        Some((out[0], out[1], out[2], out[3]))
    } else {
        None
    }
}

fn first_int(v: &[u8]) -> Option<i64> {
    let mut i = 0usize;
    while i < v.len() && (v[i] == b' ' || v[i] == b'\t') {
        i += 1;
    }
    let neg = i < v.len() && v[i] == b'-';
    if neg {
        i += 1;
    }
    let start = i;
    while i < v.len() && v[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return None;
    }
    let mut x: i64 = 0;
    for &b in &v[start..i] {
        x = x.checked_mul(10)?.checked_add((b - b'0') as i64)?;
    }
    Some(if neg { -x } else { x })
}

/// Parse the `%!PS-Adobe` signature line plus the DSC comment block.
/// `None` when the file does not start with `%!PS`.
pub fn parse(d: &[u8]) -> Option<Eps> {
    let end0 = body_end(d, 0);
    if !d.get(..end0)?.starts_with(b"%!PS") {
        return None;
    }
    let epsf = d[..end0].windows(5).any(|w| w == b"EPSF-");
    let cs = comments(d);
    let mut bb = None;
    let mut pages = None;
    let mut page_marks = 0usize;
    let mut has_eof = false;
    let mut header_end = line_end(d, 0) + 1;
    for c in &cs {
        let k = key(d, c);
        let v = val(d, c);
        if k == b"BoundingBox" {
            bb = int_fields(v);
        } else if k == b"Pages" {
            pages = first_int(v);
        } else if k == b"Page" {
            page_marks += 1;
        } else if k == b"EndComments" {
            header_end = c.next;
        } else if k == b"EOF" {
            has_eof = true;
        }
    }
    Some(Eps {
        epsf,
        bounding_box: bb,
        pages,
        header_end,
        page_marks,
        has_eof,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        b"%!PS-Adobe-3.0 EPSF-3.0\n%%Creator: devin\n%%Title: t\n%%BoundingBox: -10 5 100 200\n%%HiResBoundingBox: -10.5 5 100 200\n%%Pages: 2\n%%EndComments\n%%Page: p1 1\n%%Page: p2 2\n%%Trailer\n%%BoundingBox: -10 5 100 200\n%%EOF\n".to_vec()
    }

    #[test]
    fn header_fields() {
        let d = fixture();
        let e = parse(&d).unwrap();
        assert!(e.epsf);
        assert_eq!(e.bounding_box, Some((-10, 5, 100, 200)));
        assert_eq!(e.pages, Some(2));
        assert_eq!(e.page_marks, 2);
        assert!(e.has_eof);
        let cs = comments(&d);
        assert!(cs.len() >= 8);
        assert_eq!(key(&d, &cs[0]), b"Creator");
        assert_eq!(val(&d, &cs[0]), b" devin");
        // BoundingBox is at cs[2]; HiRes at cs[3]
        assert_eq!(key(&d, &cs[3]), b"HiResBoundingBox");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"%%BoundingBox: 0 0 1 1\n").is_none());
        assert!(parse(b"PDF-1.4\n%%EOF\n").is_none());
        // bare signature still parses (zero comments)
        assert!(parse(b"%!PS").unwrap().bounding_box.is_none());
    }

    #[test]
    fn non_eps_ps() {
        let d = b"%!PS-Adobe-3.0\n%%Pages: 1\n%%EOF\n";
        let e = parse(d).unwrap();
        assert!(!e.epsf);
        assert_eq!(e.bounding_box, None);
        assert_eq!(e.pages, Some(1));
    }
}

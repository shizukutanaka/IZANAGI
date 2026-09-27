//! MHTML / MIME-HTML multipart archive scanning (RFC 2557).
//!
//! An `.mht` file is a `multipart/related` message: a top header block
//! declares `Content-Type: multipart/related; boundary="..."`, then each
//! part is split on `--boundary` and carries its own MIME headers
//! (`Content-Type`, `Content-Location`, `Content-Transfer-Encoding`).
//! Bodies are kept verbatim (no transfer decoding).
//!
//! ```
//! use izanagi_kit::mht;
//! let d = b"Content-Type: multipart/related; boundary=\"x\"\r\n\r\n\
//!           --x\r\nContent-Type: text/html\r\n\r\n<html/>\r\n\
//!           --x\r\nContent-Location: i.png\r\nContent-Type: image/png\r\n\r\n\x89PNG\r\n--x--\r\n";
//! let m = mht::parse(d).unwrap();
//! assert_eq!(m.parts.len(), 2);
//! assert_eq!(m.parts[1].content_location.as_deref(), Some(b"i.png".as_ref()));
//! ```

use std::vec::Vec;

/// One MIME body part.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    /// All part headers as `(name, value)` pairs, names verbatim case.
    pub headers: Vec<(Vec<u8>, Vec<u8>)>,
    /// `Content-Type` value (`b"text/html"`), or empty.
    pub content_type: Vec<u8>,
    /// `Content-Location` value, if present.
    pub content_location: Option<Vec<u8>>,
    /// Undecoded body bytes (between the header blank line and boundary).
    pub data: Vec<u8>,
}

/// A parsed MHTML message.
#[derive(Clone, Debug, PartialEq)]
pub struct Mht {
    /// Boundary delimiter bytes (without the leading `--`).
    pub boundary: Vec<u8>,
    /// Body parts in file order; the first is normally the HTML page.
    pub parts: Vec<Part>,
}

type Hdrs = Vec<(Vec<u8>, Vec<u8>)>;

fn headers(d: &[u8]) -> Option<(Hdrs, usize)> {
    // Returns (headers, offset-after-blank-line). Supports folded lines.
    let mut hs = Vec::new();
    let mut i = 0;
    loop {
        let eol = find(d, i, b"\n")?;
        let mut line_end = eol;
        if line_end > i && d[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        let line = &d[i..line_end];
        if line.is_empty() {
            return Some((hs, eol + 1));
        }
        if line[0] == b' ' || line[0] == b'\t' {
            let last = hs.last_mut()?;
            last.1.push(b' ');
            last.1.extend_from_slice(trim(line));
        } else {
            let (name, val) = split_colon(line)?;
            hs.push((name.to_vec(), trim(val).to_vec()));
        }
        i = eol + 1;
    }
}

fn split_colon(line: &[u8]) -> Option<(&[u8], &[u8])> {
    for (i, &b) in line.iter().enumerate() {
        if b == b':' {
            return Some((&line[..i], &line[i + 1..]));
        }
    }
    None
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

fn find(d: &[u8], from: usize, pat: &[u8]) -> Option<usize> {
    if pat.is_empty() || d.len() < pat.len() || from > d.len() - pat.len() {
        return None;
    }
    let mut i = from;
    while i + pat.len() <= d.len() {
        if &d[i..i + pat.len()] == pat {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn header<'a>(hs: &'a [(Vec<u8>, Vec<u8>)], name: &[u8]) -> Option<&'a [u8]> {
    hs.iter()
        .find(|(n, _)| eq_ci(n, name))
        .map(|(_, v)| v.as_slice())
}

fn eq_ci(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn boundary_of(ct: &[u8]) -> Option<Vec<u8>> {
    // Content-Type: multipart/related; boundary="----x"
    let i = find(ct, 0, b"boundary")?;
    let mut j = i + 8;
    while j < ct.len() && (ct[j] == b' ' || ct[j] == b'=') {
        j += 1;
    }
    if j >= ct.len() {
        return None;
    }
    if ct[j] == b'"' {
        let e = find(ct, j + 1, b"\"")?;
        Some(ct[j + 1..e].to_vec())
    } else {
        let mut e = j;
        while e < ct.len() && ct[e] != b';' && ct[e] != b' ' {
            e += 1;
        }
        Some(ct[j..e].to_vec())
    }
}

/// Parses a raw `.mht` message.
pub fn parse(d: &[u8]) -> Option<Mht> {
    let (top, body_off) = headers(d)?;
    let ct = header(&top, b"Content-Type")?;
    let boundary = boundary_of(ct)?;
    if boundary.is_empty() {
        return None;
    }
    let mut delim = Vec::with_capacity(boundary.len() + 2);
    delim.extend_from_slice(b"--");
    delim.extend_from_slice(&boundary);
    let mut parts = Vec::new();
    let mut i = body_off;
    loop {
        let s = find(d, i, &delim)?;
        let mut j = s + delim.len();
        // "--" suffix = final boundary.
        if d.get(j..j + 2) == Some(b"--") {
            break;
        }
        // Skip to end of the boundary line.
        while j < d.len() && d[j] != b'\n' {
            j += 1;
        }
        j += 1;
        let next = find(d, j, &delim).unwrap_or(d.len());
        let seg = &d[j..next];
        if let Some((hs, off)) = headers(seg) {
            let content_type = header(&hs, b"Content-Type").unwrap_or(b"").to_vec();
            let content_location = header(&hs, b"Content-Location").map(|v| v.to_vec());
            let mut data = seg[off..].to_vec();
            // Trim the CRLF immediately before the next boundary.
            while data.last() == Some(&b'\r') || data.last() == Some(&b'\n') {
                data.pop();
            }
            parts.push(Part {
                headers: hs,
                content_type,
                content_location,
                data,
            });
        }
        i = next;
        if next >= d.len() {
            break;
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(Mht { boundary, parts })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_parts() {
        let d = b"Mime-Version: 1.0\r\nContent-Type: multipart/related; boundary=sep\n\n\
                  --sep\nContent-Type: text/html\n\n<a/>\n--sep\n\
                  Content-Location: x.png\nContent-Type: image/png\n\nPNG!\n--sep--";
        let m = parse(d).unwrap();
        assert_eq!(&m.boundary, b"sep");
        assert_eq!(m.parts.len(), 2);
        assert_eq!(&m.parts[0].content_type, b"text/html");
        assert_eq!(
            m.parts[1].content_location.as_deref(),
            Some(b"x.png".as_ref())
        );
        assert_eq!(&m.parts[1].data, b"PNG!");
    }

    #[test]
    fn part_headers_kept() {
        let d = b"Content-Type: multipart/related; boundary=\"b\"\n\n\
                  --b\nContent-Type: text/html\nX-Extra: 1\n\nx\n--b--";
        let m = parse(d).unwrap();
        assert!(m.parts[0]
            .headers
            .iter()
            .any(|(n, v)| n == b"X-Extra" && v == b"1"));
    }

    #[test]
    fn rejects_missing_boundary() {
        assert!(parse(b"Content-Type: text/plain\n\nhi").is_none());
        assert!(parse(b"garbage").is_none());
    }
}

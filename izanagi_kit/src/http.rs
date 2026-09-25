//! HTTP/1.1 wire codec (RFC 7230): request/status lines, header
//! blocks, `Content-Length` and `chunked` transfer decoding, and
//! canonical emission. Parsers are total — a malformed header line or
//! truncated chunk yields `None`, never a panic.
//!
//! `ws` covers the upgrade tunnel and `dns` the name resolution;
//! `http` is the request/response layer between them.
//!
//! ```
//! use izanagi_kit::http;
//! let r = http::parse_request(b"GET /x HTTP/1\x2e1\r\nHost: h\r\n\r\n").unwrap();
//! assert_eq!((r.method.as_str(), r.target.as_str()), ("GET", "/x"));
//! ```

/// One header line, name preserved as-sent.
pub type Header = (String, String);

fn split_head(d: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let mut i = 0;
    while i + 3 < d.len() {
        if d[i] == b'\r' && d[i + 1] == b'\n' && d[i + 2] == b'\r' && d[i + 3] == b'\n' {
            return Some((d[..i].to_vec(), &d[i + 4..]));
        }
        i += 1;
    }
    // tolerate bare-LF framing
    let mut i = 0;
    while i + 1 < d.len() {
        if d[i] == b'\n' && d[i + 1] == b'\n' {
            return Some((d[..i].to_vec(), &d[i + 2..]));
        }
        i += 1;
    }
    None
}

fn parse_headers(lines: &str) -> Option<Vec<Header>> {
    let mut out = Vec::new();
    for line in lines.lines() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            return None; // obs-fold: deprecated, treat as malformed
        }
        let c = line.find(':')?;
        let name = &line[..c];
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return None;
        }
        out.push((name.to_string(), line[c + 1..].trim().to_string()));
    }
    Some(out)
}

/// Case-insensitive header lookup; first match wins.
pub fn header<'a>(hs: &'a [Header], name: &str) -> Option<&'a str> {
    hs.iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Set/replace a header value (canonical emission helper).
pub fn set_header(hs: &mut Vec<Header>, name: &str, value: &str) {
    if let Some(e) = hs.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
        e.1 = value.to_string();
    } else {
        hs.push((name.to_string(), value.to_string()));
    }
}

fn body_of(hs: &[Header], rest: &[u8]) -> Option<Vec<u8>> {
    if let Some(v) = header(hs, "transfer-encoding") {
        if v.to_ascii_lowercase().contains("chunked") {
            return chunked(rest).map(|(b, _)| b);
        }
    }
    if let Some(v) = header(hs, "content-length") {
        let n: usize = v.trim().parse().ok()?;
        if rest.len() < n {
            return None;
        }
        return Some(rest[..n].to_vec());
    }
    Some(rest.to_vec())
}

/// Decode a chunked body; returns `(body, bytes consumed)` so trailers
/// or a following pipelined message stay reachable. `None` on a bad
/// chunk-size or a truncated chunk.
pub fn chunked(d: &[u8]) -> Option<(Vec<u8>, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    loop {
        let eol = d[at..].iter().position(|&c| c == b'\n')? + at;
        let line = std::str::from_utf8(&d[at..eol]).ok()?.trim();
        let line = line.split(';').next()?; // chunk extensions
        if line.is_empty() {
            return None;
        }
        let n = usize::from_str_radix(line, 16).ok()?;
        at = eol + 1;
        if n == 0 {
            // optional trailer headers until blank line
            loop {
                let te = d.get(at)?.eq(&b'\r') || d.get(at)?.eq(&b'\n');
                if te {
                    // bare blank line ends trailers
                    let adv = if d.get(at) == Some(&b'\r') && d.get(at + 1) == Some(&b'\n') {
                        2
                    } else {
                        1
                    };
                    return Some((out, at + adv));
                }
                let nl = d[at..].iter().position(|&c| c == b'\n')? + at;
                at = nl + 1;
            }
        }
        let end = at.checked_add(n)?;
        if end + 1 >= d.len() {
            return None;
        }
        out.extend_from_slice(&d[at..end]);
        if d.get(end) == Some(&b'\r') && d.get(end + 1) == Some(&b'\n') {
            at = end + 2;
        } else if d.get(end) == Some(&b'\n') {
            at = end + 1;
        } else {
            return None;
        }
    }
}

/// A parsed HTTP request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// Method token (`GET`, `POST`, …).
    pub method: String,
    /// Request-target (`/path?query`).
    pub target: String,
    /// Header list in wire order.
    pub headers: Vec<Header>,
    /// Body after `Content-Length`/`chunked` resolution.
    pub body: Vec<u8>,
}

/// A parsed HTTP response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    /// Three-digit status code.
    pub status: u16,
    /// Reason phrase (may be empty).
    pub reason: String,
    /// Header list in wire order.
    pub headers: Vec<Header>,
    /// Body after `Content-Length`/`chunked` resolution.
    pub body: Vec<u8>,
}

/// Parse `request-line + headers + body`; `None` on malformed syntax.
pub fn parse_request(d: &[u8]) -> Option<Request> {
    let (head, rest) = split_head(d)?;
    let text = std::str::from_utf8(&head).ok()?;
    let mut lines = text.lines();
    let rl = lines.next()?;
    let mut it = rl.splitn(3, ' ');
    let method = it.next()?;
    let target = it.next()?;
    let ver = it.next()?;
    if method.is_empty() || target.is_empty() || !ver.starts_with("HTTP/") {
        return None;
    }
    let headers = parse_headers(lines.collect::<Vec<_>>().join("\n").as_str())?;
    let body = body_of(&headers, rest)?;
    Some(Request {
        method: method.to_string(),
        target: target.to_string(),
        headers,
        body,
    })
}

/// Parse `status-line + headers + body`; `None` on malformed syntax.
pub fn parse_response(d: &[u8]) -> Option<Response> {
    let (head, rest) = split_head(d)?;
    let text = std::str::from_utf8(&head).ok()?;
    let mut lines = text.lines();
    let sl = lines.next()?;
    if !sl.starts_with("HTTP/") {
        return None;
    }
    let sp = sl.find(' ')?;
    let status: u16 = sl[sp + 1..sp + 4].parse().ok()?;
    let reason = sl[sp + 4..].trim().to_string();
    let headers = parse_headers(lines.collect::<Vec<_>>().join("\n").as_str())?;
    let body = body_of(&headers, rest)?;
    Some(Response {
        status,
        reason,
        headers,
        body,
    })
}

fn emit_head(first: &str, hs: &[Header], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(first.as_bytes());
    out.extend_from_slice(b"\r\n");
    let mut hs = hs.to_vec();
    if !body.is_empty() && header(&hs, "content-length").is_none() {
        set_header(&mut hs, "Content-Length", &body.len().to_string());
    }
    for (n, v) in &hs {
        out.extend_from_slice(n.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(v.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    out
}

/// Canonical request emission (adds `Content-Length` for a body).
pub fn emit_request(r: &Request) -> Vec<u8> {
    emit_head(
        &format!("{} {} HTTP/1\x2e1", r.method, r.target),
        &r.headers,
        &r.body,
    )
}

/// Canonical response emission (adds `Content-Length` for a body).
pub fn emit_response(r: &Response) -> Vec<u8> {
    emit_head(
        &format!("HTTP/1\x2e1 {} {}", r.status, r.reason),
        &r.headers,
        &r.body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let raw = b"POST /a/b?q=1 HTTP/1\x2e1\r\nHost: example\r\nX-A:  2 \r\nContent-Length: 4\r\n\r\nPING";
        let r = parse_request(raw).unwrap();
        assert_eq!(r.method, "POST");
        assert_eq!(r.target, "/a/b?q=1");
        assert_eq!(header(&r.headers, "host"), Some("example"));
        assert_eq!(header(&r.headers, "X-A"), Some("2"));
        assert_eq!(r.body, b"PING");
        let r2 = parse_request(&emit_request(&r)).unwrap();
        assert_eq!(r2, r);
    }

    #[test]
    fn response_and_chunked() {
        let raw = b"HTTP/1\x2e1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.reason, "OK");
        assert_eq!(r.body, b"Wikipedia");
        // chunk extensions + trailer
        let (b, used) = chunked(b"3;x=1\r\nabc\r\n0\r\nX-T: v\r\n\r\n").unwrap();
        assert_eq!(b, b"abc");
        assert!(used > 10);
        assert!(chunked(b"zz\r\n").is_none());
        assert!(chunked(b"5\r\nab").is_none());
    }

    #[test]
    fn malformed_inputs() {
        assert!(parse_request(b"GET /x\r\n\r\n").is_none()); // no version
        assert!(parse_request(b"GET /x HTTP/1\x2e1\r\nBad Header\r\n\r\n").is_none());
        assert!(parse_request(b"GET /x HTTP/1\x2e1\r\nBad: v\r\n\tfold\r\n\r\n").is_none());
        assert!(parse_request(b"GET /x HTTP/1\x2e1\r\nContent-Length: 99\r\n\r\nsh").is_none());
        assert!(parse_response(b"NOTHTTP 200 OK\r\n\r\n").is_none());
        assert!(parse_response(b"HTTP/1\x2e1 abc OK\r\n\r\n").is_none());
        assert!(
            parse_response(b"HTTP/1\x2e1 200 OK\r\n\r\nbody")
                .unwrap()
                .body
                == b"body"
        );
    }

    #[test]
    fn emit_canonicalizes() {
        let r = Request {
            method: "GET".into(),
            target: "/".into(),
            headers: vec![("Host".into(), "h".into())],
            body: b"b".to_vec(),
        };
        let wire = emit_request(&r);
        let text = String::from_utf8(wire.clone()).unwrap();
        assert!(text.starts_with("GET / HTTP/1\x2e1\r\n"));
        assert!(text.contains("Content-Length: 1\r\n"));
        assert!(text.ends_with("\r\nb"));
        let back = parse_request(&wire).unwrap();
        assert_eq!(back.body, b"b");
    }

    #[test]
    fn response_emit_and_set_header() {
        let mut r = Response {
            status: 404,
            reason: "Nope".into(),
            headers: vec![("X-A".into(), "1".into())],
            body: b"zz".to_vec(),
        };
        set_header(&mut r.headers, "x-a", "2"); // case-insensitive replace
        set_header(&mut r.headers, "X-B", "3"); // appended
        let wire = emit_response(&r);
        let text = String::from_utf8_lossy(&wire);
        assert!(text.starts_with("HTTP/1\x2e1 404 Nope\r\n"));
        assert!(text.contains("X-A: 2\r\n"));
        assert!(text.contains("X-B: 3\r\n"));
        assert!(text.contains("Content-Length: 2\r\n"));
        let back = parse_response(&wire).unwrap();
        assert_eq!(back.status, 404);
        assert_eq!(header(&back.headers, "x-a"), Some("2"));
        assert_eq!(header(&back.headers, "x-b"), Some("3"));
        assert_eq!(back.body, b"zz");
    }
}

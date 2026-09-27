//! SIP message parsing (RFC 3261).
//!
//! A message is a start line (`METHOD uri SIP/2.0` for requests or
//! `SIP/2.0 <code> <reason>` for responses), `Name: value` headers
//! (compact forms accepted raw), a blank line, then the
//! `Content-Length`-bounded body.
//!
//! ```
//! use izanagi_kit::sip;
//! let d = b"INVITE sip:bob@example.com SIP/2.0\r\nVia: SIP/2.0/UDP host\r\nTo: <sip:bob@e>\r\nContent-Length: 0\r\n\r\n";
//! let s = sip::parse(d).unwrap();
//! assert_eq!(s.method.as_deref(), Some("INVITE"));
//! assert_eq!(s.status_code, None);
//! ```

use std::collections::BTreeMap;
use std::string::String;

/// A parsed SIP message.
#[derive(Clone, Debug, PartialEq)]
pub struct Sip {
    /// Request method (`INVITE`, `REGISTER`, ...) — `None` for responses.
    pub method: Option<String>,
    /// Request URI.
    pub uri: String,
    /// Response status code — `None` for requests.
    pub status_code: Option<u16>,
    /// Reason phrase.
    pub reason: String,
    /// Header map (name → joined values).
    pub headers: BTreeMap<String, String>,
    /// `Content-Length` value (0 when absent).
    pub content_length: usize,
    /// Body offset.
    pub body_at: usize,
}

/// Parses a SIP message: start line classified request vs response,
/// headers until the blank line, `Content-Length` must fit.
pub fn parse(d: &[u8]) -> Option<Sip> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split("\r\n");
    let start = lines.next()?;
    if start.is_empty() {
        return None;
    }
    let (mut method, mut uri, mut status, mut reason) = (None, String::new(), None, String::new());
    if let Some(rest) = start.strip_prefix("SIP/2\x2e0 ") {
        let mut it = rest.splitn(2, ' ');
        status = Some(it.next()?.trim().parse::<u16>().ok()?);
        reason = it.next().unwrap_or("").to_string();
    } else {
        // request: METHOD uri SIP/2.0
        let (rest, ver) = start.rsplit_once(' ')?;
        if ver != "SIP/2\x2e0" {
            return None;
        }
        let mut it2 = rest.splitn(2, ' ');
        let m = it2.next()?;
        if m.is_empty() || !m.bytes().all(|b| b.is_ascii_alphabetic()) {
            return None;
        }
        method = Some(m.to_string());
        uri = it2.next().unwrap_or("").to_string();
    }
    let mut headers: BTreeMap<String, String> = BTreeMap::new();
    let mut consumed = start.len() + 2;
    let mut content_length = 0usize;
    let mut body_at = consumed;
    let mut found_blank = false;
    for line in lines {
        consumed += line.len() + 2;
        if line.is_empty() {
            body_at = consumed;
            found_blank = true;
            break;
        }
        let (k, v) = line.split_once(':')?;
        let k = k.trim().to_string();
        let v = v.trim().to_string();
        if k.eq_ignore_ascii_case("content-length") {
            content_length = v.parse().ok()?;
        }
        match headers.get_mut(&k) {
            Some(old) => {
                old.push_str(", ");
                old.push_str(&v);
            }
            None => {
                headers.insert(k, v);
            }
        }
    }
    if !found_blank && content_length > 0 {
        return None;
    }
    if body_at.checked_add(content_length)? > d.len() {
        return None;
    }
    Some(Sip {
        method,
        uri,
        status_code: status,
        reason,
        headers,
        content_length,
        body_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQ: &[u8] =
        b"REGISTER sip:reg.example SIP/2\x2e0\r\nVia: SIP/2\x2e0/UDP h\r\nContent-Length: 3\r\n\r\nabc";
    const RESP: &[u8] = b"SIP/2\x2e0 200 OK\r\nVia: SIP/2\x2e0/UDP h\r\nContent-Length: 0\r\n\r\n";

    #[test]
    fn parses_request() {
        let s = parse(REQ).unwrap();
        assert_eq!(s.method.as_deref(), Some("REGISTER"));
        assert_eq!(s.uri, "sip:reg.example");
        assert_eq!(s.content_length, 3);
        assert_eq!(
            s.headers.get("Via").map(|v| v.as_str()),
            Some("SIP/2\x2e0/UDP h")
        );
    }

    #[test]
    fn parses_response() {
        let s = parse(RESP).unwrap();
        assert_eq!(s.status_code, Some(200));
        assert_eq!(s.reason, "OK");
        assert!(s.method.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"garbage line\r\n\r\n").is_none());
        // Content-Length beyond end
        assert!(parse(b"INVITE sip:x SIP/2\x2e0\r\nContent-Length: 99\r\n\r\n").is_none());
    }
}

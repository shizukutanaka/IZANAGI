//! RTSP message parsing (RFC 2326).
//!
//! Same wire shape as SIP-style messages: request
//! `METHOD url RTSP/1.0` or response `RTSP/1.0 <code> <reason>`,
//! `Header: value` lines, blank line, optional `Content-Length`
//! body. `CSeq` is required by the spec and extracted.
//!
//! ```
//! use izanagi_kit::rtsp;
//! let d = b"DESCRIBE rtsp://cam/live RTSP/1.0\r\nCSeq: 3\r\nAccept: application/sdp\r\n\r\n";
//! let r = rtsp::parse(d).unwrap();
//! assert_eq!(r.method.as_deref(), Some("DESCRIBE"));
//! assert_eq!(r.cseq, Some(3));
//! ```

use std::collections::BTreeMap;
use std::string::String;

/// A parsed RTSP message.
#[derive(Clone, Debug, PartialEq)]
pub struct Rtsp {
    /// Request method — `None` for responses.
    pub method: Option<String>,
    /// Request URI.
    pub uri: String,
    /// Response status code — `None` for requests.
    pub status_code: Option<u16>,
    /// Reason phrase.
    pub reason: String,
    /// Header map.
    pub headers: BTreeMap<String, String>,
    /// `CSeq` sequence number.
    pub cseq: Option<u32>,
    /// `Content-Length`.
    pub content_length: usize,
    /// Body offset.
    pub body_at: usize,
}

/// Parses an RTSP message: `RTSP/1\x2e0` protocol tag on the start
/// line, headers until blank line, `Content-Length` must fit.
pub fn parse(d: &[u8]) -> Option<Rtsp> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split("\r\n");
    let start = lines.next()?;
    let (mut method, mut uri, mut status, mut reason) = (None, String::new(), None, String::new());
    if let Some(rest) = start.strip_prefix("RTSP/1\x2e0 ") {
        let mut it = rest.splitn(2, ' ');
        status = Some(it.next()?.trim().parse::<u16>().ok()?);
        reason = it.next().unwrap_or("").to_string();
    } else {
        let (rest, ver) = start.rsplit_once(' ')?;
        if ver != "RTSP/1\x2e0" {
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
    let mut cseq = None;
    let mut body_at = consumed;
    for line in lines {
        consumed += line.len() + 2;
        if line.is_empty() {
            body_at = consumed;
            break;
        }
        let (k, v) = line.split_once(':')?;
        let k = k.trim().to_string();
        let v = v.trim().to_string();
        if k.eq_ignore_ascii_case("content-length") {
            content_length = v.parse().ok()?;
        }
        if k.eq_ignore_ascii_case("cseq") {
            cseq = v.parse().ok();
        }
        headers.insert(k, v);
    }
    if body_at.checked_add(content_length)? > d.len() {
        return None;
    }
    Some(Rtsp {
        method,
        uri,
        status_code: status,
        reason,
        headers,
        cseq,
        content_length,
        body_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQ: &[u8] =
        b"SETUP rtsp://cam/track1 RTSP/1\x2e0\r\nCSeq: 7\r\nTransport: RTP/AVP;unicast\r\n\r\n";
    const RESP: &[u8] = b"RTSP/1\x2e0 200 OK\r\nCSeq: 7\r\nContent-Length: 4\r\n\r\nabcd";

    #[test]
    fn parses_request() {
        let r = parse(REQ).unwrap();
        assert_eq!(r.method.as_deref(), Some("SETUP"));
        assert_eq!(r.cseq, Some(7));
        assert_eq!(
            r.headers.get("Transport").map(|v| v.as_str()),
            Some("RTP/AVP;unicast")
        );
    }

    #[test]
    fn parses_response() {
        let r = parse(RESP).unwrap();
        assert_eq!(r.status_code, Some(200));
        assert_eq!(r.content_length, 4);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"HELLO\r\n\r\n").is_none());
        assert!(parse(b"SETUP u RTSP/2.0\r\n\r\n").is_none());
    }
}

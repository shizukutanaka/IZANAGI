//! MIME — RFC 2045/2046 message structure: header block, `Content-Type`
//! parameters, `multipart/*` boundary splitting, and
//! `Content-Transfer-Encoding` (`base64` / `quoted-printable` / `7bit`)
//! resolution. Total API — malformed input degrades to `None` or empty
//! parts rather than panicking.
//!
//! `http` covers the transport framing; `mime` covers the entity layer
//! mail and multipart uploads share.
//!
//! ```
//! use izanagi_kit::mime;
//! let m = mime::parse(b"Content-Type: text/plain; charset=utf-8\r\n\r\nhi").unwrap();
//! let (ty, params) = mime::content_type(&m);
//! assert_eq!(ty, "text/plain");
//! assert_eq!(params[0], ("charset".to_string(), "utf-8".to_string()));
//! ```

use crate::base64;

/// One MIME entity: header block plus raw body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// Header list in wire order.
    pub headers: Vec<(String, String)>,
    /// Raw body bytes (still transfer-encoded).
    pub body: Vec<u8>,
}

/// Split head/body at the first blank line (`\r\n\r\n` or `\n\n`).
pub fn parse(d: &[u8]) -> Option<Message> {
    let (head, body) = {
        let mut i = 0;
        let mut found = None;
        while i + 1 < d.len() {
            if d[i..].starts_with(b"\r\n\r\n") {
                found = Some((&d[..i], &d[i + 4..]));
                break;
            }
            i += 1;
        }
        if found.is_none() {
            let mut i = 0;
            while i + 1 < d.len() {
                if d[i] == b'\n' && d[i + 1] == b'\n' {
                    found = Some((&d[..i], &d[i + 2..]));
                    break;
                }
                i += 1;
            }
        }
        found?
    };
    let text = std::str::from_utf8(head).ok()?;
    let mut headers = Vec::new();
    let mut cur: Option<(String, String)> = None;
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // RFC 5322 folding IS legal in mail — unfold onto the value.
            let e = cur.as_mut()?;
            e.1.push(' ');
            e.1.push_str(line.trim());
            continue;
        }
        if let Some(e) = cur.take() {
            headers.push(e);
        }
        let c = line.find(':')?;
        let name = line[..c].trim();
        if name.is_empty() {
            return None;
        }
        cur = Some((name.to_string(), line[c + 1..].trim().to_string()));
    }
    if let Some(e) = cur.take() {
        headers.push(e);
    }
    Some(Message {
        headers,
        body: body.to_vec(),
    })
}

/// Case-insensitive header lookup; first match wins.
pub fn header<'a>(m: &'a Message, name: &str) -> Option<&'a str> {
    m.headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// `Content-Type` as (`"type/subtype"` lowercased, params).
/// Missing header → `("text/plain", [])` per RFC 2046 §5.1 default.
pub fn content_type(m: &Message) -> (String, Vec<(String, String)>) {
    let raw = match header(m, "content-type") {
        Some(v) => v,
        None => return ("text/plain".to_string(), Vec::new()),
    };
    let mut parts = raw.split(';');
    let ty = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let mut params = Vec::new();
    for p in parts {
        let p = p.trim();
        if let Some(eq) = p.find('=') {
            let (k, v) = (p[..eq].trim().to_ascii_lowercase(), p[eq + 1..].trim());
            let v = if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
                v[1..v.len() - 1].to_string()
            } else {
                v.to_string()
            };
            params.push((k, v));
        }
    }
    (ty, params)
}

/// `boundary` parameter of a multipart `Content-Type`.
pub fn boundary(m: &Message) -> Option<String> {
    content_type(m)
        .1
        .into_iter()
        .find(|(k, _)| k == "boundary")
        .map(|(_, v)| v)
}

/// Split a `multipart/*` body on its boundary; preamble and epilogue
/// are dropped. Each part parses as a `Message` — a part whose own
/// header block is malformed is skipped, not fatal.
pub fn parts(m: &Message) -> Vec<Message> {
    let b = match boundary(m) {
        Some(b) => b,
        None => return Vec::new(),
    };
    let dash = format!("--{}", b);
    let body = String::from_utf8_lossy(&m.body);
    let mut out = Vec::new();
    for seg in body.split(&dash) {
        let seg = seg
            .trim_start_matches(['\r', '\n'])
            .trim_end_matches(['\r', '\n']);
        // the closing delimiter `--boundary--` yields a segment still
        // starting with `--`; preamble/epilogue pieces end up empty or
        // unparseable and are skipped below.
        if seg.is_empty() || seg.starts_with('-') {
            continue;
        }
        if let Some(part) = parse(seg.as_bytes()) {
            out.push(part);
        }
    }
    out
}

/// `Content-Transfer-Encoding` resolution: `base64` (whitespace-tolerant),
/// `quoted-printable`, `7bit`/`8bit`/`binary` pass-through. Absent →
/// `7bit`. Unknown tokens → `None` (do not guess).
pub fn decode_transfer(body: &[u8], cte: Option<&str>) -> Option<Vec<u8>> {
    match cte.unwrap_or("7bit").to_ascii_lowercase().as_str() {
        "base64" => {
            let clean: Vec<u8> = body
                .iter()
                .filter(|c| !c.is_ascii_whitespace())
                .copied()
                .collect();
            base64::decode(std::str::from_utf8(&clean).ok()?)
        }
        "quoted-printable" => Some(quoted_printable_decode(body)),
        "7bit" | "8bit" | "binary" => Some(body.to_vec()),
        _ => None,
    }
}

/// Decode quoted-printable (RFC 2045 §6.7): `=HH` hex octets,
/// `=<CRLF>`/`=<LF>` soft line breaks, everything else literal.
pub fn quoted_printable_decode(d: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(d.len());
    let mut i = 0;
    let hexv = |c: u8| -> Option<u8> { (c as char).to_digit(16).map(|v| v as u8) };
    while i < d.len() {
        if d[i] == b'=' && i + 1 < d.len() {
            if d[i + 1] == b'\r' {
                if d.get(i + 2) == Some(&b'\n') {
                    i += 3;
                } else {
                    i += 2;
                }
                continue;
            }
            if d[i + 1] == b'\n' {
                i += 2;
                continue;
            }
            if i + 2 < d.len() {
                if let (Some(h), Some(l)) = (hexv(d[i + 1]), hexv(d[i + 2])) {
                    out.push((h << 4) | l);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(d[i]);
        i += 1;
    }
    out
}

/// Encode quoted-printable: printable ASCII (except `=`) and tab/space
/// pass literally; everything else becomes `=HH` uppercase; lines are
/// soft-broken before exceeding 75 columns (`=\r\n`).
pub fn quoted_printable_encode(d: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(d.len() + d.len() / 3);
    let mut col = 0usize;
    for (i, &b) in d.iter().enumerate() {
        let last_on_line = i + 1 == d.len() || d[i + 1] == b'\r' || d[i + 1] == b'\n';
        // CR and LF in the *data* are encoded like any other unsafe
        // byte — a bare CRLF in output only ever means a soft break.
        let literal =
            (b == b'\t' || b == b' ') && !last_on_line || (33..=126).contains(&b) && b != b'=';
        let cost = if literal { 1 } else { 3 };
        if col + cost > 74 {
            out.extend_from_slice(b"=\r\n");
            col = 0;
        }
        if literal {
            out.push(b);
        } else {
            out.push(b'=');
            out.push(b"0123456789ABCDEF"[(b >> 4) as usize]);
            out.push(b"0123456789ABCDEF"[(b & 15) as usize]);
        }
        col += cost;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_headers() {
        let m = parse(b"Subject: hi there\r\n  folded\r\nX-A: 1\r\n\r\nbody\r\n").unwrap();
        assert_eq!(header(&m, "subject"), Some("hi there folded"));
        assert_eq!(header(&m, "x-a"), Some("1"));
        assert_eq!(m.body, b"body\r\n");
        assert!(parse(b"no colon\r\n\r\nb").is_none());
        assert!(parse(b"").is_none()); // no blank line at all
    }

    #[test]
    fn content_type_params() {
        let m = parse(b"Content-Type: Multipart/Mixed; BOUNDARY=\"==a==\"\r\n\r\n").unwrap();
        let (ty, params) = content_type(&m);
        assert_eq!(ty, "multipart/mixed");
        assert_eq!(params, vec![("boundary".to_string(), "==a==".to_string())]);
        assert_eq!(boundary(&m).as_deref(), Some("==a=="));
        let plain = parse(b"X: 1\r\n\r\n").unwrap();
        assert_eq!(content_type(&plain).0, "text/plain");
    }

    #[test]
    fn multipart_split() {
        let m = parse(
            b"Content-Type: multipart/mixed; boundary=B\r\n\r\npreamble\r\n--B\r\nA: 1\r\n\r\none\r\n--B\r\nContent-Type: text/html\r\n\r\n<b>two</b>\r\n--B--\r\nepilogue",
        )
        .unwrap();
        let ps = parts(&m);
        assert_eq!(ps.len(), 2);
        assert_eq!(header(&ps[0], "a"), Some("1"));
        assert_eq!(ps[0].body, b"one");
        assert_eq!(ps[1].body, b"<b>two</b>");
        assert!(parts(&parse(b"X:1\r\n\r\n").unwrap()).is_empty());
    }

    #[test]
    fn transfer_encodings() {
        assert_eq!(
            decode_transfer(b"aGV sbG8=", Some("base64")).unwrap(),
            b"hello"
        );
        assert_eq!(decode_transfer(b"ab", Some("7bit")).unwrap(), b"ab");
        assert_eq!(decode_transfer(b"ab", None).unwrap(), b"ab");
        assert!(decode_transfer(b"ab", Some("x-unknown")).is_none());
        assert!(decode_transfer(b"===", Some("base64")).is_none());
    }

    #[test]
    fn quoted_printable() {
        assert_eq!(quoted_printable_decode(b"a=3Db"), b"a=b");
        assert_eq!(quoted_printable_decode(b"a=\r\nb"), b"ab");
        assert_eq!(quoted_printable_decode(b"a=\nb"), b"ab");
        assert_eq!(quoted_printable_decode(b"=C3=A9"), "\u{e9}".as_bytes());
        assert_eq!(quoted_printable_decode(b"trailing="), b"trailing=");
        // roundtrip: arbitrary bytes survive
        let src: Vec<u8> = (0u8..=255).collect();
        let enc = quoted_printable_encode(&src);
        assert_eq!(quoted_printable_decode(&enc), src);
        // lines stay <= 75 chars
        for line in String::from_utf8_lossy(&enc).split("\r\n") {
            assert!(line.len() <= 75);
        }
    }
}

//! roff/man page requests: `.XX args` and `'XX args` control lines, plain text
//! otherwise (groff_man(7)). Macro names are the 1-2 chars after the dot.
//!
//! ```
//! use izanagi_kit::roff::parse;
//!
//! let d = b".TH CMD 1\n.SH NAME\ncmd - does things\n.B bold\n";
//! let r = parse(d).unwrap();
//! assert_eq!(r.requests.len(), 3);
//! assert_eq!(r.requests[0].mac, "TH");
//! assert_eq!(r.requests[1].args, "NAME");
//! assert_eq!(r.text_lines, 1);
//! ```

/// One control line.
#[derive(Debug, Clone)]
pub struct Req {
    /// Macro name (`TH`, `SH`, `B`, `IP`, ...).
    pub mac: String,
    /// Arguments verbatim.
    pub args: String,
}

/// Parsed roff input.
#[derive(Debug, Clone)]
pub struct Roff {
    /// Control requests in order.
    pub requests: Vec<Req>,
    /// Non-control, non-empty line count.
    pub text_lines: usize,
}

/// Parse roff input: `.xx` / `'xx` lines are requests; `.\"` is a comment.
pub fn parse(data: &[u8]) -> Option<Roff> {
    let text = std::str::from_utf8(data).ok()?;
    let mut requests = Vec::new();
    let mut text_lines = 0usize;
    for line in text.lines() {
        let t = line;
        if t.is_empty() {
            continue;
        }
        let first = t.as_bytes()[0];
        if first == b'.' || first == b'\'' {
            let b = &t.as_bytes()[1..];
            if b.starts_with(b"\\\"") {
                continue; // .\" comment
            }
            // macro name: letters/digits until whitespace
            let mut i = 0usize;
            while i < b.len() && b[i].is_ascii_alphanumeric() {
                i += 1;
            }
            if i == 0 {
                return None;
            }
            let mac = String::from_utf8_lossy(&b[..i]).into_owned();
            let args = String::from_utf8_lossy(&b[i..]).trim().to_string();
            requests.push(Req { mac, args });
        } else {
            text_lines += 1;
        }
    }
    Some(Roff {
        requests,
        text_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b".\\\" comment\n.TH X 1\n.SH NAME\nx \\- y\n.IP \\(bu\nitem\n";
        let r = parse(d).unwrap();
        assert_eq!(r.requests.len(), 3);
        assert_eq!(r.requests[2].mac, "IP");
        assert_eq!(r.text_lines, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b". no name\n").is_none());
    }
}

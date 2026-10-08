//! SWIFT MT message parser — the `{1:}/{2:}/{4:}/{5:}` block structure used by
//! all FIN category messages (MT103 customer transfer, MT202 cover, MT940
//! statement, …). Block 1 `basic` identifies application/service/LT; block 2
//! carries direction + 3-digit message type + receiver LT address; block 4 is
//! the `:TAG:VALUE` field list where values may continue on following lines;
//! block 5 is the `{CHK:…}`/`{MAC:…}` trailer.
//!
//! ```
//! let d = b"{1:F01BANKBEBBAXXX0000000000}{2:I103BANKDEFFXXXXN}{4:\r\n:20:REF001\r\n:23B:CRED\r\n:32A:260101EUR1000,00\r\n:50K:ORDERING CO\r\n:59:BE12345\r\nBENEFICIARY\r\n-}{5:{CHK:112233445566}}";
//! let m = izanagi_kit::swiftmt::parse(d).unwrap();
//! assert_eq!(m.msg_type, "103");
//! assert_eq!(m.sender_lt.as_deref(), Some("BANKBEBBAXXX"));
//! assert_eq!(m.receiver_addr.as_deref(), Some("BANKDEFFXXXX"));
//! assert_eq!(m.tag("20"), Some("REF001"));
//! assert_eq!(m.fields.len(), 5);
//! assert_eq!(m.trailer_tags.len(), 1);
//! assert!(izanagi_kit::swiftmt::detect(d));
//! ```

/// A parsed SWIFT MT message.
#[derive(Debug)]
pub struct SwiftMt {
    /// 3-digit message type from block 2 (e.g. `103`, `940`).
    pub msg_type: String,
    /// `I` (input to the network) or `O` (output) from block 2.
    pub direction: Option<char>,
    /// 12-char logical-terminal address found in the basic block.
    pub sender_lt: Option<String>,
    /// 12-char receiver LT address for input (`I`) messages.
    pub receiver_addr: Option<String>,
    /// `:TAG:VALUE` pairs from the text block, in order; multi-line values are
    /// joined with `\n`.
    pub fields: Vec<(String, String)>,
    /// Trailer sub-block tag names (`CHK`, `MAC`, `PDE`, …).
    pub trailer_tags: Vec<String>,
}

impl SwiftMt {
    /// First value of `tag` in the text block.
    #[must_use]
    pub fn tag(&self, tag: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(t, _)| t == tag)
            .map(|(_, v)| v.as_str())
    }
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// A `{1:…}` / `{4:…}`-block stream starts a SWIFT MT message.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let s = strip_bom(s);
    let s = s.trim_start();
    s.starts_with("{1:") && s.contains("{4:")
}

fn block<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    let j = s[i..].find(close).map(|k| i + k).unwrap_or(i);
    s.get(i..j)
}

/// Parses the message; `None` if the basic/app blocks are missing.
#[must_use]
pub fn parse(b: &[u8]) -> Option<SwiftMt> {
    let s = core::str::from_utf8(b).ok()?;
    let s = strip_bom(s);
    let basic = block(s, "{1:", "}")?;
    // F01 + 12-char LT + 4 session + 6 sequence
    let sender_lt = basic.get(3..15).map(str::to_string);
    let app = block(s, "{2:", "}")?;
    let direction = app.chars().next().filter(|c| matches!(c, 'I' | 'O'));
    if app.len() < 4 || !app.as_bytes()[1..4].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let msg_type = app[1..4].to_string();
    let receiver_addr = app
        .get(4..16)
        .filter(|_| direction == Some('I'))
        .map(str::to_string);
    let text = block(s, "{4:", "-}")?;
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let t = line.trim_end();
        if let Some(rest) = t.strip_prefix(':') {
            if let Some((tag, val)) = rest.split_once(':') {
                if !tag.is_empty() && tag.len() <= 4 {
                    fields.push((tag.to_string(), val.to_string()));
                }
            } else if let Some(last) = fields.last_mut() {
                last.1.push('\n');
                last.1.push_str(t);
            }
        } else if let Some(last) = fields.last_mut() {
            last.1.push('\n');
            last.1.push_str(t);
        }
    }
    let trailer_tags = match block(s, "{5:", "}") {
        Some(t) => {
            let mut v = Vec::new();
            let mut rest = t;
            while let Some(i) = rest.find('{') {
                rest = &rest[i + 1..];
                if let Some(colon) = rest.find(':') {
                    let name = rest[..colon].to_string();
                    v.push(name);
                    rest = &rest[colon..];
                } else {
                    break;
                }
            }
            v
        }
        None => Vec::new(),
    };
    Some(SwiftMt {
        msg_type,
        direction,
        sender_lt,
        receiver_addr,
        fields,
        trailer_tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = b"{1:F01BANKBEBBAXXX0000000000}{2:I103BANKDEFFXXXXN}{4:\r\n:20:REF001\r\n:23B:CRED\r\n:32A:260101EUR1000,00\r\n:50K:ORDERING CO\r\n:59:BE12345\r\nBENEFICIARY\r\n-}{5:{CHK:112233445566}}";

    #[test]
    fn detect_works() {
        assert!(detect(FIXTURE));
        assert!(!detect(b"swift mt103"));
        assert!(!detect(&[0xff, 0x00]));
    }

    #[test]
    fn parses_blocks() {
        let m = parse(FIXTURE).unwrap();
        assert_eq!(m.msg_type, "103");
        assert_eq!(m.direction, Some('I'));
        assert_eq!(m.sender_lt.as_deref(), Some("BANKBEBBAXXX"));
        assert_eq!(m.receiver_addr.as_deref(), Some("BANKDEFFXXXX"));
        assert_eq!(m.tag("23B"), Some("CRED"));
        assert_eq!(m.tag("32A"), Some("260101EUR1000,00"));
        assert_eq!(m.tag("59"), Some("BE12345\nBENEFICIARY"));
        assert_eq!(m.tag("99"), None);
        assert_eq!(m.trailer_tags, vec!["CHK".to_string()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"{4::20:X-}").is_none());
        assert!(parse(b"{1:X}{2:Y}{4::-}").is_none());
    }

    #[test]
    fn handles_multibyte_fixed_offsets() {
        let _ = parse("{1:\u{1d11e}F0000000000}".as_bytes());
        let _ = parse("{1:FFééééééé}{2:I103BANKDEFFXXXXN}{4:\r\n:20:X\r\n-}".as_bytes());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

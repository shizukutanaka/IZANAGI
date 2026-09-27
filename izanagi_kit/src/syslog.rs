//! Syslog messages: RFC 3164 BSD (`<PRI>Mon dd hh:mm:ss host tag: msg`) and
//! RFC 5424 (`<PRI>VERSION ts host app pid msgid [sd] msg`).
//!
//! ```
//! use izanagi_kit::syslog::parse_line;
//!
//! let m = parse_line(b"<34>Oct 11 22:14:15 myhost su: hi").unwrap();
//! assert_eq!(m.severity, 2); // crit
//! assert_eq!(m.facility, 4); // auth
//! let n = parse_line(b"<165>1 2003-10-11T22:14:15Z host app 42 ID47 - msg").unwrap();
//! assert_eq!(n.version, Some(1));
//! ```

/// One parsed syslog message.
#[derive(Debug, Clone)]
pub struct Message {
    /// Raw PRI value.
    pub pri: u16,
    /// `pri >> 3`.
    pub facility: u16,
    /// `pri & 7`.
    pub severity: u16,
    /// RFC 5424 version digit (`None` for 3164).
    pub version: Option<u16>,
    /// Timestamp / date field verbatim (`-` allowed).
    pub timestamp: String,
    /// Hostname field.
    pub host: String,
    /// Tag (3164) or app-name (5424).
    pub tag: String,
    /// Message text.
    pub message: String,
}

fn pri(d: &[u8]) -> Option<(u16, usize)> {
    if d.first() != Some(&b'<') {
        return None;
    }
    let end = d.iter().position(|&b| b == b'>')?;
    if !(2..=5).contains(&end) {
        return None;
    }
    let mut v: u16 = 0;
    for &b in &d[1..end] {
        if !b.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add(u16::from(b - b'0'))?;
    }
    if v > 191 {
        return None;
    }
    Some((v, end + 1))
}

fn tok(d: &[u8], at: usize) -> Option<(String, usize)> {
    let mut i = at;
    while i < d.len() && d[i] == b' ' {
        i += 1;
    }
    let start = i;
    while i < d.len() && d[i] != b' ' {
        i += 1;
    }
    if i == start {
        return None;
    }
    Some((String::from_utf8_lossy(&d[start..i]).into_owned(), i))
}

/// Parse one syslog line (3164 or 5424, auto-detected by a digit after `>`).
pub fn parse_line(d: &[u8]) -> Option<Message> {
    let (p, mut at) = pri(d)?;
    let version = if d.get(at).is_some_and(|b| b.is_ascii_digit()) && d.get(at + 1) == Some(&b' ') {
        let v = u16::from(d[at] - b'0');
        at += 2;
        Some(v)
    } else {
        None
    };
    if version.is_some() {
        // 5424: TIMESTAMP HOSTNAME APP-NAME PROCID MSGID [SD] MSG
        let (ts, a) = tok(d, at)?;
        let (host, a) = tok(d, a)?;
        let (app, a) = tok(d, a)?;
        let (_procid, a) = tok(d, a)?;
        let (_msgid, a) = tok(d, a)?;
        let mut i = a;
        while i < d.len() && d[i] == b' ' {
            i += 1;
        }
        if d.get(i) == Some(&b'[') {
            // structured data: skip to the closing `]` (no escapes inside)
            let end = d[i..].iter().position(|&b| b == b']')?;
            i += end + 1;
            while i < d.len() && d[i] == b' ' {
                i += 1;
            }
        } else if d.get(i) == Some(&b'-') {
            i += 1;
            while i < d.len() && d[i] == b' ' {
                i += 1;
            }
        }
        Some(Message {
            pri: p,
            facility: p >> 3,
            severity: p & 7,
            version,
            timestamp: ts,
            host,
            tag: app,
            message: String::from_utf8_lossy(d.get(i..)?).into_owned(),
        })
    } else {
        // 3164: "Mon dd hh:mm:ss" then host then "tag: msg"
        let (a, i1) = tok(d, at)?; // month
        let (b, i2) = tok(d, i1)?; // day
        let (c, i3) = tok(d, i2)?; // time
        let (host, i4) = tok(d, i3)?;
        let mut i = i4;
        while i < d.len() && d[i] == b' ' {
            i += 1;
        }
        let rest = d.get(i..)?;
        let tag_end = rest.iter().position(|&x| x == b':').unwrap_or(rest.len());
        let tag = String::from_utf8_lossy(&rest[..tag_end]).into_owned();
        let msg_at = if tag_end < rest.len() {
            tag_end + 1
        } else {
            tag_end
        };
        let mut j = msg_at;
        while j < rest.len() && rest[j] == b' ' {
            j += 1;
        }
        Some(Message {
            pri: p,
            facility: p >> 3,
            severity: p & 7,
            version,
            timestamp: format!("{a} {b} {c}"),
            host,
            tag,
            message: String::from_utf8_lossy(&rest[j..]).into_owned(),
        })
    }
}

/// Parse a multi-line syslog buffer, skipping blank lines.
pub fn parse(data: &[u8]) -> Option<Vec<Message>> {
    let text = std::str::from_utf8(data).ok()?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        out.push(parse_line(line.as_bytes())?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bsd_3164() {
        let m = parse_line(b"<13>Jan  1 00:00:01 box cron[9]: ran").unwrap();
        assert_eq!(m.version, None);
        assert_eq!(m.host, "box");
        assert_eq!(m.tag, "cron[9]");
        assert_eq!(m.message, "ran");
    }

    #[test]
    fn ietf_5424() {
        let m = parse_line(b"<191>1 - host app 7 ID1 [x y=\"z\"] body").unwrap();
        assert_eq!(m.version, Some(1));
        assert_eq!(m.tag, "app");
        assert_eq!(m.message, "body");
    }

    #[test]
    fn rejects() {
        assert!(parse_line(b"no pri").is_none());
        assert!(parse_line(b"<999>x").is_none());
        assert!(parse(b"ok<0> bad\n").is_none());
    }
}

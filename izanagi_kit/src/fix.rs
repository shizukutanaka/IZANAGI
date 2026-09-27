//! FIX protocol message parsing (FIX 4.x/5.x session layer).
//!
//! Fields are `tag=value` pairs separated by `0x01` (SOH). The body
//! must open with `8=FIX.x.y` and close with `10=nnn` where `nnn`
//! is the byte sum of everything before the `10=` field mod 256.
//! Tag `35` carries the message type.
//!
//! ```
//! use izanagi_kit::fix;
//! // 8=FIX.4.2 | 35=A | 10=checksum
//! let body = b"8=FIX.4.2\x0135=A\x01";
//! let mut m = body.to_vec();
//! let sum = m.iter().map(|&b| b as u32).sum::<u32>() % 256;
//! m.extend_from_slice(format!("10={sum:03}\x01").as_bytes());
//! let f = fix::parse(&m).unwrap();
//! assert_eq!(f.msg_type.as_deref(), Some("A"));
//! ```

use std::collections::BTreeMap;
use std::string::String;

/// A parsed FIX message.
#[derive(Clone, Debug, PartialEq)]
pub struct Fix {
    /// `BeginString` — e.g. `FIX.4.2`, `FIX.4.4`, `FIXT.1.1`.
    pub begin: String,
    /// `MsgType` (tag 35).
    pub msg_type: Option<String>,
    /// All fields: `tag → value` (later duplicates overwrite).
    pub fields: BTreeMap<u32, String>,
    /// Declared `CheckSum` (tag 10).
    pub checksum: u32,
    /// Number of tag=value pairs.
    pub count: usize,
}

/// Parses one FIX message: `8=FIX…` head, `tag=value\x01` fields,
/// `10=nnn` trailer whose value must equal the byte sum mod 256 of
/// everything before it.
pub fn parse(d: &[u8]) -> Option<Fix> {
    let soh = 0x01u8;
    // locate the `10=` trailer start: last `10=` field
    let mut trailer_at = None;
    for i in 0..d.len() {
        if (i == 0 || d[i - 1] == soh) && d.get(i..i.checked_add(3)?) == Some(b"10=") {
            trailer_at = Some(i);
        }
    }
    let t_at = trailer_at?;
    let sum: u32 = d[..t_at].iter().map(|&b| b as u32).sum::<u32>() % 256;
    let trailer = d.get(t_at + 3..)?;
    let val_end = trailer.iter().position(|&b| b == soh)?;
    let checksum: u32 = std::str::from_utf8(trailer.get(..val_end)?)
        .ok()?
        .parse()
        .ok()?;
    if checksum != sum {
        return None;
    }
    let mut fields = BTreeMap::new();
    let mut begin = None;
    let mut msg_type = None;
    let mut count = 0usize;
    for field in d[..t_at].split(|&b| b == soh) {
        if field.is_empty() {
            continue;
        }
        let eq = field.iter().position(|&b| b == b'=')?;
        let tag: u32 = std::str::from_utf8(field.get(..eq)?).ok()?.parse().ok()?;
        let value = String::from_utf8_lossy(field.get(eq + 1..)?).into_owned();
        match tag {
            8 => {
                if begin.is_some() {
                    return None;
                }
                if !(value.starts_with("FIX.") || value.starts_with("FIXT.")) {
                    return None;
                }
                begin = Some(value.clone());
            }
            35 => msg_type = Some(value.clone()),
            _ => {}
        }
        fields.insert(tag, value);
        count += 1;
    }
    Some(Fix {
        begin: begin?,
        msg_type,
        fields,
        checksum,
        count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;

    fn msg(fields: &[&[u8]]) -> Vec<u8> {
        let mut d = Vec::new();
        for f in fields {
            d.extend_from_slice(f);
            d.push(0x01);
        }
        let sum = d.iter().map(|&b| b as u32).sum::<u32>() % 256;
        d.extend_from_slice(format!("10={sum:03}").as_bytes());
        d.push(0x01);
        d
    }

    #[test]
    fn parses_message() {
        let m = msg(&[b"8=FIX.4.4", b"35=D", b"49=SENDER", b"55=AAPL", b"40=1"]);
        let f = parse(&m).unwrap();
        assert_eq!(f.begin, "FIX.4.4");
        assert_eq!(f.msg_type.as_deref(), Some("D"));
        assert_eq!(f.fields.get(&55).map(|v| v.as_str()), Some("AAPL"));
        assert_eq!(f.count, 5);
    }

    #[test]
    fn rejects() {
        // broken checksum
        let m = b"8=FIX.4.2\x0135=A\x0110=000\x01";
        assert!(parse(m).is_none());
        // no trailer
        assert!(parse(b"8=FIX.4.2\x0135=A\x01").is_none());
        // no begin
        assert!(parse(&msg(&[b"35=A"])).is_none());
    }
}

//! `candump` log lines (can-utils): `(ts) iface id [dlc] data...`.
//!
//! Two accepted shapes:
//! `(1669134731.000000) can0 123 [3] 11 22 33` (logged) and
//! `can0  123   [3] 11 22 33` (live `-L` style without timestamp).
//!
//! ```
//! use izanagi_kit::candump::parse_line;
//!
//! let f = parse_line("(1.5) can0 123 [2] AB CD").unwrap();
//! assert_eq!(f.id, 0x123);
//! assert_eq!(f.data, vec![0xAB, 0xCD]);
//! ```

/// One parsed candump line.
#[derive(Clone, Debug)]
pub struct CanFrame {
    /// Optional `(seconds.usec)` timestamp, verbatim text.
    pub timestamp: Option<String>,
    /// Interface name (`can0`, `vcan0`…).
    pub interface: String,
    /// 11- or 29-bit CAN id.
    pub id: u32,
    /// Extended-frame flag (`##` remote/extended marker or long id).
    pub extended: bool,
    /// Data bytes.
    pub data: Vec<u8>,
}

/// Parse one candump line. `None` on missing interface/id/bracket or
/// malformed hex.
pub fn parse_line(line: &str) -> Option<CanFrame> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    let (timestamp, rest) = if let Some(r) = t.strip_prefix('(') {
        let e = r.find(')')?;
        (Some(r[..e].to_string()), r[e + 1..].trim())
    } else {
        (None, t)
    };
    let mut it = rest.split_whitespace();
    let interface = it.next()?.to_string();
    let id_tok = it.next()?;
    // RTR `id#R` and extended `id#data` compact forms
    let (id, data) = if let Some((i, d)) = id_tok.split_once('#') {
        let id = u32::from_str_radix(i, 16).ok()?;
        let mut bytes = Vec::new();
        let mut j = 0;
        while j + 2 <= d.len() {
            bytes.push(u8::from_str_radix(&d[j..j + 2], 16).ok()?);
            j += 2;
        }
        (id, bytes)
    } else {
        let id = u32::from_str_radix(id_tok, 16).ok()?;
        let dlc = it.next()?;
        if !dlc.starts_with('[') || !dlc.ends_with(']') {
            return None;
        }
        let mut bytes = Vec::new();
        for b in it {
            bytes.push(u8::from_str_radix(b, 16).ok()?);
        }
        (id, bytes)
    };
    let extended = id > 0x7FF || id_tok.contains("##");
    Some(CanFrame {
        timestamp,
        interface,
        id,
        extended,
        data,
    })
}

/// Parse a whole candump log (one frame per line).
pub fn parse(d: &[u8]) -> Option<Vec<CanFrame>> {
    let text = std::str::from_utf8(d).ok()?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(parse_line(line)?);
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let f = parse_line("(1669134731.000000) can0 123 [3] 11 22 33").unwrap();
        assert_eq!(f.timestamp.as_deref(), Some("1669134731.000000"));
        assert_eq!(f.id, 0x123);
        assert_eq!(f.data, vec![0x11, 0x22, 0x33]);
        let rtr = parse_line("vcan0  7DF#R").unwrap();
        assert_eq!(rtr.id, 0x7DF);
        assert!(rtr.data.is_empty());
        let ext = parse_line("can0 18FF50E5 [2] AA BB").unwrap();
        assert!(ext.extended);
        let multi = parse(b"(0.1) can0 1 [1] 00\ncan0 2 [1] FF\n").unwrap();
        assert_eq!(multi.len(), 2);
    }

    #[test]
    fn rejects() {
        assert!(parse_line("").is_none());
        assert!(parse_line("can0").is_none());
        assert!(parse_line("can0 ZZZ [1] 00").is_none());
        assert!(parse(b"").is_none());
        assert!(parse(b"bad line\n").is_none());
    }
}

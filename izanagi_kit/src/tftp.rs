//! TFTP packet parser (RFC 1350, with RFC 2347 option negotiation).
//!
//! Every packet starts with a 16-bit opcode: `1` RRQ, `2` WRQ,
//! `3` DATA, `4` ACK, `5` ERROR, `6` OACK. RRQ/WRQ carry a
//! NUL-terminated filename and mode (`octet`/`netascii`/`mail`);
//! OACK carries `opt\0value\0` pairs. DATA/ACK carry a block number
//! (modulo 2^16, wraps). ERROR carries a code and message.
//!
//! ```
//! let f = b"\x00\x01file.txt\x00octet\x00";
//! let t = izanagi_kit::tftp::parse(f).unwrap();
//! assert_eq!(t.opcode, 1);
//! assert_eq!(t.filename.as_deref(), Some("file.txt"));
//! assert_eq!(t.mode.as_deref(), Some("octet"));
//! ```

/// TFTP opcode (RRQ, WRQ, DATA, ACK, ERROR, OACK).
#[derive(Debug, Clone, PartialEq)]
pub struct Tftp {
    /// Opcode word (`1`–`6`).
    pub opcode: u16,
    /// RRQ/WRQ filename, or `None` for DATA/ACK/ERROR packets.
    pub filename: Option<String>,
    /// RRQ/WRQ mode string as transmitted (lower-case on the wire).
    pub mode: Option<String>,
    /// DATA/ACK block number (`u16`), `None` otherwise.
    pub block: Option<u16>,
    /// ERROR code (`u16`), `None` otherwise.
    pub error_code: Option<u16>,
    /// OACK option names and values (`(name, value)` pairs).
    pub options: Vec<(String, String)>,
    /// DATA payload byte length (`0`–`512`; short packet = last block).
    pub data_len: usize,
}

fn cstr(s: &[u8], from: usize) -> Option<(usize, usize)> {
    // returns (end_exclusive, text_start)
    if from >= s.len() {
        return None;
    }
    let rel = s[from..].iter().position(|&b| b == 0)?;
    Some((from + rel + 1, from))
}

/// Parse a TFTP packet; `None` for unknown opcodes or malformed fields.
pub fn parse(d: &[u8]) -> Option<Tftp> {
    if d.len() < 2 {
        return None;
    }
    let opcode = ((d[0] as u16) << 8) | d[1] as u16;
    let mut t = Tftp {
        opcode,
        filename: None,
        mode: None,
        block: None,
        error_code: None,
        options: Vec::new(),
        data_len: 0,
    };
    match opcode {
        1 | 2 => {
            // `file\0mode\0` — both strings required.
            let (e1, s1) = cstr(d, 2)?;
            let name = std::str::from_utf8(&d[s1..e1 - 1]).ok()?;
            if name.is_empty() {
                return None;
            }
            let (e2, s2) = cstr(d, e1)?;
            let mode = std::str::from_utf8(&d[s2..e2 - 1]).ok()?;
            if mode.is_empty() {
                return None;
            }
            t.filename = Some(name.to_string());
            t.mode = Some(mode.to_string());
            if e2 != d.len() {
                // trailing option strings (RFC 2347 negotiation) — parse
                // as name\0value\0 pairs like OACK.
                let mut i = e2;
                while i < d.len() {
                    let (en, sn) = cstr(d, i)?;
                    let (ev, sv) = cstr(d, en)?;
                    let n = std::str::from_utf8(&d[sn..en - 1]).ok()?;
                    let v = std::str::from_utf8(&d[sv..ev - 1]).ok()?;
                    t.options.push((n.to_string(), v.to_string()));
                    i = ev;
                }
            }
        }
        3 => {
            if d.len() < 4 {
                return None;
            }
            t.block = Some(((d[2] as u16) << 8) | d[3] as u16);
            t.data_len = d.len() - 4;
        }
        4 => {
            if d.len() != 4 {
                return None;
            }
            t.block = Some(((d[2] as u16) << 8) | d[3] as u16);
        }
        5 => {
            if d.len() < 5 {
                return None;
            }
            t.error_code = Some(((d[2] as u16) << 8) | d[3] as u16);
            let (e, s) = cstr(d, 4)?;
            t.filename = Some(std::str::from_utf8(&d[s..e - 1]).ok()?.to_string());
        }
        6 => {
            let mut i = 2usize;
            if i == d.len() {
                return None;
            }
            while i < d.len() {
                let (en, sn) = cstr(d, i)?;
                let (ev, sv) = cstr(d, en)?;
                let n = std::str::from_utf8(&d[sn..en - 1]).ok()?;
                let v = std::str::from_utf8(&d[sv..ev - 1]).ok()?;
                t.options.push((n.to_string(), v.to_string()));
                i = ev;
            }
        }
        _ => return None,
    }
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rrq() {
        let f = b"\x00\x01file.txt\x00octet\x00";
        let t = parse(f).unwrap();
        assert_eq!(t.opcode, 1);
        assert_eq!(t.filename.as_deref(), Some("file.txt"));
        assert_eq!(t.mode.as_deref(), Some("octet"));
        assert!(t.options.is_empty());
    }

    #[test]
    fn wrq_with_options() {
        let f = b"\x00\x02a\x00netascii\x00blksize\x00512\x00";
        let t = parse(f).unwrap();
        assert_eq!(t.opcode, 2);
        assert_eq!(t.options.len(), 1);
        assert_eq!(t.options[0].0, "blksize");
    }

    #[test]
    fn data_ack_error_oack() {
        let mut d = b"\x00\x03\x00\x01".to_vec();
        d.resize(4 + 512, 0xAB);
        let t = parse(&d).unwrap();
        assert_eq!(t.block, Some(1));
        assert_eq!(t.data_len, 512);

        let t = parse(b"\x00\x04\x00\x2a").unwrap();
        assert_eq!(t.block, Some(42));

        let t = parse(b"\x00\x05\x00\x02no such file\x00").unwrap();
        assert_eq!(t.error_code, Some(2));
        assert_eq!(t.filename.as_deref(), Some("no such file"));

        let t = parse(b"\x00\x06tsize\x001024\x00").unwrap();
        assert_eq!(t.options, vec![("tsize".to_string(), "1024".to_string())]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x00").is_none());
        assert!(parse(b"\x00\x09xx").is_none()); // unknown opcode
        assert!(parse(b"\x00\x01\x00").is_none()); // empty filename
        assert!(parse(b"\x00\x01f\x00").is_none()); // missing mode
        assert!(parse(b"\x00\x04\x00").is_none()); // truncated ack
    }
}

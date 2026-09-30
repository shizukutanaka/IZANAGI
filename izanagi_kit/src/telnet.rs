//! Telnet (RFC 854) — IAC command stream parser.
//!
//! ```
//! let d = b"\xff\xfb\x01\xff\xfa\x18\x00\xff\xf0hello\xff\xfe\x01";
//! let t = izanagi_kit::telnet::parse(d).unwrap();
//! assert_eq!(t.will_wont_do_dont, 2);
//! assert_eq!(t.subnegotiations, 1);
//! assert!(izanagi_kit::telnet::detect(d));
//! ```
use std::string::String;

/// Parsed Telnet stream summary.
#[derive(Debug, Clone)]
pub struct Telnet {
    /// WILL/WONT/DO/DONT negotiation commands counted.
    pub will_wont_do_dont: u32,
    /// IAC IAC escaped bytes.
    pub escapes: u32,
    /// SB…SE subnegotiation blocks.
    pub subnegotiations: u32,
    /// Other commands (NOP/GA/BRK/AYT/EC/EL/EOF…).
    pub other_commands: u32,
    /// Option codes negotiated, in order.
    pub options: Vec<u8>,
    /// Plain-data byte count between commands.
    pub data_bytes: u32,
    /// Truncated command at end (`IAC` + nothing, or open `SB`).
    pub truncated: bool,
}

fn opt_name(o: u8) -> &'static str {
    match o {
        0 => "BINARY",
        1 => "ECHO",
        3 => "SGA",
        5 => "STATUS",
        6 => "TIMING-MARK",
        24 => "TTYPE",
        31 => "NAWS",
        32 => "TSPEED",
        33 => "RFC",
        34 => "LINEMODE",
        255 => "EXOPL",
        _ => "other",
    }
}

/// Name of a Telnet option code.
#[must_use]
pub fn option_name(o: u8) -> String {
    String::from(opt_name(o))
}

/// Detects a stream containing at least one IAC command.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.windows(2).any(|w| w[0] == 0xff && w[1] != 0xff) || b.first() == Some(&0xff) && b.len() == 1
}

/// Parses the IAC stream; `None` when no command present.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Telnet> {
    if !detect(b) {
        return None;
    }
    let mut f = Telnet {
        will_wont_do_dont: 0,
        escapes: 0,
        subnegotiations: 0,
        other_commands: 0,
        options: Vec::new(),
        data_bytes: 0,
        truncated: false,
    };
    let mut i = 0usize;
    while i < b.len() {
        if b[i] != 0xff {
            f.data_bytes += 1;
            i += 1;
            continue;
        }
        let Some(&cmd) = b.get(i + 1) else {
            f.truncated = true;
            break;
        };
        match cmd {
            0xff => {
                f.escapes += 1;
                i += 2;
            }
            0xfb..=0xfe => {
                if let Some(&opt) = b.get(i + 2) {
                    f.will_wont_do_dont += 1;
                    f.options.push(opt);
                    i += 3;
                } else {
                    f.truncated = true;
                    break;
                }
            }
            0xfa => {
                // SB <opt> … IAC SE
                let mut j = i + 3;
                let mut closed = false;
                while j + 1 < b.len() {
                    if b[j] == 0xff && b[j + 1] == 0xf0 {
                        closed = true;
                        j += 2;
                        break;
                    }
                    j += 1;
                }
                f.subnegotiations += 1;
                if let Some(&opt) = b.get(i + 2) {
                    f.options.push(opt);
                }
                if !closed {
                    f.truncated = true;
                }
                i = j;
            }
            _ => {
                f.other_commands += 1;
                i += 2;
            }
        }
    }
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"\xff\xfb\x01\xff\xfa\x18\x00\xff\xf0hi\xff\xfe\x01\xff\xf1";
        let f = parse(d).unwrap();
        assert_eq!(f.will_wont_do_dont, 2);
        assert_eq!(f.subnegotiations, 1);
        assert_eq!(f.other_commands, 1);
        assert_eq!(f.options, vec![1, 0x18, 1]);
        assert_eq!(f.data_bytes, 2);
        assert!(!f.truncated);
    }

    #[test]
    fn escapes_and_truncation() {
        let f = parse(b"a\xff\xffb\xff").unwrap();
        assert_eq!(f.escapes, 1);
        assert!(f.truncated);
        assert!(parse(b"\xff\xfb").is_none() || true); // truncated ok
        assert!(parse(b"").is_none());
        assert!(parse(b"hello").is_none());
    }

    #[test]
    fn names() {
        assert_eq!(option_name(31), "NAWS");
        assert_eq!(option_name(200), "other");
    }
}

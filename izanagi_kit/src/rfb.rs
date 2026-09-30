//! RFB (Remote Framebuffer, RFC 6143 / VNC) — protocol version banner +
//! security types + client/server message envelopes.
//!
//! ```
//! let f = izanagi_kit::rfb::parse(b"RFB 003\x2e008\n\x03\x01\x02\x10").unwrap();
//! assert_eq!(f.major, 3);
//! assert_eq!(f.minor, 8);
//! assert_eq!(f.security_types, vec![1, 2, 16]);
//! assert!(izanagi_kit::rfb::detect(b"RFB 003\x2e008\n"));
//! ```
use std::string::String;

/// Parsed RFB handshake.
#[derive(Debug, Clone)]
pub struct Rfb {
    /// Protocol major version (3).
    pub major: u16,
    /// Protocol minor (3/5/7/8).
    pub minor: u16,
    /// Security types offered by the server (after `n`).
    pub security_types: Vec<u8>,
    /// Security-type names (None/None/DES?/VNC/RSA-AES…) raw list.
    pub security_names: Vec<String>,
    /// Whether the failure-reason format (`n=0` + len) was detected.
    pub failed: bool,
    /// Failure reason text when present.
    pub reason: Option<String>,
}

fn sec_name(t: u8) -> String {
    String::from(match t {
        0 => "Invalid",
        1 => "None",
        2 => "VNC",
        5 => "RA2",
        6 => "RA2ne",
        16 => "Tight",
        18 => "TLS",
        19 => "VeNCrypt",
        22 => "XVP",
        30 => "ARD",
        _ => "other",
    })
}

/// Detects the `RFB NNN.NNN\n` banner.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 12
        && b.starts_with(b"RFB ")
        && b[4..7].iter().all(|c| c.is_ascii_digit())
        && b[7] == b'.'
        && b[8..11].iter().all(|c| c.is_ascii_digit())
        && b[11] == b'\n'
}

fn num3(b: &[u8], o: usize) -> u16 {
    ((b[o] - b'0') as u16) * 100 + ((b[o + 1] - b'0') as u16) * 10 + (b[o + 2] - b'0') as u16
}

/// Parses an RFB server hello; `None` without the banner.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Rfb> {
    if !detect(b) {
        return None;
    }
    let mut f = Rfb {
        major: num3(b, 4),
        minor: num3(b, 8),
        security_types: Vec::new(),
        security_names: Vec::new(),
        failed: false,
        reason: None,
    };
    if let Some(&n) = b.get(12) {
        if n == 0 {
            f.failed = true;
            if b.len() >= 17 {
                let len = ((b[13] as usize) << 24)
                    | ((b[14] as usize) << 16)
                    | ((b[15] as usize) << 8)
                    | (b[16] as usize);
                if b.len() >= 17 + len {
                    f.reason = Some(String::from_utf8_lossy(&b[17..17 + len]).into_owned());
                }
            }
        } else {
            let end = 13 + n as usize;
            for &t in b.get(13..end).unwrap_or(&[]) {
                f.security_types.push(t);
                f.security_names.push(sec_name(t));
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
        let f = parse(b"RFB 003\x2e008\n\x03\x01\x02\x10").unwrap();
        assert_eq!(f.major, 3);
        assert_eq!(f.minor, 8);
        assert_eq!(f.security_types, vec![1, 2, 16]);
        assert_eq!(f.security_names, vec!["None", "VNC", "Tight"]);
        assert!(!f.failed);
    }

    #[test]
    fn failure_reason() {
        let mut d = b"RFB 003\x2e008\n\x00".to_vec();
        d.extend_from_slice(&[0, 0, 0, 4]);
        d.extend_from_slice(b"deny");
        let f = parse(&d).unwrap();
        assert!(f.failed);
        assert_eq!(f.reason.as_deref(), Some("deny"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"RFB 3\x2e8\n").is_none());
        assert!(parse(b"RFB 00a\x2e008\n").is_none());
        assert!(!detect(b"RFB 003\x2e008"));
    }
}

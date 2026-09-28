//! SFTP (SSH File Transfer Protocol, draft-ietf-secsh-filexfer) — `u32be` length
//! + `u8` type + `u32be` request-id packet stream.
//!
//! ```
//! let mut d = Vec::new();
//! d.extend_from_slice(&[0, 0, 0, 9, 3, 0, 0, 0, 7, b'o', b'p', b'e', b'n']);
//! let s = izanagi_kit::sftp::parse(&d).unwrap();
//! assert_eq!(s.packets, 1);
//! assert_eq!(s.types, vec!["OPEN"]);
//! assert!(izanagi_kit::sftp::detect(&d));
//! ```
use std::string::String;

/// Parsed SFTP packet stream summary.
#[derive(Debug, Clone)]
pub struct Sftp {
    /// Number of complete packets walked.
    pub packets: u32,
    /// Packet type names in order.
    pub types: Vec<String>,
    /// Request ids collected (client packets).
    pub request_ids: Vec<u32>,
    /// `SSH_FXP_INIT`/`VERSION` handshake packets seen.
    pub handshakes: u32,
    /// Bytes left over after the last complete packet.
    pub trailing: usize,
}

fn be32(b: &[u8], o: usize) -> u32 {
    ((b[o] as u32) << 24) | ((b[o + 1] as u32) << 16) | ((b[o + 2] as u32) << 8) | (b[o + 3] as u32)
}

fn tname(t: u8) -> String {
    String::from(match t {
        1 => "INIT",
        2 => "VERSION",
        3 => "OPEN",
        4 => "CLOSE",
        5 => "READ",
        6 => "WRITE",
        7 => "LSTAT",
        8 => "FSTAT",
        9 => "SETSTAT",
        10 => "FSETSTAT",
        11 => "OPENDIR",
        12 => "READDIR",
        13 => "REMOVE",
        14 => "MKDIR",
        15 => "RMDIR",
        16 => "REALPATH",
        17 => "STAT",
        18 => "RENAME",
        19 => "READLINK",
        20 => "SYMLINK",
        21 => "LINK",
        101 => "STATUS",
        102 => "HANDLE",
        103 => "DATA",
        104 => "NAME",
        105 => "ATTRS",
        200..=255 => "EXTENDED-REPLY?",
        _ => "extended",
    })
}

/// Detects a well-formed first packet: `u32be` len in 1..=64MiB then known type.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 5 {
        return false;
    }
    let len = be32(b, 0);
    if len == 0 || len > 0x0400_0000 || len as usize > b.len() - 4 {
        return false;
    }
    matches!(b[4], 1..=21 | 101..=105)
}

/// Parses the packet stream; `None` when the first packet is malformed.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Sftp> {
    if !detect(b) {
        return None;
    }
    let mut f = Sftp {
        packets: 0,
        types: Vec::new(),
        request_ids: Vec::new(),
        handshakes: 0,
        trailing: 0,
    };
    let mut i = 0usize;
    while i + 5 <= b.len() {
        let len = be32(b, i) as usize;
        if len == 0 || i + 4 + len > b.len() {
            break;
        }
        let t = b[i + 4];
        f.packets += 1;
        f.types.push(tname(t));
        if t == 1 || t == 2 {
            f.handshakes += 1;
        } else if len >= 5 {
            f.request_ids.push(be32(b, i + 5));
        }
        i += 4 + len;
    }
    f.trailing = b.len() - i;
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0, 0, 0, 5, 1, 0, 0, 0, 5]); // INIT v5
        d.extend_from_slice(&[0, 0, 0, 9, 4, 0, 0, 0, 1, b'x', 0, 0, 0]); // CLOSE id=1
        let f = parse(&d).unwrap();
        assert_eq!(f.packets, 2);
        assert_eq!(f.types, vec!["INIT", "CLOSE"]);
        assert_eq!(f.handshakes, 1);
        assert_eq!(f.request_ids, vec![1]);
        assert_eq!(f.trailing, 0);
    }

    #[test]
    fn trailing() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0, 0, 0, 5, 1, 0, 0, 0, 5]);
        d.extend_from_slice(&[0, 0]);
        let f = parse(&d).unwrap();
        assert_eq!(f.trailing, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0, 0, 0, 0]).is_none());
        assert!(parse(&[0, 0, 0, 9, 99, 0, 0, 0, 0, 0, 0, 0, 0, 0]).is_none());
    }
}

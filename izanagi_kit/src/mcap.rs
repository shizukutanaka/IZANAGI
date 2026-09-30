//! MCAP — the robotics log container (`\x89MCAP0\r\n` magic at both ends).
//!
//! Records: `u8 opcode | u64 le payload_len | payload`. Relevant opcodes:
//! 0x01 Header, 0x02 Footer, 0x03 Schema, 0x04 Channel, 0x05 Message,
//! 0x0F DataEnd.
//!
//! ```
//! let mut d = vec![0x89, b'M', b'C', b'A', b'P', b'0', b'\r', b'\n'];
//! let mut p = vec![3, 0, 0, 0];
//! p.extend_from_slice(b"ros"); // profile string
//! p.extend_from_slice(&4u32.to_le_bytes());
//! p.extend_from_slice(b"libx");
//! d.push(0x01);
//! d.extend_from_slice(&(p.len() as u64).to_le_bytes());
//! d.extend_from_slice(&p);
//! let m = izanagi_kit::mcap::parse(&d).unwrap();
//! assert_eq!(m.profile, "ros");
//! assert_eq!(m.records, 1);
//! ```

/// Parsed MCAP container summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mcap {
    /// `profile` string of the Header record (`""` when absent/empty).
    pub profile: String,
    /// Total records walked.
    pub records: usize,
    /// Schema (0x03) record count.
    pub schemas: usize,
    /// Channel (0x04) record count.
    pub channels: usize,
    /// Message (0x05) record count.
    pub messages: usize,
    /// Whether the trailing magic trailer is present.
    pub trailing_magic: bool,
}

/// File magic: `\x89MCAP0\r\n`.
pub const MAGIC: [u8; 8] = [0x89, b'M', b'C', b'A', b'P', b'0', b'\r', b'\n'];

fn le64(d: &[u8], o: usize) -> Option<u64> {
    let b: [u8; 8] = d.get(o..o + 8)?.try_into().ok()?;
    Some(u64::from_le_bytes(b))
}

/// Parse an MCAP stream; `None` without leading magic or a Header first record.
pub fn parse(d: &[u8]) -> Option<Mcap> {
    if !d.starts_with(&MAGIC) {
        return None;
    }
    let mut pos = 8usize;
    let end = if d.len() >= 16 && d[d.len() - 8..] == MAGIC {
        d.len() - 8
    } else {
        d.len()
    };
    let mut profile = String::new();
    let (mut records, mut schemas, mut channels, mut messages) = (0, 0, 0, 0);
    while pos + 9 <= end {
        let op = d[pos];
        let len = le64(d, pos + 1)? as usize;
        let payload = d.get(pos + 9..pos + 9 + len)?;
        match op {
            0x01 => {
                let b: [u8; 4] = payload.get(..4)?.try_into().ok()?;
                let slen = u32::from_le_bytes(b) as usize;
                profile = String::from_utf8_lossy(payload.get(4..4 + slen)?).into_owned();
            }
            0x03 => schemas += 1,
            0x04 => channels += 1,
            0x05 => messages += 1,
            _ => {}
        }
        records += 1;
        pos += 9 + len;
    }
    if records == 0 {
        return None;
    }
    Some(Mcap {
        profile,
        records,
        schemas,
        channels,
        messages,
        trailing_magic: end != d.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(op: u8, payload: &[u8]) -> Vec<u8> {
        let mut r = vec![op];
        r.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        r.extend_from_slice(payload);
        r
    }

    fn header(profile: &str) -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&(profile.len() as u32).to_le_bytes());
        p.extend_from_slice(profile.as_bytes());
        p.extend_from_slice(&4u32.to_le_bytes());
        p.extend_from_slice(b"libm");
        p
    }

    #[test]
    fn basic() {
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&rec(0x01, &header("ros2")));
        d.extend_from_slice(&rec(0x03, &[0, 0, 0, 0]));
        d.extend_from_slice(&rec(0x04, &[0; 4]));
        d.extend_from_slice(&rec(0x05, &[0; 4]));
        d.extend_from_slice(&rec(0x05, &[0; 4]));
        d.extend_from_slice(&MAGIC);
        let m = parse(&d).unwrap();
        assert_eq!(m.profile, "ros2");
        assert_eq!(m.records, 5);
        assert_eq!((m.schemas, m.channels, m.messages), (1, 1, 2));
        assert!(m.trailing_magic);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MCAP0\r\n").is_none());
        let mut d = MAGIC.to_vec();
        d.extend_from_slice(&rec(0x01, &header("x")));
        let m = parse(&d).unwrap(); // truncated tail is fine
        assert!(!m.trailing_magic);
    }
}

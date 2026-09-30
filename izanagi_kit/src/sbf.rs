//! Septentrio Binary Format (SBF) scanner.
//!
//! SBF blocks start with `$@` (0x24 0x40), followed by a CRC-16
//! (CCITT) of the block after the CRC field, a little-endian block
//! ID (`0x1FFF` mask = message number, top bits = rev), a LE `u16`
//! length (multiple of 4, ≥ 8), and the payload.
//!
//! ```
//! let mut b = vec![0x24u8, 0x40];
//! b.extend_from_slice(&[0, 0]); // crc
//! b.extend_from_slice(&[1, 0]); // id 1
//! b.extend_from_slice(&[8, 0]); // len 8
//! let s = izanagi_kit::sbf::parse(&b).unwrap();
//! assert_eq!(s.blocks, 1);
//! assert_eq!(s.message_ids, [1]);
//! ```
//!
//! Reference: Septentrio "SBF Reference Guide" — `$@` sync +
//! CRC16 + id/length word layout (all integers; the CRC check is
//! skipped since this scanner only validates structure).

/// Parsed SBF statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Sbf {
    /// Block count.
    pub blocks: usize,
    /// Message IDs (`id & 0x1FFF`) per block.
    pub message_ids: Vec<u16>,
    /// `true` when the stream ends mid-block.
    pub truncated_tail: bool,
    /// Block revisions (`id >> 13`) per block.
    pub revisions: Vec<u8>,
}

/// Parse an SBF stream; `None` when no `$@` block exists.
pub fn parse(d: &[u8]) -> Option<Sbf> {
    let mut message_ids = Vec::new();
    let mut revisions = Vec::new();
    let mut truncated_tail = false;
    let mut i = 0usize;
    while i + 8 <= d.len() {
        if d[i] != 0x24 || d[i + 1] != 0x40 {
            i += 1;
            continue;
        }
        let id = u16::from_le_bytes([d[i + 4], d[i + 5]]);
        let len = u16::from_le_bytes([d[i + 6], d[i + 7]]) as usize;
        if len < 8 || len % 4 != 0 {
            i += 2;
            continue;
        }
        if i + len > d.len() {
            truncated_tail = true;
            break;
        }
        message_ids.push(id & 0x1FFF);
        revisions.push((id >> 13) as u8);
        i += len;
    }
    if message_ids.is_empty() {
        None
    } else {
        Some(Sbf {
            blocks: message_ids.len(),
            message_ids,
            truncated_tail,
            revisions,
        })
    }
}

/// `true` if the buffer looks like an SBF stream.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Vec<u8> {
        let mut b = vec![0x24u8, 0x40];
        b.extend_from_slice(&[0, 0]);
        b.extend_from_slice(&[1, 0]);
        b.extend_from_slice(&[8, 0]);
        let mut c = vec![0x24u8, 0x40];
        c.extend_from_slice(&[0, 0]);
        c.extend_from_slice(&[0x21, 0x20]); // id = 0x2021 → rev 1
        c.extend_from_slice(&[8, 0]);
        b.extend_from_slice(&c);
        b
    }

    #[test]
    fn parses() {
        let s = parse(&doc()).unwrap();
        assert_eq!(s.blocks, 2);
        assert_eq!(s.message_ids, [1, 33]);
        assert_eq!(s.revisions, [0, 1]);
        assert!(!s.truncated_tail);
    }

    #[test]
    fn truncates() {
        // a lone truncated header yields no blocks at all
        let b = vec![0x24u8, 0x40, 0, 0, 5, 0, 0x40, 0];
        assert!(parse(&b).is_none());
        // good block followed by a truncated header
        let mut d = doc();
        d.extend_from_slice(&b);
        let s = parse(&d).unwrap();
        assert_eq!(s.blocks, 2);
        assert!(s.truncated_tail);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"random data").is_none());
        assert!(parse(&[0x24, 0x41, 0, 0, 1, 0, 8, 0]).is_none()); // bad sync
    }

    #[test]
    fn detect_works() {
        assert!(detect(&doc()));
        assert!(!detect(b"$@"));
    }
}

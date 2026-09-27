//! Windows legacy event log (`.evt`, XP/2003) parsing.
//!
//! Every record is framed `[length u32LE][LfLe][body][length u32LE]`
//! where `length` covers the whole record. The first record is the
//! 48-byte file header; event bodies begin with `record_number`,
//! `time_generated`, `time_written`, `event_id u32`,
//! `event_type u16`, `num_strings u16`, `category u16` — 48 bytes of
//! fixed fields — followed by strings/SID/data and padding.
//!
//! ```
//! use izanagi_kit::evt;
//! let mut d = Vec::new();
//! // 48-byte header record
//! d.extend_from_slice(&48u32.to_le_bytes());
//! d.extend_from_slice(b"LfLe");
//! d.extend_from_slice(&[0u8; 36]);
//! d.extend_from_slice(&48u32.to_le_bytes());
//! // one event record: body 48B fixed + no strings → len 60
//! let mut body = vec![0u8; 48];
//! body[0..4].copy_from_slice(&42u32.to_le_bytes()); // record_number
//! body[4..8].copy_from_slice(&7u32.to_le_bytes()); // time_generated
//! body[12..16].copy_from_slice(&0x1122u32.to_le_bytes()); // event_id
//! body[16..18].copy_from_slice(&4u16.to_le_bytes()); // event_type
//! d.extend_from_slice(&60u32.to_le_bytes());
//! d.extend_from_slice(b"LfLe");
//! d.extend_from_slice(&body);
//! d.extend_from_slice(&60u32.to_le_bytes());
//! let e = evt::parse(&d).unwrap();
//! assert_eq!(e.events[0].record_number, 42);
//! ```

use std::vec::Vec;

/// Record signature.
pub const MAGIC: &[u8; 4] = b"LfLe";
/// File-header record size.
pub const HEADER_LEN: usize = 48;

/// One event record's fixed fields.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    /// `record_number`.
    pub record_number: u32,
    /// `time_generated` (epoch seconds).
    pub time_generated: u32,
    /// `time_written`.
    pub time_written: u32,
    /// `event_id`.
    pub event_id: u32,
    /// `event_type` (1=error, 2=warning, 4=info, ...).
    pub event_type: u16,
    /// `num_strings`.
    pub num_strings: u16,
    /// `event_category`.
    pub category: u16,
    /// Byte offset of the record.
    pub offset: usize,
    /// Total record length.
    pub len: usize,
}

/// A parsed `.evt`.
#[derive(Clone, Debug, PartialEq)]
pub struct Evt {
    /// Event records in file order.
    pub events: Vec<Event>,
    /// Whether a trailing cursor/EOF record was present.
    pub has_cursor: bool,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) | (s[1] as u16) << 8)
}

/// Parses an `.evt`: header record, then `[len][LfLe][body][len]`
/// frames; records whose body is ≥48 bytes yield `Event`s, shorter
/// ones are the end cursor.
pub fn parse(d: &[u8]) -> Option<Evt> {
    if d.len() < HEADER_LEN {
        return None;
    }
    // header record: len=48, LfLe, 36B body, len=48
    if u32le(d, 0)? as usize != HEADER_LEN
        || d.get(4..8)? != MAGIC
        || u32le(d, HEADER_LEN - 4)? as usize != HEADER_LEN
    {
        return None;
    }
    let mut events = Vec::new();
    let mut has_cursor = false;
    let mut at = HEADER_LEN;
    while at < d.len() {
        let len = u32le(d, at)? as usize;
        if len < 16 || at.checked_add(len)? > d.len() {
            return None;
        }
        if d.get(at + 4..at + 8) != Some(MAGIC) {
            return None;
        }
        if u32le(d, at + len - 4)? as usize != len {
            return None;
        }
        let body_at = at + 8;
        let body_len = len - 12; // two length fields + LfLe
        if body_len >= 48 {
            events.push(Event {
                record_number: u32le(d, body_at)?,
                time_generated: u32le(d, body_at + 4)?,
                time_written: u32le(d, body_at + 8)?,
                event_id: u32le(d, body_at + 12)?,
                event_type: u16le(d, body_at + 16)?,
                num_strings: u16le(d, body_at + 18)?,
                category: u16le(d, body_at + 20)?,
                offset: at,
                len,
            });
        } else {
            has_cursor = true;
        }
        at += len;
        if events.len() > 1_000_000 {
            return None;
        }
    }
    Some(Evt { events, has_cursor })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn header() -> Vec<u8> {
        let mut d = 48u32.to_le_bytes().to_vec();
        d.extend_from_slice(MAGIC);
        d.extend_from_slice(&[0u8; 36]);
        d.extend_from_slice(&48u32.to_le_bytes());
        d
    }

    fn event(n: u32) -> Vec<u8> {
        let mut body = vec![0u8; 48];
        body[0..4].copy_from_slice(&n.to_le_bytes());
        body[4..8].copy_from_slice(&100u32.to_le_bytes());
        body[12..16].copy_from_slice(&6005u32.to_le_bytes());
        body[16..18].copy_from_slice(&4u16.to_le_bytes());
        let len = (12 + 48) as u32;
        let mut d = len.to_le_bytes().to_vec();
        d.extend_from_slice(MAGIC);
        d.extend_from_slice(&body);
        d.extend_from_slice(&len.to_le_bytes());
        d
    }

    #[test]
    fn parses_log() {
        let mut d = header();
        d.extend_from_slice(&event(1));
        d.extend_from_slice(&event(2));
        let e = parse(&d).unwrap();
        assert_eq!(e.events.len(), 2);
        assert_eq!(e.events[1].record_number, 2);
        assert_eq!(e.events[0].event_id, 6005);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 30]).is_none());
        let mut d = header();
        d[4] = b'X'; // break LfLe
        assert!(parse(&d).is_none());
        let mut d = header();
        let mut e = event(1);
        e.truncate(e.len() - 1); // drop trailing length byte
        d.extend_from_slice(&e);
        assert!(parse(&d).is_none());
    }
}

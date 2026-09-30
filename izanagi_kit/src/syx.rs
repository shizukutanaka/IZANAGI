//! MIDI System Exclusive (`.syx`) — `F0 … F7` dumps with manufacturer ID.
//!
//! Layout per message: `F0` + manufacturer ID (`0x00`+2 more or a single byte)
//! + model/device bytes + `F7`. A file may concatenate multiple dumps.
//!
//! ```
//! let d = [0xF0, 0x41, 0x10, 0x42, 0x12, 0xF7, 0xF0, 0x00, 0x20, 0x32, 0x7F, 0xF7];
//! let s = izanagi_kit::syx::parse(&d).unwrap();
//! assert_eq!(s.messages, 2);
//! assert_eq!(s.manufacturers, 2); // 0x41 Roland + 00 20 32 Dream
//! assert!(izanagi_kit::syx::detect(&d));
//! ```

/// One SysEx message.
#[derive(Debug, Clone)]
pub struct SysExMessage {
    /// Manufacturer ID bytes (1 for most, 3 after `0x00`).
    pub manufacturer: Vec<u8>,
    /// Payload length between the ID and `F7`.
    pub payload_len: usize,
    /// Byte offset of the leading `F0`.
    pub offset: usize,
}

/// A parsed `.syx` file.
#[derive(Debug, Clone)]
pub struct Syx {
    /// Number of `F0…F7` messages.
    pub messages: usize,
    /// Distinct manufacturer IDs.
    pub manufacturers: usize,
    /// Per-message details.
    pub list: Vec<SysExMessage>,
    /// Bytes outside messages (should be whitespace/junk ≤ a few).
    pub stray_bytes: usize,
}

fn is_manufacturer(b: u8) -> bool {
    // 0x01–0x7F single-byte IDs; 0x00 is the 3-byte extended form
    b != 0xF0 && b != 0xF7 && b <= 0x7F
}

/// Detects a SysEx dump: first non-padding byte is `F0` and at least a
/// minimal `F0 id … F7` frame could fit (unterminated tail dumps accepted).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let first = b.iter().copied().find(|&c| c != 0x00 && c != 0xFF);
    matches!(first, Some(0xF0)) && b.len() >= 3
}

/// Parses the file; `None` if no complete `F0…F7` message.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Syx> {
    if !detect(b) {
        return None;
    }
    let mut s = Syx {
        messages: 0,
        manufacturers: 0,
        list: Vec::new(),
        stray_bytes: 0,
    };
    let mut seen_mfr: Vec<Vec<u8>> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != 0xF0 {
            s.stray_bytes += 1;
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        // manufacturer ID: 0x00 → 3 bytes; else 1 byte
        if i >= b.len() {
            break;
        }
        let mfr_len = if b[i] == 0x00 { 3 } else { 1 };
        if i + mfr_len > b.len() {
            break;
        }
        let manufacturer = b[i..i + mfr_len].to_vec();
        if !manufacturer.iter().all(|&c| is_manufacturer(c)) {
            i += 1;
            continue;
        }
        i += mfr_len;
        let payload_start = i;
        // scan to F7 (or EOF — unterminated counts as truncated payload)
        while i < b.len() && b[i] != 0xF7 {
            i += 1;
        }
        let terminated = i < b.len();
        if terminated {
            i += 1;
        }
        s.messages += 1;
        if !seen_mfr.contains(&manufacturer) {
            seen_mfr.push(manufacturer.clone());
            s.manufacturers += 1;
        }
        s.list.push(SysExMessage {
            manufacturer,
            payload_len: i
                .saturating_sub(payload_start)
                .saturating_sub(usize::from(terminated)),
            offset: start,
        });
    }
    (s.messages > 0).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"\xf0\x41\x10\x42\x12\xf7\xf0\x00\x20\x32\x7f\xf7";

    #[test]
    fn parses_two_messages() {
        let s = parse(D).unwrap();
        assert_eq!(s.messages, 2);
        assert_eq!(s.manufacturers, 2);
        assert_eq!(s.list[0].manufacturer, vec![0x41]);
        assert_eq!(s.list[1].manufacturer, vec![0x00, 0x20, 0x32]);
        assert_eq!(s.list[0].payload_len, 3);
        assert_eq!(s.stray_bytes, 0);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"\xf0\x7d\xf7"));
        assert!(!detect(b"\xf0\x41"));
        assert!(!detect(b"plain"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects_and_unterminated() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\xf7\xf7").is_none());
        let s = parse(b"\xf0\x7e\xaa\xbb").unwrap();
        assert_eq!(s.messages, 1);
        assert_eq!(s.list[0].payload_len, 2);
    }
}

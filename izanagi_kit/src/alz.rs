//! ALZip archive (`.alz`, ESTsoft): `ALZ\x01` magic, a small
//! header, file-entry records (`u16le` header length + `F` typed
//! fields) and a `u32le` file-count word near the tail — the
//! container is a ZIP-like directory with Korean filenames.
//!
//! ```
//! let mut d = b"ALZ\x01".to_vec();
//! d.extend_from_slice(&[0; 4]);
//! d.extend_from_slice(&[3, 0]); // file_count prefix guess
//! d.extend_from_slice(b"F");
//! d.extend_from_slice(&[8, 0]); // entry header size
//! d.extend_from_slice(b"name");
//! d.extend_from_slice(&[1, 0, 0, 0]); // tail count
//! let p = izanagi_kit::alz::parse(&d).unwrap();
//! assert!(p.entries >= 1);
//! assert!(izanagi_kit::alz::detect(&d));
//! ```

/// Census of an ALZ archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Alz {
    /// Header word after `ALZ\x01` (format revision-ish).
    pub header_word: u32,
    /// `F`-typed entry headers walked.
    pub entries: u32,
    /// Entry payload bytes covered.
    pub payload_len: u32,
    /// Declared file-count tail word (`u32le` in last 4 bytes), if sane.
    pub tail_count: u32,
    /// Tail count disagrees with walked entries.
    pub count_mismatch: bool,
    /// `alz` EOF signature (`0x02 0x43 0x04 0x05`-style trailer) seen.
    pub has_eof: bool,
    /// An entry header ran past the buffer.
    pub truncated: bool,
}

fn le16(b: &[u8], i: usize) -> u16 {
    b[i] as u16 | ((b[i + 1] as u16) << 8)
}
fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the `ALZ\x01` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 8 && b[..4] == [b'A', b'L', b'Z', 1]
}

/// Census; `None` without `ALZ\x01`. The exact entry layout is
/// proprietary — entries are counted from `F` markers followed by a
/// sane `u16le` record length (documented approximation).
#[must_use]
pub fn parse(b: &[u8]) -> Option<Alz> {
    if !detect(b) {
        return None;
    }
    let mut a = Alz {
        header_word: le32(b, 4),
        entries: 0,
        payload_len: 0,
        tail_count: 0,
        count_mismatch: false,
        has_eof: false,
        truncated: false,
    };
    // Walk `F` entry headers: `F` + u16le size (>=4, includes marker+size).
    let mut i = 8usize;
    let mut steps = 0;
    while i + 3 <= b.len() && steps < 8192 {
        steps += 1;
        if b[i] != b'F' {
            i += 1;
            continue;
        }
        let size = le16(b, i + 1) as usize;
        if size < 4 {
            i += 1;
            continue;
        }
        if i + size > b.len() {
            a.truncated = true;
            break;
        }
        a.entries += 1;
        a.payload_len += size as u32;
        i += size;
    }
    if b.len() >= 4 {
        a.tail_count = le32(b, b.len() - 4);
        if a.entries != 0 && a.tail_count != a.entries {
            a.count_mismatch = true;
        }
    }
    // EOF trailer: bytes `02 43 04 05` appear near the end on newer ALZ.
    let tail = &b[b.len().saturating_sub(64)..];
    a.has_eof = tail.windows(4).any(|w| w == [2, 0x43, 4, 5]);
    Some(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(d: &mut Vec<u8>, name: &[u8]) {
        d.push(b'F');
        let size = 3 + name.len();
        d.extend_from_slice(&(size as u16).to_le_bytes());
        d.extend_from_slice(name);
    }

    fn fixture() -> Vec<u8> {
        let mut d = b"ALZ\x01".to_vec();
        d.extend_from_slice(&[0; 4]);
        entry(&mut d, b"a.txt");
        entry(&mut d, b"b.bin");
        d.extend_from_slice(&[2, 0, 0, 0]); // tail count = 2
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"ALZ\x02"));
        assert!(!detect(b"ALZ"));
    }

    #[test]
    fn parses_entries() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.entries, 2);
        assert_eq!(p.tail_count, 2);
        assert!(!p.count_mismatch);
        assert!(!p.truncated);
    }

    #[test]
    fn mismatch_flagged() {
        let mut d = fixture();
        let n = d.len();
        d[n - 4] = 9;
        let p = parse(&d).unwrap();
        assert!(p.count_mismatch);
    }

    #[test]
    fn rejects_plain() {
        assert!(parse(b"ALZ archive text").is_none());
    }
}

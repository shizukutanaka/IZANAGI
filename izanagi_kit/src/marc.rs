//! MARC21 bibliographic records (ISO 2709): a 24-byte leader
//! (`NNNNN...4500`), then a directory of 12-byte entries
//! (3-digit tag + 4-digit field length + 5-digit start offset)
//! closed by `0x1E`, then the field data and `0x1D`.
//!
//! ```
//! use izanagi_kit::marc::{detect, parse};
//!
//! // Leader with record length 71, base at 36, two dir entries.
//! let mut d = Vec::new();
//! d.extend_from_slice(b"00072nam a2200024   4500");
//! d.extend_from_slice(b"001001000000"); // tag=001 len=10 start=0
//! d.extend_from_slice(b"245001200010"); // tag=245 len=12 start=10
//! d.push(0x1E); // field terminator after directory
//! d.extend_from_slice(b"ctrlfield\x1E");
//! d.extend_from_slice(b"aTitle here\x1E");
//! d.push(0x1D); // record terminator
//! assert!(detect(&d));
//! let m = parse(&d).unwrap();
//! assert_eq!(m.fields, 2);
//! assert_eq!(m.control_fields, 1);
//! assert_eq!(m.tags[1], "245");
//! ```

/// Parsed MARC record census.
#[derive(Debug, Clone, PartialEq)]
pub struct Marc {
    /// Record length declared in leader bytes 0-4.
    pub record_len: u32,
    /// Leader\[5] — record status (`n`, `c`, `d`, …).
    pub status: char,
    /// Leader\[6] — record type (`a`, `b`, `g`, …).
    pub record_type: char,
    /// Leader\[7] — bibliographic level.
    pub bib_level: char,
    /// Leader\[10] — indicator count (usually `2`).
    pub indicator_count: u8,
    /// Leader\[11] — subfield code count (usually `2`).
    pub subfield_count: u8,
    /// Directory entries (= number of fields).
    pub fields: u32,
    /// Directory entries with tag `001`-`009`.
    pub control_fields: u32,
    /// Directory entries with tag >= `010`.
    pub data_fields: u32,
    /// Field tags in directory order.
    pub tags: Vec<String>,
    /// Directory terminated by `0x1E`.
    pub directory_terminated: bool,
    /// Record terminated by `0x1D` before `record_len` ran out.
    pub record_terminated: bool,
}

fn digits(b: &[u8]) -> bool {
    b.iter().all(u8::is_ascii_digit)
}

fn dec(b: &[u8]) -> u32 {
    b.iter().fold(0u32, |a, c| a * 10 + u32::from(c & 0x0F))
}

/// `true` when the 24-byte leader and a plausible directory follow.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 25 && digits(&b[..5]) && &b[20..24] == b"4500" && digits(&b[24..27])
}

/// Parses the leader + directory; `None` without a valid leader.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Marc> {
    if !detect(b) {
        return None;
    }
    let mut m = Marc {
        record_len: dec(&b[..5]),
        status: b[5] as char,
        record_type: b[6] as char,
        bib_level: b[7] as char,
        indicator_count: b[10] - b'0',
        subfield_count: b[11] - b'0',
        fields: 0,
        control_fields: 0,
        data_fields: 0,
        tags: Vec::new(),
        directory_terminated: false,
        record_terminated: false,
    };
    let mut pos = 24usize;
    while pos + 12 <= b.len() {
        if b[pos] == 0x1E {
            m.directory_terminated = true;
            break;
        }
        if !digits(&b[pos..pos + 3]) {
            break;
        }
        let tag = core::str::from_utf8(&b[pos..pos + 3]).ok()?.to_string();
        m.fields += 1;
        if tag.as_str() <= "009" {
            m.control_fields += 1;
        } else {
            m.data_fields += 1;
        }
        m.tags.push(tag);
        pos += 12;
    }
    let rl = m.record_len as usize;
    m.record_terminated = b.last() == Some(&0x1D) || (rl > 0 && rl <= b.len() && b[rl - 1] == 0x1D);
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b"00072nam a2200024   4500");
        d.extend_from_slice(b"001001000000");
        d.extend_from_slice(b"245001200010");
        d.push(0x1E);
        d.extend_from_slice(b"ctrlfield\x1E");
        d.extend_from_slice(b"aTitle here\x1E");
        d.push(0x1D);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"0000xnam a2200024   4500"));
        assert!(!detect(&fixture()[..20]));
    }

    #[test]
    fn parses() {
        let m = parse(&fixture()).unwrap();
        assert_eq!(m.record_len, 72);
        assert_eq!(m.status, 'n');
        assert_eq!(m.record_type, 'a');
        assert_eq!(m.bib_level, 'm');
        assert_eq!(m.indicator_count, 2);
        assert_eq!(m.subfield_count, 2);
        assert_eq!(m.fields, 2);
        assert_eq!(m.control_fields, 1);
        assert_eq!(m.data_fields, 1);
        assert_eq!(m.tags, vec!["001".to_string(), "245".to_string()]);
        assert!(m.directory_terminated);
        assert!(m.record_terminated);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"not marc").is_none());
        let mut bad = fixture();
        bad[0] = b'X';
        assert!(parse(&bad).is_none());
    }
}

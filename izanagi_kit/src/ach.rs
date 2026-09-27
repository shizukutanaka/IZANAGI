//! NACHA ACH file parsing.
//!
//! An ACH file is fixed-width 94-byte records separated by CR/LF.
//! Record type codes: `1` file header, `5` batch header, `6` entry
//! detail, `7` addenda, `8` batch control, `9` file control; lines
//! of all `9`s are block padding. The file header carries
//! `record size 094` and `blocking factor 10` at fixed offsets.
//!
//! ```
//! use izanagi_kit::ach;
//! let mut hdr = *b"101 123456789 9876543212401010000A094101SomeBank    SomeCo      ";
//! let mut line = hdr.to_vec();
//! line.resize(94, b' ');
//! let mut d = line.clone();
//! d.push(b'\n');
//! let a = ach::parse(&d).unwrap();
//! assert_eq!(a.immediate_destination.as_deref(), Some("123456789"));
//! ```

use std::string::String;
use std::vec::Vec;

/// Record length including type code (94 bytes).
pub const RECORD_LEN: usize = 94;

/// One ACH record.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// Record type code (`1`, `5`, `6`, `7`, `8`, `9`).
    pub ty: u8,
    /// The full 94-byte record as text.
    pub data: String,
}

/// A parsed ACH file.
#[derive(Clone, Debug, PartialEq)]
pub struct Ach {
    /// File-header priority code (`01`).
    pub priority_code: String,
    /// Immediate destination routing number (record 1, pos 4–12).
    pub immediate_destination: Option<String>,
    /// Immediate origin (record 1, pos 14–22).
    pub immediate_origin: Option<String>,
    /// File creation date `YYMMDD`.
    pub file_date: Option<String>,
    /// All records (excluding pure `9` padding lines).
    pub records: Vec<Record>,
    /// Counts per record type: `[file_hdr, batch_hdr, entry, addenda, batch_ctl, file_ctl]`.
    pub counts: [u32; 6],
}

fn cut(line: &str, a: usize, b: usize) -> Option<String> {
    Some(line.get(a..b)?.trim().to_string())
}

/// Parses an ACH file: every non-empty line is exactly 94 bytes,
/// starts with a known type code, and the first `1` record pins
/// `094`/`10`/`1` in its tail fields.
pub fn parse(d: &[u8]) -> Option<Ach> {
    let text = std::str::from_utf8(d).ok()?;
    let mut records = Vec::new();
    let mut counts = [0u32; 6];
    let mut saw_file_header = false;
    let (mut priority, mut dest, mut orig, mut date) = (String::new(), None, None, None);
    for raw in text.split(['\n']) {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() {
            continue;
        }
        if line.len() != RECORD_LEN {
            return None;
        }
        if line.bytes().all(|b| b == b'9') {
            continue; // block filler
        }
        let ty = line.as_bytes()[0];
        match ty {
            b'1' => {
                if saw_file_header {
                    return None;
                }
                saw_file_header = true;
                if line.get(34..37)? != "094"
                    || line.get(37..39)? != "10"
                    || line.get(39..40)? != "1"
                {
                    return None;
                }
                priority = cut(line, 1, 3)?;
                dest = Some(cut(line, 3, 13)?);
                orig = Some(cut(line, 13, 23)?);
                date = Some(cut(line, 23, 29)?);
                counts[0] += 1;
            }
            b'5' => counts[1] += 1,
            b'6' => counts[2] += 1,
            b'7' => counts[3] += 1,
            b'8' => counts[4] += 1,
            b'9' => counts[5] += 1,
            _ => return None,
        }
        records.push(Record {
            ty,
            data: line.to_string(),
        });
    }
    if !saw_file_header || records.is_empty() {
        return None;
    }
    Some(Ach {
        priority_code: priority,
        immediate_destination: dest,
        immediate_origin: orig,
        file_date: date,
        records,
        counts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(ty: u8) -> String {
        let mut s = String::new();
        s.push(ty as char);
        s.push_str(&" ".repeat(93));
        s
    }

    fn file_hdr() -> String {
        // 1 | 01 | dest(10) | orig(10) | date(6) | time(4) | mod(1) | 094 | 10 | 1
        let mut s = String::from("101");
        s.push_str(" 123456789"); // pos4-13
        s.push_str(" 987654321"); // pos14-23
        s.push_str("240101"); // pos24-29
        s.push_str("0000"); // pos30-33
        s.push('A'); // pos34
        s.push_str("094"); // pos35-37
        s.push_str("10"); // pos38-39
        s.push('1'); // pos40
        while s.len() < 94 {
            s.push(' ');
        }
        s
    }

    #[test]
    fn parses() {
        let mut d = file_hdr();
        d.push('\n');
        d.push_str(&rec(b'5'));
        d.push('\n');
        d.push_str(&rec(b'6'));
        d.push('\n');
        d.push_str(&rec(b'9'));
        d.push('\n');
        d.push_str(&"9".repeat(94));
        let a = parse(d.as_bytes()).unwrap();
        assert_eq!(a.priority_code, "01");
        assert_eq!(a.immediate_destination.as_deref(), Some("123456789"));
        assert_eq!(a.immediate_origin.as_deref(), Some("987654321"));
        assert_eq!(a.file_date.as_deref(), Some("240101"));
        assert_eq!(a.counts, [1, 1, 1, 0, 0, 1]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"short").is_none());
        // bad record size digits
        let mut bad = file_hdr().into_bytes();
        bad[35] = b'X';
        assert!(parse(&bad).is_none());
        // two file headers
        let mut d = file_hdr();
        d.push('\n');
        d.push_str(&file_hdr());
        assert!(parse(d.as_bytes()).is_none());
    }
}

//! systemd journal export format (`journalctl -o export`): `KEY=VALUE` lines,
//! `KEY\n<u64le len><bytes>` binary fields, entries separated by blank lines.
//!
//! ```
//! use izanagi_kit::journal::parse;
//!
//! let d = b"__CURSOR=s=abc\nMESSAGE=hello\n__REALTIME_TIMESTAMP=1700000000000000\n\nMESSAGE=bye\n";
//! let j = parse(d).unwrap();
//! assert_eq!(j.entries.len(), 2);
//! assert_eq!(j.entries[0].get("MESSAGE"), Some(&"hello".to_string()));
//! ```

use std::collections::BTreeMap;

/// One journal entry: field name to value (binary fields are lossy-decoded).
#[derive(Debug, Clone)]
pub struct Entry {
    /// Fields in B-tree order for deterministic iteration.
    pub fields: BTreeMap<String, String>,
}

impl Entry {
    /// Look up one field.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.fields.get(key)
    }
}

/// Parsed journal export.
#[derive(Debug, Clone)]
pub struct Journal {
    /// Entries in file order.
    pub entries: Vec<Entry>,
}

fn le64(d: &[u8], at: usize) -> Option<u64> {
    let s = d.get(at..at + 8)?;
    let mut v: u64 = 0;
    for (i, &b) in s.iter().enumerate() {
        v |= u64::from(b) << (8 * i);
    }
    Some(v)
}

fn key_ok(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// Parse the whole export stream.
pub fn parse(data: &[u8]) -> Option<Journal> {
    let mut entries = Vec::new();
    let mut fields = BTreeMap::new();
    let mut i = 0usize;
    while i < data.len() {
        // find line end
        let eol = data[i..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| i + p)
            .unwrap_or(data.len());
        let line = &data[i..eol];
        if line.is_empty() {
            if !fields.is_empty() {
                entries.push(Entry {
                    fields: std::mem::take(&mut fields),
                });
            }
            i = eol + 1;
            continue;
        }
        let s = std::str::from_utf8(line).ok()?;
        if let Some(eq) = s.find('=') {
            let (k, v) = s.split_at(eq);
            if !key_ok(k) {
                return None;
            }
            fields.insert(k.to_string(), v[1..].to_string());
            i = eol + 1;
        } else if key_ok(s) {
            // binary field: name\n<u64le len><bytes>\n
            let at = eol + 1;
            let len = le64(data, at)? as usize;
            let body = data.get(at + 8..at + 8 + len)?;
            fields.insert(s.to_string(), String::from_utf8_lossy(body).into_owned());
            i = at + 8 + len;
            if data.get(i) == Some(&b'\n') {
                i += 1;
            }
        } else {
            return None;
        }
    }
    if !fields.is_empty() {
        entries.push(Entry { fields });
    }
    Some(Journal { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text() {
        let d = b"MESSAGE=a\nPRIORITY=6\n\nMESSAGE=b\n";
        let j = parse(d).unwrap();
        assert_eq!(j.entries.len(), 2);
        assert_eq!(j.entries[1].get("MESSAGE").map(String::as_str), Some("b"));
    }

    #[test]
    fn parses_binary() {
        let mut d = Vec::new();
        d.extend_from_slice(b"BINARY_FIELD\n");
        d.extend_from_slice(&3u64.to_le_bytes());
        d.extend_from_slice(b"\x00\x01\x02\nMESSAGE=x\n");
        let j = parse(&d).unwrap();
        assert_eq!(j.entries.len(), 1);
        assert_eq!(j.entries[0].get("MESSAGE").map(String::as_str), Some("x"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"lowercase=x\n").is_none());
        assert!(parse(b"NOEQSIGN\n").is_none()); // key_ok? no '=' and valid key → treated as binary name, then missing len → None
    }
}

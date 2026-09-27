//! Minimal reader for IGES (Initial Graphics Exchange Specification)
//! files: fixed 80-column records split into `S`tart, `G`lobal,
//! `D`irectory Entry, `P`arameter Data, and `T`erminate sections — the
//! section letter sits in column 73 and the sequence number in
//! columns 74–80. The T section carries the four section counts.
//!
//! ```
//! use izanagi_kit::iges::parse;
//!
//! // "S" record: 72 chars of text + 'S' + 7-digit sequence
//! let s = format!("{:72}S{:07}", "test file", 1);
//! let g = format!("{:72}G{:07}", "", 1);
//! let t = format!("{:72}T{:07}", "", 1);
//! let doc = format!("{s}\n{g}\n{t}\n");
//! let i = parse(doc.as_bytes()).unwrap();
//! assert_eq!(i.count(b'S'), 1);
//! ```

/// One 80-column record: the section letter and its payload (cols 1–72).
#[derive(Debug)]
pub struct Record {
    /// `S`, `G`, `D`, `P`, or `T`.
    pub section: u8,
    /// Sequence number from columns 74–80 (may be 0 on malformed files —
    /// but `parse` only accepts numeric fields).
    pub seq: u64,
    /// Columns 1–72 with trailing spaces trimmed.
    pub text: String,
}

/// A parsed IGES file.
#[derive(Debug)]
pub struct Iges {
    /// All records in file order.
    pub records: Vec<Record>,
}

impl Iges {
    /// Number of records in `section` (`b'S'`, `b'G'`, `b'D'`, `b'P'`, `b'T'`).
    pub fn count(&self, section: u8) -> usize {
        self.records.iter().filter(|r| r.section == section).count()
    }

    /// T-section counts `(S, G, D, P)` when a Terminate record exists —
    /// its 72-col text is four space-separated integers.
    pub fn terminate(&self) -> Option<[u64; 4]> {
        let t = self.records.iter().find(|r| r.section == b'T')?;
        let mut out = [0u64; 4];
        let mut w = t.text.split_whitespace();
        for slot in &mut out {
            *slot = w.next()?.parse().ok()?;
        }
        Some(out)
    }
}

/// Parse an IGES file. `None` when any line isn't an 80-col record with a
/// valid section letter + numeric sequence, or when no `T` record exists.
pub fn parse(data: &[u8]) -> Option<Iges> {
    let text = std::str::from_utf8(data).ok()?;
    let mut records = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.len() != 80 {
            return None;
        }
        let b = line.as_bytes();
        let section = b[72];
        if !matches!(section, b'S' | b'G' | b'D' | b'P' | b'T') {
            return None;
        }
        let seq_text = std::str::from_utf8(&b[73..]).ok()?.trim();
        let seq: u64 = seq_text.parse().ok()?;
        records.push(Record {
            section,
            seq,
            text: line[..72].trim_end().to_string(),
        });
    }
    let ig = Iges { records };
    ig.records.iter().find(|r| r.section == b'T')?;
    Some(ig)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(sec: u8, text: &str, seq: u64) -> String {
        format!("{:72}{}{:07}", text, sec as char, seq)
    }

    #[test]
    fn parses() {
        let doc = format!(
            "{}\n{}\n{}\n{}\n",
            rec(b'S', "iges demo", 1),
            rec(b'G', ",,,;", 1),
            rec(b'D', "     110       1", 1),
            rec(b'T', "1 1 1 0", 1),
        );
        let i = parse(doc.as_bytes()).unwrap();
        assert_eq!(i.count(b'S'), 1);
        assert_eq!(i.count(b'D'), 1);
        assert_eq!(i.terminate(), Some([1, 1, 1, 0]));
        assert_eq!(i.records[0].seq, 1);
        assert_eq!(i.records[0].text, "iges demo");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none()); // no T record
                                       // wrong width
        assert!(parse(b"short line\n").is_none());
        // bad section letter at col 73
        let bad = format!("{:72}X{:07}\n{:72}T{:07}\n", "", 1, "", 1);
        assert!(parse(bad.as_bytes()).is_none());
        // non-numeric sequence
        let bad2 = format!("{:72}S{:>7}\n{:72}T{:07}\n", "", "abc", "", 1);
        assert!(parse(bad2.as_bytes()).is_none());
    }
}

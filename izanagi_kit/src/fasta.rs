//! FASTA sequence format — `>` defline followed by wrapped sequence
//! lines, multi-record per file.
//!
//! A defline is `>` + id (first whitespace-delimited token) +
//! description (the rest of the line). Sequence lines carry IUPAC
//! letters; whitespace and digits are stripped on collect.
//!
//! ```
//! use izanagi_kit::fasta::records;
//!
//! let d = b">seq1 first record\nACGT\nacgt\n>seq2\nNN\n";
//! let r: Vec<_> = records(d).collect();
//! assert_eq!(r.len(), 2);
//! assert_eq!(r[0].id, "seq1");
//! assert_eq!(r[0].description, "first record");
//! assert_eq!(r[0].sequence, b"ACGTacgt");
//! ```

use std::vec::Vec;

/// One FASTA record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    /// First whitespace-delimited token after `>`.
    pub id: &'a str,
    /// Remainder of the defline after the id (may be empty).
    pub description: &'a str,
    /// Sequence letters with whitespace/digits removed (case kept).
    pub sequence: Vec<u8>,
}

/// True when the data starts a FASTA record (`>` at the start).
pub fn is_fasta(d: &[u8]) -> bool {
    d.first() == Some(&b'>')
}

/// Iterate over every record; a file with no `>` yields none.
pub fn records(d: &[u8]) -> Records<'_> {
    Records { d, at: 0 }
}

/// Record iterator over a FASTA file.
pub struct Records<'a> {
    d: &'a [u8],
    at: usize,
}

fn line(d: &[u8], at: usize) -> (&[u8], usize) {
    let end = d[at..]
        .iter()
        .position(|&c| c == b'\n')
        .map(|p| at + p)
        .unwrap_or(d.len());
    let mut l = &d[at..end];
    if l.last() == Some(&b'\r') {
        l = &l[..l.len() - 1];
    }
    (l, end + usize::from(end < d.len()))
}

impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;
    fn next(&mut self) -> Option<Record<'a>> {
        // skip junk before the first defline
        loop {
            if self.at >= self.d.len() {
                return None;
            }
            let (l, next) = line(self.d, self.at);
            self.at = next;
            if l.first() == Some(&b'>') {
                let def = std::str::from_utf8(&l[1..]).unwrap_or("");
                let mut it = def.splitn(2, char::is_whitespace);
                let id = it.next().unwrap_or("");
                let description = it.next().unwrap_or("").trim();
                let mut sequence = Vec::new();
                loop {
                    if self.at >= self.d.len() {
                        break;
                    }
                    let (s, n) = line(self.d, self.at);
                    if s.first() == Some(&b'>') {
                        break;
                    }
                    self.at = n;
                    sequence.extend(
                        s.iter()
                            .filter(|c| c.is_ascii_alphabetic() || **c == b'*' || **c == b'-')
                            .copied(),
                    );
                }
                return Some(Record {
                    id,
                    description,
                    sequence,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b">a d1\nACG\n>b\nTT\n>c\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].id, "a");
        assert_eq!(r[0].description, "d1");
        assert_eq!(r[0].sequence, b"ACG");
        assert_eq!(r[1].sequence, b"TT");
        assert_eq!(r[2].sequence, b"");
    }

    #[test]
    fn junk_and_gaps() {
        // bytes before the first `>` and blank lines are skipped
        let d = b"junk\n\n>x\nA C\tG12\nT*-\n>y\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].sequence, b"ACGT*-");
        assert!(!is_fasta(d)); // starts with 'j'
        assert!(is_fasta(b">x\n"));
        assert!(!is_fasta(b""));
        assert_eq!(records(b"no records").count(), 0);
    }

    #[test]
    fn no_description() {
        let d = b">only_id\nA\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r[0].description, "");
    }
}

//! GFF3 — Generic Feature Format v3: nine tab-separated columns
//! (`seqid source type start end score strand phase attributes`),
//! `##` pragma directives, `#` comments, optional `##FASTA` tail.
//!
//! `start`/`end` are 1-based inclusive; missing values are `.`.
//! `score`/`phase` are kept as raw text so no floating-point state
//! enters the parser.
//!
//! ```
//! use izanagi_kit::gff::{records, directives, Strand};
//!
//! let d = b"##gff-version 3\nseq1\tsrc\tgene\t100\t900\t.\t+\t.\tID=g1\n";
//! assert_eq!(directives(d).next(), Some("gff-version 3"));
//! let r: Vec<_> = records(d).collect();
//! assert_eq!(r[0].start, 100);
//! assert_eq!(r[0].strand, Strand::Plus);
//! ```

use std::vec::Vec;

/// Column 7 strand value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strand {
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `.` — feature is not stranded
    Unstranded,
    /// `?` — stranded but direction unknown
    Unknown,
}

/// One GFF3 record line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    /// Column 1: sequence id.
    pub seqid: &'a str,
    /// Column 2: source.
    pub source: &'a str,
    /// Column 3: feature type.
    pub kind: &'a str,
    /// Column 4: 1-based start.
    pub start: u64,
    /// Column 5: inclusive end.
    pub end: u64,
    /// Column 6: score text (`.` when absent).
    pub score: &'a str,
    /// Column 7: strand.
    pub strand: Strand,
    /// Column 8: phase text (`.`, `0`, `1`, `2`).
    pub phase: &'a str,
    /// Column 9: `key=value;…` attribute blob.
    pub attributes: &'a str,
}

impl<'a> Record<'a> {
    /// First value of an `key=` attribute (`;`-separated).
    pub fn attribute(&self, key: &str) -> Option<&'a str> {
        for kv in self.attributes.split(';') {
            let mut it = kv.splitn(2, '=');
            if it.next() == Some(key) {
                return it.next();
            }
        }
        None
    }
}

/// `##` pragma lines (without the `##`).
pub fn directives(d: &[u8]) -> Directives<'_> {
    Directives { d, at: 0 }
}

/// Data records; comments, pragmas and the `##FASTA` tail are skipped.
pub fn records(d: &[u8]) -> Records<'_> {
    Records { d, at: 0 }
}

/// Iterator over `##` directive lines.
pub struct Directives<'a> {
    d: &'a [u8],
    at: usize,
}

/// Iterator over data rows.
pub struct Records<'a> {
    d: &'a [u8],
    at: usize,
}

fn next_line<'a>(d: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
    if *at >= d.len() {
        return None;
    }
    let start = *at;
    let end = d[start..]
        .iter()
        .position(|&c| c == b'\n')
        .map(|p| start + p)
        .unwrap_or(d.len());
    *at = end + usize::from(end < d.len());
    let mut l = &d[start..end];
    if l.last() == Some(&b'\r') {
        l = &l[..l.len() - 1];
    }
    Some(l)
}

impl<'a> Iterator for Directives<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        while let Some(l) = next_line(self.d, &mut self.at) {
            if l == b"##FASTA" {
                return None;
            }
            if l.starts_with(b"##") {
                return Some(std::str::from_utf8(&l[2..]).unwrap_or(""));
            }
        }
        None
    }
}

impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;
    fn next(&mut self) -> Option<Record<'a>> {
        while let Some(l) = next_line(self.d, &mut self.at) {
            if l.is_empty() || l.first() == Some(&b'#') || l.first() == Some(&b'>') {
                if l == b"##FASTA" {
                    return None;
                }
                continue;
            }
            let t = std::str::from_utf8(l).unwrap_or("");
            let f: Vec<&str> = t.split('\t').collect();
            if f.len() < 9 {
                continue; // malformed row
            }
            let (start, end) = match (f[3].parse::<u64>().ok(), f[4].parse::<u64>().ok()) {
                (Some(s), Some(e)) => (s, e),
                _ => continue,
            };
            let strand = match f[6] {
                "+" => Strand::Plus,
                "-" => Strand::Minus,
                "." => Strand::Unstranded,
                _ => Strand::Unknown,
            };
            return Some(Record {
                seqid: f[0],
                source: f[1],
                kind: f[2],
                start,
                end,
                score: f[5],
                strand,
                phase: f[7],
                attributes: f[8],
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_row() {
        let d = b"##gff-version 3\n# comment\nchr1\tsrc\tmRNA\t1\t10\t50\t-\t2\tID=a;Name=b\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r.len(), 1);
        let r = &r[0];
        assert_eq!(r.seqid, "chr1");
        assert_eq!(r.kind, "mRNA");
        assert_eq!((r.start, r.end), (1, 10));
        assert_eq!(r.score, "50");
        assert_eq!(r.strand, Strand::Minus);
        assert_eq!(r.phase, "2");
        assert_eq!(r.attribute("ID"), Some("a"));
        assert_eq!(r.attribute("Name"), Some("b"));
        assert_eq!(r.attribute("nope"), None);
    }

    #[test]
    fn fasta_tail_and_junk() {
        let d = b"chr1\ts\tg\t1\t2\t.\t.\t.\tID=x\n##FASTA\n>chr1\nACGT\nchr2\ts\tg\t1\t2\t.\t.\t.\tID=y\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r.len(), 1);
        // malformed rows skipped
        assert_eq!(records(b"chr1\ts\tg\tnan\t2\t.\t.\t.\tID=x\n").count(), 0);
        assert_eq!(records(b"too\tfew\n").count(), 0);
    }

    #[test]
    fn directive_list() {
        let d = b"##gff-version 3\n##sequence-region chr1 1 100\nchr1\ts\tg\t1\t2\t.\t.\t.\tID=x\n";
        let v: Vec<_> = directives(d).collect();
        assert_eq!(v, ["gff-version 3", "sequence-region chr1 1 100"]);
    }
}

//! BED — UCSC Genome Browser track format: `chrom start end` plus up
//! to nine optional columns (name, score, strand, thickStart,
//! thickEnd, itemRgb, blockCount, blockSizes, blockStarts).
//!
//! Lines are tab-separated (`browser`/`track` headers, `#` comments
//! and blank lines are skipped). `start` is 0-based, `end` exclusive.
//! Only the three mandatory columns are parsed; optional columns are
//! exposed verbatim through [`Record::extra`].
//!
//! ```
//! use izanagi_kit::bed::records;
//!
//! let d = b"track name=t\nchr1\t10\t20\tgeneA\t900\t+\nchr2\t0\t5\n";
//! let r: Vec<_> = records(d).collect();
//! assert_eq!(r.len(), 2);
//! assert_eq!((r[0].start, r[0].end), (10, 20));
//! assert_eq!(r[0].extra(0), Some("geneA"));
//! assert_eq!(r[1].extra(0), None);
//! ```

use std::vec::Vec;

/// One BED record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    /// Column 1: chromosome/contig name.
    pub chrom: &'a str,
    /// Column 2: 0-based start.
    pub start: u64,
    /// Column 3: exclusive end.
    pub end: u64,
    /// Columns 4..12 verbatim (`name score strand thick …`).
    pub extras: Vec<&'a str>,
}

impl<'a> Record<'a> {
    /// Optional column `4 + n` (0 = name, 1 = score, …).
    pub fn extra(&self, n: usize) -> Option<&'a str> {
        self.extras.get(n).copied()
    }
}

/// Iterate BED records, skipping headers, comments and blank lines.
pub fn records(d: &[u8]) -> Records<'_> {
    Records { d, at: 0 }
}

/// Record iterator over a BED file.
pub struct Records<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;
    fn next(&mut self) -> Option<Record<'a>> {
        while self.at < self.d.len() {
            let start = self.at;
            let end = self.d[start..]
                .iter()
                .position(|&c| c == b'\n')
                .map(|p| start + p)
                .unwrap_or(self.d.len());
            self.at = end + usize::from(end < self.d.len());
            let mut l = &self.d[start..end];
            if l.last() == Some(&b'\r') {
                l = &l[..l.len() - 1];
            }
            if l.is_empty()
                || l.first() == Some(&b'#')
                || l.starts_with(b"track")
                || l.starts_with(b"browser")
            {
                continue;
            }
            let t = std::str::from_utf8(l).unwrap_or("");
            let f: Vec<&str> = if t.contains('\t') {
                t.split('\t').collect()
            } else {
                t.split_whitespace().collect()
            };
            if f.len() < 3 {
                continue;
            }
            let (s, e) = match (f[1].parse::<u64>().ok(), f[2].parse::<u64>().ok()) {
                (Some(s), Some(e)) => (s, e),
                _ => continue,
            };
            return Some(Record {
                chrom: f[0],
                start: s,
                end: e,
                extras: f[3..].to_vec(),
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandatory_and_extras() {
        let d = b"chr1\t0\t10\ta\t60\t-\t0\t10\t255,0,0\t2\t1,2\t0,5\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].chrom, "chr1");
        assert_eq!(r[0].extras.len(), 9);
        assert_eq!(r[0].extra(8), Some("0,5"));
    }

    #[test]
    fn whitespace_and_skips() {
        let d = b"# c\nbrowser pos chr1\ntrack x\n\nchr1 1 9\nbad\trow\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].start, r[0].end), (1, 9));
        assert_eq!(records(b"").count(), 0);
        assert_eq!(records(b"chr1\tx\t9\n").count(), 0);
    }
}

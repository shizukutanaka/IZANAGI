//! FASTQ — Sanger/Illumina read format: `@id` header, sequence lines,
//! `+` separator, then quality lines totalling the sequence length.
//!
//! Sequence may wrap over several lines; quality is collected until it
//! reaches the sequence length (Cock et al. 2009). Truncated or
//! malformed records terminate the iterator.
//!
//! ```
//! use izanagi_kit::fastq::{records, phred33};
//!
//! let d = b"@read1 desc\nACGT\n+\nIIII\n@r2\nNN\n+\n##\n";
//! let r: Vec<_> = records(d).collect();
//! assert_eq!(r.len(), 2);
//! assert_eq!(r[0].id, "read1");
//! assert_eq!(r[0].quality, b"IIII");
//! assert_eq!(phred33(b'I'), 40);
//! ```

/// One FASTQ record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    /// First whitespace-delimited token after `@`.
    pub id: &'a str,
    /// Remainder of the header line after the id (may be empty).
    pub description: &'a str,
    /// Concatenated sequence lines (case kept).
    pub sequence: Vec<u8>,
    /// Concatenated quality bytes; `sequence.len() == quality.len()`.
    pub quality: Vec<u8>,
}

/// Sanger Phred+33 quality value of one quality byte.
pub fn phred33(q: u8) -> u8 {
    q.saturating_sub(33)
}

/// Solexa/Illumina-1.0 Phred+64 quality value of one quality byte.
pub fn phred64(q: u8) -> u8 {
    q.saturating_sub(64)
}

/// Iterate every complete record; trailing junk is ignored.
pub fn records(d: &[u8]) -> Records<'_> {
    Records { d, at: 0 }
}

/// Record iterator over a FASTQ file.
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
        loop {
            if self.at >= self.d.len() {
                return None;
            }
            let (h, next) = line(self.d, self.at);
            self.at = next;
            if h.first() != Some(&b'@') {
                continue; // junk line
            }
            let def = std::str::from_utf8(&h[1..]).unwrap_or("");
            let mut it = def.splitn(2, char::is_whitespace);
            let id = it.next().unwrap_or("");
            let description = it.next().unwrap_or("").trim();

            let mut sequence = Vec::new();
            loop {
                if self.at >= self.d.len() {
                    return None; // truncated
                }
                let (s, n) = line(self.d, self.at);
                self.at = n;
                if s.first() == Some(&b'+') {
                    break;
                }
                sequence.extend_from_slice(s);
            }
            let mut quality = Vec::new();
            while quality.len() < sequence.len() {
                if self.at >= self.d.len() {
                    return None; // truncated quality
                }
                let (q, n) = line(self.d, self.at);
                self.at = n;
                quality.extend_from_slice(q);
            }
            return Some(Record {
                id,
                description,
                sequence,
                quality,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_and_wrap() {
        let d = b"@r1\nAC\nGT\n+r1\nII\nII\n";
        let r: Vec<_> = records(d).collect();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].sequence, b"ACGT");
        assert_eq!(r[0].quality, b"IIII");
    }

    #[test]
    fn truncation_stops() {
        assert_eq!(records(b"@r\nACGT\n+\nII").count(), 0);
        assert_eq!(records(b"@r\nACGT").count(), 0);
        assert_eq!(records(b"garbage\n").count(), 0);
    }

    #[test]
    fn phred() {
        assert_eq!(phred33(b'!'), 0);
        assert_eq!(phred33(b'I'), 40);
        assert_eq!(phred64(b'B'), 2);
        assert_eq!(phred64(b'@'), 0);
        assert_eq!(phred64(10), 0);
    }
}

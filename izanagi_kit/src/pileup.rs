//! SAMtools mpileup text output (samtools `mpileup` doc): one
//! line per reference position — `seq pos ref depth readbases
//! basequals [mapquals …]` where `readbases` uses `.`,`,` for
//! ref-match, `ACGTNacgtn` mismatches, `^<char>` read-start +
//! mapq, `$` read-end, `+N<seq>`/`-N<seq>` indels, `*` deletion
//! placeholders.
//!
//! ```
//! let d = b"seq1\t181\tA\t11\t.,,,,,..,.,.\tFFFFFFFFFFF\n";
//! let p = izanagi_kit::pileup::parse(d).unwrap();
//! assert_eq!(p.rows, 1);
//! assert_eq!(p.depth, 11);
//! assert!(izanagi_kit::pileup::detect(d));
//! ```

/// Census of an mpileup stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Pileup {
    /// Position rows parsed.
    pub rows: u32,
    /// Distinct reference names (col 1).
    pub seqs: u32,
    /// Sum of `depth` column.
    pub depth: u64,
    /// Largest single-row depth.
    pub max_depth: u32,
    /// Rows carrying an indel marker (`+N`/`-N` inside col 5).
    pub indels: u32,
    /// Rows with at least one `*` deletion placeholder.
    pub deletions: u32,
    /// Rows whose base column contains a ref-match only
    /// (`.`/`,` and markers but no mismatch letter).
    pub ref_only: u32,
    /// Rows with a 7th+ column (mapq or extra annotations).
    pub extra_cols: u32,
}

fn numeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

fn bases_ok(s: &str) -> bool {
    // The column carries ref-matches (.,), mismatches (ACGTNacgtn),
    // `^`+one printable mapq char, `$`, `+N<seq>`/`-N<seq>` indels
    // and `*` placeholders — any printable, non-whitespace text with
    // at least one base/marker symbol qualifies.
    !s.is_empty()
        && s.bytes().all(|c| (33..=126).contains(&c))
        && s.bytes().any(|c| {
            c.is_ascii_alphanumeric() || matches!(c, b'.' | b',' | b'^' | b'$' | b'*' | b'+' | b'-')
        })
}

fn is_row(line: &str) -> bool {
    let f: Vec<&str> = line.split('\t').collect();
    f.len() >= 6
        && !f[0].is_empty()
        && numeric(f[1])
        && f[2].chars().count() == 1
        && matches!(
            f[2].chars().next(),
            Some('A' | 'C' | 'G' | 'T' | 'N' | 'a' | 'c' | 'g' | 't' | 'n' | '*' | '>' | '<')
        )
        && numeric(f[3])
        && bases_ok(f[4])
        && !f[5].is_empty()
}

/// `true` when a majority of non-empty lines match the shape.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut good = 0u32;
    let mut lines = 0u32;
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        lines += 1;
        if is_row(line) {
            good += 1;
        }
    }
    good >= 1 && good * 2 >= lines
}

/// Census; `None` without at least one well-formed row.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Pileup> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut p = Pileup {
        rows: 0,
        seqs: 0,
        depth: 0,
        max_depth: 0,
        indels: 0,
        deletions: 0,
        ref_only: 0,
        extra_cols: 0,
    };
    let mut seq_names = Vec::new();
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if !is_row(line) {
            continue;
        }
        p.rows += 1;
        let f: Vec<&str> = line.split('\t').collect();
        if !seq_names.iter().any(|n| *n == f[0]) {
            seq_names.push(f[0].to_string());
        }
        let d: u32 = f[3].parse().unwrap_or(0);
        p.depth += d as u64;
        p.max_depth = p.max_depth.max(d);
        let bases = f[4];
        if bases.contains('+') || bases.contains('-') {
            p.indels += 1;
        }
        if bases.contains('*') {
            p.deletions += 1;
        }
        if !bases
            .bytes()
            .any(|c| matches!(c, b'A' | b'C' | b'G' | b'T' | b'a' | b'c' | b'g' | b't'))
        {
            p.ref_only += 1;
        }
        if f.len() > 6 {
            p.extra_cols += 1;
        }
    }
    p.seqs = seq_names.len() as u32;
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"seq1\t181\tA\t11\t.,,,,,..,.,.\tFFFFFFFFFFF\n\
seq1\t182\tT\t12\t.,$,,,,..,.,.+2TT\tFFFFFFFFFFFF\tGGGGGGGGGGGG\n\
seq1\t183\tG\t13\t.,,,,.,,^~.\tEEEEEEEEEEEEE\n\
seq2\t10\tC\t3\t**A\tFFF\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"not a pileup"));
        assert!(!detect(b"seq\tpos"));
    }

    #[test]
    fn parses() {
        let p = parse(D).unwrap();
        assert_eq!(p.rows, 4);
        assert_eq!(p.seqs, 2);
        assert_eq!(p.depth, 39);
        assert_eq!(p.max_depth, 13);
        assert_eq!(p.indels, 1);
        assert_eq!(p.deletions, 1);
        assert_eq!(p.ref_only, 2);
        assert_eq!(p.extra_cols, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

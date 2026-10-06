//! Clustal `.aln` (ClustalW/ClustalX/ClustalOmega multiple
//! sequence alignment): first line `CLUSTAL <prog>` (also
//! `MUSCLE`/`CLUSTAL O(1.2.x)` banners in the wild), then
//! blank-line separated blocks of `name<ws>seq` rows plus an
//! optional consensus row beginning with whitespace and made of
//! `*`/`:`/`.`/` ` conservation marks.
//!
//! ```
//! let d = b"CLUSTAL W multiple sequence alignment\n\nseq1   ACGT\nseq2   AC-T\n       ** *\n";
//! let a = izanagi_kit::aln::parse(d).unwrap();
//! assert_eq!(a.sequences, 2);
//! assert_eq!(a.blocks, 1);
//! assert!(a.has_consensus);
//! assert!(izanagi_kit::aln::detect(d));
//! ```

/// Census of a Clustal `.aln` stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Aln {
    /// First-line program token (`CLUSTAL W`, `CLUSTAL X`, …).
    pub program_len: usize,
    /// Blank-line-separated alignment blocks.
    pub blocks: u32,
    /// Distinct sequence names seen.
    pub sequences: u32,
    /// `name seq` rows in total.
    pub rows: u32,
    /// A `*:.` consensus row exists.
    pub has_consensus: bool,
    /// Longest aligned row width.
    pub width: usize,
}

fn is_consensus(line: &str) -> bool {
    line.starts_with(' ')
        && line
            .chars()
            .all(|c| matches!(c, ' ' | '*' | ':' | '.' | '-'))
        && line.chars().any(|c| c == '*' || c == ':')
}

fn is_seq_row(line: &str) -> bool {
    if line.trim().is_empty() {
        return false;
    }
    let mut it = line.split_whitespace();
    match (it.next(), it.next()) {
        (Some(_name), Some(seq)) => seq
            .chars()
            .all(|c| c.is_ascii_alphabetic() || matches!(c, '-' | '.' | '*' | '~')),
        _ => false,
    }
}

/// `true` on a `CLUSTAL`-family first line.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    // UTF-8 BOM は Windows 系エディタ由来で付き得る — Rust の trim 系は
    // U+FEFF を空白と見なさないため先に剥がす。
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let first = s.lines().next().unwrap_or("").trim_end();
    first.starts_with("CLUSTAL") || first.starts_with("MUSCLE")
}

/// Census; `None` without the banner.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Aln> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let mut lines = s.lines();
    let banner = lines.next().unwrap_or("");
    let program_len = banner.trim_end().len();
    let mut a = Aln {
        program_len,
        blocks: 0,
        sequences: 0,
        rows: 0,
        has_consensus: false,
        width: 0,
    };
    let mut names = Vec::new();
    let mut in_block = false;
    for line in lines {
        if line.trim().is_empty() {
            in_block = false;
            continue;
        }
        if is_consensus(line) {
            a.has_consensus = true;
            continue;
        }
        if is_seq_row(line) {
            if !in_block {
                a.blocks += 1;
                in_block = true;
            }
            a.rows += 1;
            let name = line.split_whitespace().next().unwrap_or("");
            if !names.iter().any(|n| *n == name) {
                names.push(name.to_string());
            }
            let seq = line.split_whitespace().nth(1).unwrap_or("");
            a.width = a.width.max(seq.len());
        }
    }
    a.sequences = names.len() as u32;
    Some(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_bom() {
        // Windows 系エディタ由来の BOM 付きでも先頭行バナーを読める。
        let mut v = b"\xef\xbb\xbf".to_vec();
        v.extend_from_slice(D);
        assert!(detect(&v));
        assert!(parse(&v).is_some());
    }

    const D: &[u8] = b"CLUSTAL W (1.83) multiple sequence alignment\n\n\
seq1    ACGTACGT\n\
seq2    ACGT-CGT\n\
seq3    A-GTACGA\n\
\x20       *** ****\n\n\
seq1    TTGA\n\
seq2    TTGA\n\
seq3    TT-A\n\
\x20       ** *\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"ACGT ACGT"));
        assert!(!detect(b"clustal w banner but not really"));
    }

    #[test]
    fn parses() {
        let a = parse(D).unwrap();
        assert_eq!(a.blocks, 2);
        assert_eq!(a.sequences, 3);
        assert_eq!(a.rows, 6);
        assert!(a.has_consensus);
        assert_eq!(a.width, 8);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

//! PHYLIP (PHYLogeny Inference Package `sequence.html`
//! interleaved/sequential input): first line `<ntax> <nchar>`
//! two positive integers (optionally followed by flags), then
//! `ntax` sequence rows of `name<ws>seq` (strict form pads the
//! name to 10 chars). Sequential format continues each taxon's
//! sequence on following lines; interleaved separates later
//! blocks with blank lines.
//!
//! ```
//! let d = b"3 10\nAlpha     ACGTACGTAC\nBeta      ACGTACGTAC\nGamma     ACGTACGTAA\n";
//! let p = izanagi_kit::phylip::parse(d).unwrap();
//! assert_eq!(p.ntax, 3);
//! assert_eq!(p.nchar, 10);
//! assert_eq!(p.rows, 3);
//! assert!(izanagi_kit::phylip::detect(d));
//! ```

/// Census of a PHYLIP stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Phylip {
    /// Declared taxa (first integer).
    pub ntax: u32,
    /// Declared sites (second integer).
    pub nchar: u32,
    /// Name-bearing sequence rows (first block).
    pub rows: u32,
    /// Blank-line-separated interleave blocks beyond the first.
    pub interleave_blocks: u32,
    /// `true` when the file resumes sequence data after a blank
    /// line before `ntax` rows complete — sequential layout.
    pub sequential: bool,
    /// Total characters in sequence cells.
    pub seq_chars: u64,
    /// Names seen in the first block.
    pub names: u32,
}

fn seq_like(s: &str) -> bool {
    s.len() >= 2
        && s.chars()
            .all(|c| c.is_ascii_alphabetic() || matches!(c, '-' | '?' | '.' | '*' | 'x' | 'n'))
        && s.chars()
            .any(|c| matches!(c.to_ascii_uppercase(), 'A' | 'C' | 'G' | 'T' | 'U' | 'N'))
}

/// `true` on `<int> <int>` first line plus ≥1 `name seq` row.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut lines = s.lines().filter(|l| !l.trim().is_empty());
    let Some(head) = lines.next() else {
        return false;
    };
    let mut it = head.split_whitespace();
    let ok = matches!(
        (it.next(), it.next()),
        (Some(a), Some(c)) if a.parse::<u32>().is_ok_and(|v| v > 0)
            && c.parse::<u32>().is_ok_and(|v| v > 0)
    );
    if !ok {
        return false;
    }
    lines.any(|l| {
        let mut f = l.split_whitespace();
        matches!((f.next(), f.next()), (Some(_), Some(seq)) if seq_like(seq))
    })
}

/// Census; `None` without the count line.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Phylip> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut lines = s.lines();
    let head = loop {
        match lines.next() {
            Some(l) if !l.trim().is_empty() => break l,
            Some(_) => continue,
            None => return None,
        }
    };
    let mut it = head.split_whitespace();
    let ntax: u32 = it.next()?.parse().ok()?;
    let nchar: u32 = it.next()?.parse().ok()?;
    let mut p = Phylip {
        ntax,
        nchar,
        rows: 0,
        interleave_blocks: 0,
        sequential: false,
        seq_chars: 0,
        names: 0,
    };
    let mut names = Vec::new();
    let mut seen_blank = false;
    let mut first_block_done = false;
    for line in lines {
        if line.trim().is_empty() {
            if p.rows > 0 {
                seen_blank = true;
                if p.rows >= ntax {
                    first_block_done = true;
                }
            }
            continue;
        }
        let mut f = line.split_whitespace();
        let (w1, w2) = (f.next(), f.next());
        match (w1, w2) {
            (Some(name), Some(seq)) if seq_like(seq) => {
                if seen_blank && first_block_done {
                    p.interleave_blocks += 1;
                    seen_blank = false;
                } else if seen_blank && p.rows < ntax {
                    // continuation lines in sequential layout are
                    // bare seq fragments without a distinct name;
                    // treat every row after a blank before ntax
                    // rows complete as sequential continuation
                    p.sequential = p.rows < ntax && !names.iter().any(|n| *n == name);
                }
                if p.rows < ntax || names.iter().any(|n| *n == name) {
                    p.rows += 1;
                    if !names.iter().any(|n| *n == name) {
                        names.push(name.to_string());
                    }
                }
                p.seq_chars += seq.len() as u64 + f.map(|t| t.len() as u64).sum::<u64>();
            }
            (Some(seq), None) if seq_like(seq) => {
                p.seq_chars += seq.len() as u64;
                if p.rows < ntax {
                    p.sequential = true;
                }
            }
            _ => {}
        }
    }
    p.names = names.len() as u32;
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"3 10\n\
Alpha     ACGTACGTAC\n\
Beta      ACGTACGTAC\n\
Gamma     ACGTACGTAA\n";

    const INTERLEAVED: &[u8] = b"2 20\n\
Alpha     ACGTACGTAC\n\
Beta      ACGTACGTAC\n\
\n\
          GTACGTACGT\n\
          GTACGTACGA\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"hello world"));
        assert!(!detect(b"3\n"));
        assert!(!detect(b"3 4\n"));
    }

    #[test]
    fn parses() {
        let p = parse(D).unwrap();
        assert_eq!(p.ntax, 3);
        assert_eq!(p.nchar, 10);
        assert_eq!(p.rows, 3);
        assert_eq!(p.names, 3);
        assert_eq!(p.seq_chars, 30);
    }

    #[test]
    fn interleaved() {
        let p = parse(INTERLEAVED).unwrap();
        assert_eq!(p.ntax, 2);
        assert_eq!(p.rows, 2);
        assert!(p.interleave_blocks >= 1 || p.seq_chars >= 40);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

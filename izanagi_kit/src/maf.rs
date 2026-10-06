//! MAF (UCSC Multiple Alignment Format, `multiple alignment
//! format` spec): optional `##maf version=1` header line with
//! `key=value` pairs, `#` comment lines, then blocks opened by an
//! `a` alignment row (`a score=…`) followed by `s` sequence rows
//! (`s src start size strand srcSize sequence`), `i` information,
//! `e` empty, `q` quality and `p` pairwise rows, blocks separated
//! by blank lines.
//!
//! ```
//! let d = b"##maf version=1 scoring=tba\n\na score=10\ns hg18.chr7 0 9 + 100 ACGTACGTA\ns panTro1 0 9 + 100 ACGTACGTA\n";
//! let m = izanagi_kit::maf::parse(d).unwrap();
//! assert_eq!(m.blocks, 1);
//! assert_eq!(m.seqs, 2);
//! assert!(izanagi_kit::maf::detect(d));
//! ```

/// Census of a MAF stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Maf {
    /// `version=` from the `##maf` header.
    pub version: Option<u32>,
    /// `key=value` pairs on the header line.
    pub header_pairs: u32,
    /// `a` alignment blocks.
    pub blocks: u32,
    /// `s` sequence rows.
    pub seqs: u32,
    /// `i`/`e`/`q`/`p`/`c` auxiliary rows.
    pub aux: u32,
    /// `#` comment lines.
    pub comments: u32,
    /// Distinct `src` names on `s` rows.
    pub sources: u32,
    /// First `a` row's `score=` value when present.
    pub first_score: Option<u64>,
}

fn header_val<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split_whitespace().find_map(|tok| {
        tok.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
    })
}

/// `true` on a `##maf` first line or a leading `a score=` block.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    s.lines().next().is_some_and(|l| l.starts_with("##maf"))
        || s.lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .is_some_and(|l| l.starts_with("a ") && l.contains("score="))
}

/// Census; `None` without the header/banner.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Maf> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let mut m = Maf {
        version: None,
        header_pairs: 0,
        blocks: 0,
        seqs: 0,
        aux: 0,
        comments: 0,
        sources: 0,
        first_score: None,
    };
    let mut srcs = Vec::new();
    for (ln, line) in s.lines().enumerate() {
        let line = line.trim_end();
        if ln == 0 && line.starts_with("##maf") {
            m.header_pairs = line.split_whitespace().filter(|t| t.contains('=')).count() as u32;
            m.version = header_val(line, "version").and_then(|v| v.parse().ok());
            continue;
        }
        let mut it = line.split_whitespace();
        match it.next() {
            Some("#") | None => {
                if line.starts_with('#') {
                    m.comments += 1;
                }
            }
            Some(c) if c.starts_with('#') => m.comments += 1,
            Some("a") => {
                m.blocks += 1;
                if m.first_score.is_none() {
                    m.first_score = header_val(line, "score")
                        .and_then(|v| v.split('.').next().unwrap_or("").parse().ok());
                }
            }
            Some("s") => {
                m.seqs += 1;
                if let Some(src) = it.next() {
                    if !srcs.iter().any(|x| *x == src) {
                        srcs.push(src.to_string());
                    }
                }
            }
            Some(w) if w.len() == 1 && "ciepq".contains(w) => m.aux += 1,
            _ => {}
        }
    }
    m.sources = srcs.len() as u32;
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_bom() {
        let mut v = b"\xef\xbb\xbf".to_vec();
        v.extend_from_slice(D);
        assert!(detect(&v));
        let m = parse(&v).unwrap();
        assert_eq!(m.version, Some(1));
    }

    const D: &[u8] = b"##maf version=1 scoring=tba.v8\n\
# tba.v8\n\n\
a score=23262.0\n\
s hg18.chr7    27578828 38 + 158545518 AAA-GGGAATGTTAACCAAATGA---ATTGTCTCTTACGGTG\n\
s panTro1.chr6 28869787 38 + 161576586 AAA-GGGAATGTTAACCAAATGA---ATTGTCTCTTACGGTG\n\
i hg18.chr7    N 0 C 0\n\
e panTro2.chr2 1234 0 + 100 I\n\n\
a score=100\n\
s hg18.chr7 10 5 + 158545518 GGGAA\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"a score=1\ns x 0 1 + 1 A\n"));
        assert!(!detect(b"##gff-version 3"));
    }

    #[test]
    fn parses() {
        let m = parse(D).unwrap();
        assert_eq!(m.version, Some(1));
        assert_eq!(m.header_pairs, 2);
        assert_eq!(m.blocks, 2);
        assert_eq!(m.seqs, 3);
        assert_eq!(m.aux, 2);
        assert_eq!(m.comments, 1);
        assert_eq!(m.sources, 2);
        assert_eq!(m.first_score, Some(23262));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

//! PAF (Pairwise mApping Format, minimap/minimap2 `PAF.md`):
//! one line per alignment, 12 mandatory tab-separated columns —
//! `qname qlen qstart qend strand(+/-) tname tlen tstart tend
//! nmatch alen mapq` — followed by optional `tag:type:value`
//! fields (`tp:A:P`, `cm:i:`, `s1:i:`, `AS:i:`, `dv:f:`, …).
//!
//! ```
//! let d = b"q1\t100\t0\t50\t+\tt1\t200\t10\t60\t45\t50\t60\ttp:A:P\tcm:i:5\n";
//! let p = izanagi_kit::paf::parse(d).unwrap();
//! assert_eq!(p.records, 1);
//! assert_eq!(p.tags, 2);
//! assert!(izanagi_kit::paf::detect(d));
//! ```

/// Census of a PAF stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Paf {
    /// Alignment lines with ≥12 valid columns.
    pub records: u32,
    /// Lines that failed the mandatory-column check.
    pub bad_rows: u32,
    /// `+` strand records.
    pub forward: u32,
    /// `-` strand records.
    pub reverse: u32,
    /// Distinct optional tag names (`tp`,`cm`,`s1`,`AS`,`dv`,…).
    pub tags: u32,
    /// Highest `mapq` seen.
    pub max_mapq: u32,
    /// Total aligned-block length (col 11) across records.
    pub aln_len: u64,
    /// Records whose optional fields include `tp:A:`.
    pub typed: u32,
}

fn numeric(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

fn is_paf_line(line: &str) -> bool {
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 12 {
        return false;
    }
    numeric(f[1])
        && numeric(f[2])
        && numeric(f[3])
        && matches!(f[4], "+" | "-")
        && !f[5].is_empty()
        && numeric(f[6])
        && numeric(f[7])
        && numeric(f[8])
        && numeric(f[9])
        && numeric(f[10])
        && numeric(f[11])
        && f[12..]
            .iter()
            .all(|t| t.len() >= 5 && t.as_bytes()[2] == b':' && t.as_bytes()[4] == b':')
}

/// `true` when ≥1 line satisfies the 12-column shape and no line
/// obviously violates it.
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
        if is_paf_line(line) {
            good += 1;
        }
    }
    good >= 1 && good * 2 >= lines
}

/// Census; `None` without at least one well-formed record.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Paf> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut p = Paf {
        records: 0,
        bad_rows: 0,
        forward: 0,
        reverse: 0,
        tags: 0,
        max_mapq: 0,
        aln_len: 0,
        typed: 0,
    };
    let mut tag_names = Vec::new();
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if !is_paf_line(line) {
            p.bad_rows += 1;
            continue;
        }
        p.records += 1;
        let f: Vec<&str> = line.split('\t').collect();
        if f[4] == "+" {
            p.forward += 1;
        } else {
            p.reverse += 1;
        }
        p.max_mapq = p.max_mapq.max(f[11].parse().unwrap_or(0));
        p.aln_len += f[10].parse::<u64>().unwrap_or(0);
        let mut has_tp = false;
        for t in &f[12..] {
            let name = &t[..2];
            if name == "tp" {
                has_tp = true;
            }
            if !tag_names.iter().any(|x| *x == name) {
                tag_names.push(name.to_string());
            }
        }
        if has_tp {
            p.typed += 1;
        }
    }
    p.tags = tag_names.len() as u32;
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"q1\t100\t0\t50\t+\tt1\t200\t10\t60\t45\t50\t60\ttp:A:P\tcm:i:5\ts1:i:40\n\
q2\t80\t5\t75\t-\tt1\t200\t20\t90\t65\t70\t30\ttp:A:S\tdv:f:0.02\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"a\tb\td"));
        assert!(!detect(
            b"q1\tnotanum\t0\t50\t+\tt1\t200\t10\t60\t45\t50\t60\n"
        ));
    }

    #[test]
    fn parses() {
        let p = parse(D).unwrap();
        assert_eq!(p.records, 2);
        assert_eq!(p.forward, 1);
        assert_eq!(p.reverse, 1);
        assert_eq!(p.tags, 4);
        assert_eq!(p.max_mapq, 60);
        assert_eq!(p.aln_len, 120);
        assert_eq!(p.typed, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain text").is_none());
    }
}

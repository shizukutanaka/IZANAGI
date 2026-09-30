//! Phoneme timing label (`*.lab`, HTS/monophone) census.
//!
//! Each non-empty line is either `start end phone` (times in 100ns units,
//! `start < end`) or a bare phone; every line must match one of the two
//! shapes and at least one timed line is required.
//!
//! ```
//! let s = b"0 1250000 sil\n1250000 2500000 a\n2500000 3750000 k\n";
//! assert!(izanagi_kit::lab::detect(s));
//! let l = izanagi_kit::lab::Lab::parse(s).unwrap();
//! assert_eq!(l.lines, 3);
//! assert_eq!(l.timed, 3);
//! assert_eq!(l.phones, 3);
//! assert_eq!(l.silences, 1);
//! assert_eq!(l.vowels, 1);
//! ```

/// Parsed census of a `*.lab` timing-label file.
#[derive(Debug, Clone)]
pub struct Lab {
    /// Non-empty label lines.
    pub lines: usize,
    /// `start end phone` lines.
    pub timed: usize,
    /// Bare `phone` lines.
    pub bare: usize,
    /// Distinct phone labels.
    pub phones: usize,
    /// Labels containing `sil` or `pau`.
    pub silences: usize,
    /// Single-letter vowels (`a`/`i`/`u`/`e`/`o`).
    pub vowels: usize,
    /// Single-letter consonants.
    pub consonants: usize,
    /// Full-context labels containing `-`/`+`/`/`/`:`/`^`.
    pub full_context: usize,
    /// Smallest start time.
    pub first_start: usize,
    /// Largest end time.
    pub last_end: usize,
}

fn shape_ok(t: &str) -> bool {
    let mut timed = 0usize;
    let mut saw = false;
    for l in t.lines() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        saw = true;
        let w: Vec<&str> = l.split_whitespace().collect();
        if w.len() == 3
            && w[0].bytes().all(|c| c.is_ascii_digit())
            && w[1].bytes().all(|c| c.is_ascii_digit())
            && w[0] < w[1]
        {
            timed += 1;
        } else if w.len() != 1 {
            return false;
        }
    }
    saw && timed > 0
}

/// Reports whether `b` looks like a `*.lab` timing-label file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    shape_ok(t)
}

impl Lab {
    /// Parses `b` as a label file, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !shape_ok(t) {
            return None;
        }
        let mut l = Lab {
            lines: 0,
            timed: 0,
            bare: 0,
            phones: 0,
            silences: 0,
            vowels: 0,
            consonants: 0,
            full_context: 0,
            first_start: 0,
            last_end: 0,
        };
        let mut set = std::collections::BTreeSet::new();
        let mut first: Option<usize> = None;
        for line in t.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            l.lines += 1;
            let w: Vec<&str> = line.split_whitespace().collect();
            let ph = if w.len() == 3 {
                l.timed += 1;
                let s: usize = w[0].parse().unwrap_or(0);
                let e: usize = w[1].parse().unwrap_or(0);
                first = Some(first.map_or(s, |f| f.min(s)));
                l.last_end = l.last_end.max(e);
                w[2]
            } else {
                l.bare += 1;
                w[0]
            };
            set.insert(ph);
            if ph.contains("sil") || ph.contains("pau") {
                l.silences += 1;
            }
            if ph.len() == 1 {
                if matches!(ph, "a" | "i" | "u" | "e" | "o") {
                    l.vowels += 1;
                } else if ph.bytes().all(|c| c.is_ascii_alphabetic()) {
                    l.consonants += 1;
                }
            }
            if ph.contains('-')
                || ph.contains('+')
                || ph.contains('/')
                || ph.contains(':')
                || ph.contains('^')
            {
                l.full_context += 1;
            }
        }
        l.phones = set.len();
        l.first_start = first.unwrap_or(0);
        Some(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"0 1250000 sil\n1250000 2500000 a\n2500000 3750000 k\n";

    #[test]
    fn parses_lab() {
        assert!(detect(S));
        let l = Lab::parse(S).unwrap();
        assert_eq!(l.lines, 3);
        assert_eq!(l.timed, 3);
        assert_eq!(l.phones, 3);
        assert_eq!(l.silences, 1);
        assert_eq!(l.vowels, 1);
    }

    #[test]
    fn rejects_non_lab() {
        assert!(!detect(b"a b c"));
        assert!(!detect(b"5 3 sil"));
        assert!(Lab::parse(b"").is_none());
    }
}

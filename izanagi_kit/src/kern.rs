//! Humdrum `**kern` spine data — tab-separated columns, `!` comments, `*` tandem
//! interpretations, `=` barlines, `-` spine terminator.
//!
//! ```
//! let d = b"**kern\n*clefG2\n*k[]\n*M4/4\n=1\n4c\n4d\n4e\n4f\n=2\n1g\n*-\n";
//! let s = izanagi_kit::kern::parse(d).unwrap();
//! assert_eq!(s.spines, 1);
//! assert_eq!(s.notes, 5);
//! assert_eq!(s.bars, 2);
//! assert!(izanagi_kit::kern::detect(d));
//! ```

/// A parsed `**kern` file summary.
#[derive(Debug, Clone)]
pub struct Kern {
    /// Number of `**kern` spines (columns).
    pub spines: usize,
    /// Number of barlines (`=` tokens) across all spines.
    pub bars: usize,
    /// Note tokens (start with a duration digit or `r` rest not counted).
    pub notes: usize,
    /// Rest tokens (`r`…).
    pub rests: usize,
    /// Tandem interpretations (`*…`) — clefs, keys, meters.
    pub tandems: usize,
    /// Comments (`!`-prefixed lines).
    pub comments: usize,
    /// Global / reference records (`!!!` lines).
    pub references: usize,
}

/// Splits a data token's duration prefix from its pitch class (`4c` → `("4", "c")`).
fn pitch_class(tok: &str) -> &str {
    tok.trim_start_matches(|c: char| c.is_ascii_digit())
}

fn is_note(tok: &str) -> bool {
    if !tok.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return false;
    }
    let p = pitch_class(tok);
    p.as_bytes()
        .first()
        .is_some_and(|c| c.is_ascii_lowercase() && (b'a'..=b'g').contains(c))
}

fn is_rest(tok: &str) -> bool {
    tok.as_bytes().first().is_some_and(u8::is_ascii_digit) && pitch_class(tok).starts_with('r')
}

/// Detects `**kern` data: a `**kern` exclusive-interpretation line.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .any(|l| l.split_whitespace().next() == Some("**kern"))
}

/// Parses `**kern` data; `None` on non-UTF-8 or missing `**kern` spine.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Kern> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Kern {
        spines: 0,
        bars: 0,
        notes: 0,
        rests: 0,
        tandems: 0,
        comments: 0,
        references: 0,
    };
    for line in t.lines() {
        let line = line.trim_end();
        if line.starts_with("!!") {
            s.references += 1;
            continue;
        }
        if line.starts_with('!') {
            s.comments += 1;
            continue;
        }
        if line.is_empty() {
            continue;
        }
        for tok in line.split('\t') {
            let tok = tok.trim();
            if tok.is_empty() || tok == "." {
                continue;
            }
            if tok == "**kern" {
                s.spines += 1;
            } else if tok.starts_with('*') {
                s.tandems += 1;
            } else if tok.starts_with('=') {
                s.bars += 1;
            } else if is_rest(tok) {
                s.rests += 1;
            } else if is_note(tok) {
                s.notes += 1;
            }
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: &[u8] = b"!!!OTL: Example\n**kern\t**dynam\n*clefG2\t*\n*k[]\t*\n*M4/4\t*\n!LO:TX:t=Allegro\n=1\t=1\n4c\tmf\n4d\t.\n4e\tf\n4r\t.\n=2\t=2\n1g\t.\n*-\t*-\n";

    #[test]
    fn parses() {
        let s = parse(K).unwrap();
        assert_eq!(s.spines, 1);
        assert_eq!(s.references, 1);
        assert_eq!(s.comments, 1);
        assert_eq!(s.bars, 4); // two `=` rows × 2 spines
        assert_eq!(s.notes, 4);
        assert_eq!(s.rests, 1);
        assert!(s.tandems > 0);
    }

    #[test]
    fn detect_works() {
        assert!(detect(K));
        assert!(!detect(b"**text\nhello\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\xff\xfe").is_none());
    }
}

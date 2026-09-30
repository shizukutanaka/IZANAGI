//! Scala `.scl` scale files — `!` comments, a description line, a degree
//! count, then one pitch per line: `numerator/denominator` ratios or
//! `cents`-style decimals (ending `.`).
//!
//! ```
//! let d = b"! 12-equal\n12-TET\n12\n!\n100\x2e0\n2/1\n200\x2e0\n300\x2e\n400\x2e0\n500\x2e0\n600\x2e0\n700\x2e0\n800\x2e0\n900\x2e0\n1000\x2e0\n1100\x2e0\n1200\x2e0\n";
//! let s = izanagi_kit::scl::parse(d).unwrap();
//! assert_eq!(s.degrees, 12);
//! assert_eq!(s.pitches, 13); // 12 cents lines + `2/1`
//! assert_eq!(s.ratios, 1);
//! assert_eq!(s.cents, 12);
//! assert!(izanagi_kit::scl::detect(d));
//! ```

/// A parsed `.scl` scale.
#[derive(Debug, Clone)]
pub struct Scl {
    /// The description line.
    pub name: String,
    /// Declared degree count.
    pub degrees: usize,
    /// Pitch lines parsed.
    pub pitches: usize,
    /// `num/den` ratio pitches.
    pub ratios: usize,
    /// Cents-style pitches (token containing `.` or a plain number).
    pub cents: usize,
    /// Comment lines (`!`).
    pub comments: usize,
}

fn pitch_kind(tok: &str) -> Option<bool> {
    // true = ratio, false = cents/plain
    if tok.is_empty() {
        return None;
    }
    if tok.contains('/') {
        let mut it = tok.split('/');
        let a = it.next()?;
        let b = it.next()?;
        if !a.is_empty()
            && a.chars().all(|c| c.is_ascii_digit())
            && !b.is_empty()
            && b.chars().all(|c| c.is_ascii_digit())
        {
            return Some(true);
        }
        return None;
    }
    // cents: digits with at most one dot, possibly trailing '.', negative allowed
    let body = tok.trim_start_matches('-');
    let dots = body.chars().filter(|&c| c == '.').count();
    if dots <= 1
        && body.chars().all(|c| c.is_ascii_digit() || c == '.')
        && body.chars().any(|c| c.is_ascii_digit())
    {
        return Some(false);
    }
    None
}

/// Detects `.scl`: a `!` comment then a name then a degree count then pitches.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut noncomment = t
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('!'));
    let Some(_name) = noncomment.next() else {
        return false;
    };
    let Some(deg) = noncomment.next() else {
        return false;
    };
    deg.chars().all(|c| c.is_ascii_digit())
        && noncomment
            .take(4)
            .any(|l| pitch_kind(l.split_whitespace().next().unwrap_or("")).is_some())
}

/// Parses a `.scl`; `None` on non-UTF-8 or missing pitch data.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Scl> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Scl {
        name: String::new(),
        degrees: 0,
        pitches: 0,
        ratios: 0,
        cents: 0,
        comments: 0,
    };
    let mut stage = 0; // 0 = need name, 1 = need degrees, 2 = pitches
    for raw in t.lines() {
        let l = raw.trim();
        if l.starts_with('!') {
            s.comments += 1;
            continue;
        }
        if l.is_empty() {
            continue;
        }
        match stage {
            0 => {
                s.name = l.to_string();
                stage = 1;
            }
            1 => {
                if l.chars().all(|c| c.is_ascii_digit()) {
                    s.degrees = l.parse().unwrap_or(0);
                    stage = 2;
                }
            }
            _ => {
                let tok = l.split_whitespace().next().unwrap_or("");
                match pitch_kind(tok) {
                    Some(true) => {
                        s.ratios += 1;
                        s.pitches += 1;
                    }
                    Some(false) => {
                        s.cents += 1;
                        s.pitches += 1;
                    }
                    None => {}
                }
            }
        }
    }
    (s.pitches > 0).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"! meantone\nMeantone\n3\n!\n1/1\n5/4\n2/1\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.name, "Meantone");
        assert_eq!(s.degrees, 3);
        assert_eq!(s.pitches, 3);
        assert_eq!(s.ratios, 3);
        assert_eq!(s.comments, 2);
    }

    #[test]
    fn cents_form() {
        let s = parse(b"12-TET\n2\n100\x2e0\n2/1\n").unwrap();
        assert_eq!(s.cents, 1);
        assert_eq!(s.ratios, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"# just text\nword\n42"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"name\n5\n").is_none());
    }
}

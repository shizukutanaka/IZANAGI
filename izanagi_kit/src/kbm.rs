//! Scala `.kbm` keyboard mapping — `!` comments then fixed-order fields:
//! map size, first/last MIDI note, middle note, reference note, reference
//! frequency, scale degree, and per-key remap lines.
//!
//! ```
//! let d = b"! 12 key map\n12\n0\n127\n60\n69\n440\x2e0\n0\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n";
//! let k = izanagi_kit::kbm::parse(d).unwrap();
//! assert_eq!(k.size, 12);
//! assert_eq!(k.ref_note, 69);
//! assert_eq!(k.entries, 12);
//! assert!(izanagi_kit::kbm::detect(d));
//! ```

/// A parsed `.kbm` keyboard mapping.
#[derive(Debug, Clone)]
pub struct Kbm {
    /// Declared map size (keys per octave cycle).
    pub size: usize,
    /// First MIDI note served (`-1` = unbounded low → stored as u8 255? kept raw).
    pub first_note: i32,
    /// Last MIDI note served.
    pub last_note: i32,
    /// Middle note where the scale degree lands.
    pub middle_note: i32,
    /// Reference note sounded by `ref_freq`.
    pub ref_note: i32,
    /// `ref_freq` as the integer part (frequencies carry a decimal).
    pub ref_freq_int: u32,
    /// Scale degree the mapping starts on.
    pub scale_degree: i32,
    /// Number of remap entries read (≤ size).
    pub entries: usize,
    /// Comment lines.
    pub comments: usize,
}

fn num(l: &str) -> Option<i32> {
    let l = l.trim();
    if l.is_empty()
        || !l
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == 'x')
    {
        return None;
    }
    if l == "x" || l == "-1" {
        return Some(-1);
    }
    l.parse().ok()
}

/// Detects `.kbm`: ≥7 numeric fields after comments, plausibly ordered.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let lines: Vec<&str> = t
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('!'))
        .collect();
    if lines.len() < 7 {
        return false;
    }
    // field 1 = size, 2/3 = note range (or x), 4 = middle, 5 = ref note,
    // 6 = freq (may carry '.'), 7 = scale degree
    num(lines[0]).is_some_and(|s| s > 0 && s <= 512)
        && num(lines[1]).is_some()
        && num(lines[2]).is_some()
        && num(lines[3]).is_some()
        && num(lines[4]).is_some()
        && lines[5].chars().all(|c| c.is_ascii_digit() || c == '.')
        && num(lines[6]).is_some()
}

/// Parses a `.kbm`; `None` on non-UTF-8 or malformed fields.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Kbm> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut comments = 0;
    let lines: Vec<&str> = t
        .lines()
        .map(|l| l.trim())
        .filter(|l| {
            if l.starts_with('!') {
                comments += 1;
            }
            !l.is_empty() && !l.starts_with('!')
        })
        .collect();
    let g = |i: usize| lines.get(i).copied().and_then(num);
    Some(Kbm {
        size: g(0)? as usize,
        first_note: g(1)?,
        last_note: g(2)?,
        middle_note: g(3)?,
        ref_note: g(4)?,
        ref_freq_int: lines
            .get(5)?
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        scale_degree: g(6)?,
        entries: lines.len().saturating_sub(7),
        comments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] =
        b"! map\n12\n0\n127\n60\n69\n440\x2e0\n0\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.size, 12);
        assert_eq!(s.first_note, 0);
        assert_eq!(s.last_note, 127);
        assert_eq!(s.middle_note, 60);
        assert_eq!(s.ref_note, 69);
        assert_eq!(s.ref_freq_int, 440);
        assert_eq!(s.scale_degree, 0);
        assert_eq!(s.entries, 12);
        assert_eq!(s.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"12\nx\nx\n60\n69\n440\x2e0\n0\n0\n"));
        assert!(!detect(b"name\nx\n"));
        assert!(!detect(b""));
        assert!(!detect(b"select * from t\n1\n2\n3\n4\n5\n6\n"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"1\n2\n3\n").is_none());
    }
}

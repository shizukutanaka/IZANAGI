//! MPL2 — Polish MicroDVD-style subtitle text: `[start][end]text` lines
//! with times in deciseconds (tenths of a second).
//!
//! ```
//! let d = b"[10][25]pierwszy|drugi\n[40][50]next\n";
//! let m = izanagi_kit::mpl2::parse(d).unwrap();
//! assert_eq!(m.cues, 2);
//! assert_eq!(m.first_start_ds, 10);
//! ```

/// Parsed MPL2 summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mpl2 {
    /// Valid `[start][end]…` cue lines.
    pub cues: usize,
    /// First cue start in deciseconds.
    pub first_start_ds: u32,
}

fn bracket(s: &str) -> Option<(u32, &str)> {
    let s = s.strip_prefix('[')?;
    let end = s.find(']')?;
    let v: u32 = s[..end].parse().ok()?;
    Some((v, &s[end + 1..]))
}

/// Parse an MPL2 file; `None` unless every content line is a cue.
pub fn parse(d: &[u8]) -> Option<Mpl2> {
    let s = std::str::from_utf8(d).ok()?;
    let mut cues = 0usize;
    let mut first_start_ds = 0u32;
    for l in s.lines() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        let (start, rest) = bracket(l)?;
        let (end, _text) = bracket(rest)?;
        if end <= start {
            return None;
        }
        if cues == 0 {
            first_start_ds = start;
        }
        cues += 1;
    }
    if cues == 0 {
        return None;
    }
    Some(Mpl2 {
        cues,
        first_start_ds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"[0][12]a|b\n[15][20]c\n";
        let m = parse(d).unwrap();
        assert_eq!((m.cues, m.first_start_ds), (2, 0));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[5][4]bad order\n").is_none());
        assert!(parse(b"not brackets\n").is_none());
        assert!(parse(b"[x][1]no\n").is_none());
    }
}

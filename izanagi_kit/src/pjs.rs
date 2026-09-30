//! PJS — Phoenix Japanimation Society subtitle text:
//! `H:MM:SS:FF, H:MM:SS:FF, "caption"` lines.
//!
//! ```
//! let d = b"0:00:01:00, 0:00:03:00, \"Hello\"\n0:00:04:00, 0:00:05:12, \"World\"\n";
//! let p = izanagi_kit::pjs::parse(d).unwrap();
//! assert_eq!(p.cues, 2);
//! assert_eq!(p.first_start_frame_ms, 1000);
//! ```

/// Parsed PJS summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pjs {
    /// Valid cue lines.
    pub cues: usize,
    /// First cue start in milliseconds (frames at 25 fps → `ff * 40`).
    pub first_start_frame_ms: u64,
}

/// `H:MM:SS:FF` → milliseconds at 25 fps.
fn tc(s: &str) -> Option<u64> {
    let mut p = s.trim().split(':');
    let h: u64 = p.next()?.parse().ok()?;
    let m: u64 = p.next()?.parse().ok()?;
    let sec: u64 = p.next()?.parse().ok()?;
    let f: u64 = p.next()?.parse().ok()?;
    if p.next().is_some() || m > 59 || sec > 59 {
        return None;
    }
    Some(h * 3_600_000 + m * 60_000 + sec * 1000 + f * 40)
}

/// Parse a PJS file; `None` unless every content line is a cue.
pub fn parse(d: &[u8]) -> Option<Pjs> {
    let s = std::str::from_utf8(d).ok()?;
    let mut cues = 0usize;
    let mut first_start_frame_ms = 0;
    for l in s.lines() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        let (a, rest) = l.split_once(", ")?;
        let (b, text) = rest.split_once(", ")?;
        if !text.starts_with('"') || !text.ends_with('"') {
            return None;
        }
        let (st, en) = (tc(a)?, tc(b)?);
        if en <= st {
            return None;
        }
        if cues == 0 {
            first_start_frame_ms = st;
        }
        cues += 1;
    }
    if cues == 0 {
        return None;
    }
    Some(Pjs {
        cues,
        first_start_frame_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"0:00:00:24, 0:00:01:00, \"a\"\n";
        let p = parse(d).unwrap();
        assert_eq!(p.cues, 1);
        assert_eq!(p.first_start_frame_ms, 960); // 24 frames × 40ms
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"0:00:02:00, 0:00:01:00, \"back\"\n").is_none()); // end<start
        assert!(parse(b"0:00:01:00, 0:00:02:00, noquote\n").is_none());
        assert!(parse(b"random text\n").is_none());
    }
}

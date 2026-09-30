//! SBV — YouTube's subtitle export: `H:MM:SS.mmm,H:MM:SS.mmm` cue
//! lines, then free-form text, blank line separated.
//!
//! ```
//! let d = b"0:00:01.000,0:00:02.500\nhello\n\n0:00:03.000,0:00:04.000\nbye\n";
//! let s = izanagi_kit::sbv::parse(d).unwrap();
//! assert_eq!(s.cues, 2);
//! assert_eq!(s.first_start_ms, 1000);
//! ```

/// Parsed SBV summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sbv {
    /// Cue blocks seen.
    pub cues: usize,
    /// First cue start, in milliseconds.
    pub first_start_ms: u64,
}

/// `H:MM:SS.mmm` → milliseconds.
fn ts(s: &str) -> Option<u64> {
    let (hms, ms) = s.split_once('.')?;
    if ms.len() != 3 || !ms.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut parts = hms.split(':');
    let h: u64 = parts.next()?.parse().ok()?;
    let m: u64 = parts.next()?.parse().ok()?;
    let sec: u64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || m > 59 || sec > 59 {
        return None;
    }
    Some(h * 3_600_000 + m * 60_000 + sec * 1000 + ms.parse::<u64>().ok()?)
}

/// Parse an SBV file; `None` without at least one valid cue line.
pub fn parse(d: &[u8]) -> Option<Sbv> {
    let s = std::str::from_utf8(d).ok()?;
    let mut cues = 0usize;
    let mut first_start_ms = 0u64;
    for l in s.lines() {
        if let Some((a, b)) = l.trim().split_once(',') {
            if let (Some(st), Some(en)) = (ts(a.trim()), ts(b.trim())) {
                if en > st {
                    if cues == 0 {
                        first_start_ms = st;
                    }
                    cues += 1;
                }
            }
        }
    }
    if cues == 0 {
        return None;
    }
    Some(Sbv {
        cues,
        first_start_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"1:23:45.678,1:23:46.000\nhi\n0:00:00.000,0:00:00.001\nx\n";
        let s = parse(d).unwrap();
        assert_eq!(s.cues, 2);
        assert_eq!(s.first_start_ms, 5025678);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"1,2\n").is_none()); // not timestamps
        assert!(parse(b"0:00:02.000,0:00:01.000\n").is_none()); // end < start
        assert!(parse(b"0:00:61.000,0:01:00.000\n").is_none()); // bad sec
    }
}

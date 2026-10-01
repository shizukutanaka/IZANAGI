//! Scenarist Closed Captions `.scc` — `Scenarist_SCC V1.0` header then
//! `HH:MM:SS:FF<tab>…hex pairs…` rows (`;` on the frame separator marks
//! drop-frame).
//!
//! ```
//! let d = b"Scenarist_SCC V1.0\n\n00:00:01:00\t9420 9420 94ae 94ae\n00:00:02:15\t942f 942f\n";
//! let s = izanagi_kit::scc::parse(d).unwrap();
//! assert_eq!(s.rows, 2);
//! ```

/// Parsed SCC file summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scc {
    /// Version text after `Scenarist_SCC` (e.g. `V1\x2e0`).
    pub version: String,
    /// Caption rows (timecode lines).
    pub rows: usize,
    /// Any row used `;` drop-frame separators.
    pub drop_frame: bool,
}

fn is_timecode(l: &str) -> Option<bool> {
    // HH:MM:SS:FF or HH:MM:SS;FF
    let mut parts = l.split([':', ';']);
    let four = (0..4).all(|_| {
        parts
            .next()
            .map(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit()))
            .unwrap_or(false)
    }) && parts.next().is_none();
    four.then(|| l.contains(';'))
}

/// Parse an SCC file; `None` without the `Scenarist_SCC` banner.
pub fn parse(d: &[u8]) -> Option<Scc> {
    let s = std::str::from_utf8(d).ok()?;
    let mut lines = s.lines();
    let banner = lines.next()?.trim();
    let version = banner.strip_prefix("Scenarist_SCC")?.trim().to_string();
    if version.is_empty() {
        return None;
    }
    let mut rows = 0usize;
    let mut drop_frame = false;
    for l in lines {
        let (tc, _hex) = match l.split_once('\t').or_else(|| l.split_once(' ')) {
            Some(p) => p,
            None => continue,
        };
        if let Some(df) = is_timecode(tc.trim()) {
            rows += 1;
            drop_frame |= df;
        }
    }
    if rows == 0 {
        return None;
    }
    Some(Scc {
        version,
        rows,
        drop_frame,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"Scenarist_SCC V1.0\n\n00:00:01:00\t9420\n00:00:02;15\t942f\n";
        let s = parse(d).unwrap();
        assert_eq!((s.rows, s.drop_frame), (2, true));
        assert_eq!(s.version, "V1.0");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"Scenarist_SCC\n").is_none()); // no version
        assert!(parse(b"Scenarist_SCC V1.0\nno rows\n").is_none());
    }
}

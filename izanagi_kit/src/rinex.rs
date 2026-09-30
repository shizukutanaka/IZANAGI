//! RINEX observation/navigation file scanner.
//!
//! RINEX files are 80-column card text: the first card carries
//! `RINEX VERSION / TYPE` in columns 1–20/21–40, header labels live
//! in columns 61–80, and the header ends at `END OF HEADER`.
//!
//! ```
//! let mut f = b"     3\x2e04           OBSERVATION DATA    M (MIXED)           RINEX VERSION / TYPE".to_vec();
//! f.push(b'\n');
//! let mut l = vec![b' '; 60];
//! l.extend_from_slice(b"END OF HEADER");
//! f.extend_from_slice(&l);
//! f.push(b'\n');
//! let r = izanagi_kit::rinex::parse(&f).unwrap();
//! assert_eq!(r.version.as_deref(), Some("3\x2e04"));
//! assert_eq!(r.file_type.as_deref(), Some("OBSERVATION DATA"));
//! ```
//!
//! Reference: RINEX 2/3/4 specifications (IGS) — the `RINEX
//! VERSION / TYPE` first-card label and the column-61 label field.

/// Parsed RINEX fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Rinex {
    /// Version string from the first card's columns 1–20.
    pub version: Option<String>,
    /// File type string from columns 21–40 (`OBSERVATION DATA`,
    /// `NAVIGATION DATA`, `METEOROLOGICAL DATA`, …).
    pub file_type: Option<String>,
    /// Satellite system letter from columns 41–60, if any.
    pub system: Option<String>,
    /// Header label count (column-61 labels before `END OF HEADER`).
    pub header_labels: usize,
    /// `true` when `END OF HEADER` was seen.
    pub terminated: bool,
}

fn field(line: &[u8], lo: usize, hi: usize) -> Option<String> {
    if line.len() < hi {
        return None;
    }
    let s = core::str::from_utf8(&line[lo..hi]).ok()?.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Parse a RINEX file; `None` unless the first card carries the
/// `RINEX VERSION / TYPE` label.
pub fn parse(d: &[u8]) -> Option<Rinex> {
    let mut lines = d.split(|&b| b == b'\n');
    let first = lines.next()?;
    if !first.windows(20).any(|w| w == b"RINEX VERSION / TYPE") {
        return None;
    }
    let version = field(first, 0, 20);
    let file_type = field(first, 20, 40);
    let system = field(first, 40, 60);
    let mut header_labels = 1usize; // the VERSION / TYPE label itself
    let mut terminated = false;
    for l in lines {
        let l = l.strip_suffix(b"\r").unwrap_or(l);
        if l.len() > 60 {
            if l[60..].windows(13).any(|w| w == b"END OF HEADER") {
                terminated = true;
                break;
            }
            if core::str::from_utf8(&l[60..])
                .ok()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                header_labels += 1;
            }
        }
    }
    Some(Rinex {
        version,
        file_type,
        system,
        header_labels,
        terminated,
    })
}

/// `true` if the buffer looks like a RINEX file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Vec<u8> {
        let mut f =
            b"     3\x2e04           OBSERVATION DATA    M (MIXED)           RINEX VERSION / TYPE"
                .to_vec();
        f.push(b'\n');
        f.extend_from_slice(b"test                                                       COMMENT");
        f.push(b'\n');
        let mut l = vec![b' '; 60];
        l.extend_from_slice(b"END OF HEADER");
        f.extend_from_slice(&l);
        f.push(b'\n');
        f
    }

    #[test]
    fn parses() {
        let r = parse(&doc()).unwrap();
        assert_eq!(r.version.as_deref(), Some("3\x2e04"));
        assert_eq!(r.file_type.as_deref(), Some("OBSERVATION DATA"));
        assert_eq!(r.system.as_deref(), Some("M (MIXED)"));
        assert!(r.header_labels >= 2);
        assert!(r.terminated);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"hello\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&doc()));
        assert!(!detect(b"RINEX"));
    }
}

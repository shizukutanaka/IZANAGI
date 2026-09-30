//! ANTEX antenna calibration file scanner.
//!
//! ANTEX files are 80-column card text (same label convention as
//! RINEX): the first card's columns 61–80 hold `ANTEX VERSION / SYST`,
//! and `END OF HEADER` terminates the header block. Antenna sections
//! run from `START OF ANTENNA` to `END OF ANTENNA` with
//! `TYPE / SERIAL NO`, `# OF FREQUENCIES`, `START OF FREQUENCY`, …
//!
//! ```
//! let mut l0 = b"     1\x2e4            G".to_vec();
//! l0.resize(60, b' ');
//! l0.extend_from_slice(b"ANTEX VERSION / SYST");
//! let mut f = l0;
//! f.push(b'\n');
//! let mut l = vec![b' '; 60];
//! l.extend_from_slice(b"END OF HEADER");
//! f.extend_from_slice(&l);
//! f.push(b'\n');
//! let mut a = vec![b' '; 60];
//! a.extend_from_slice(b"START OF ANTENNA");
//! f.extend_from_slice(&a);
//! f.push(b'\n');
//! let a = izanagi_kit::antex::parse(&f).unwrap();
//! assert_eq!(a.antennas, 1);
//! ```
//!
//! Reference: ANTEX format specification (IGS, antex format docs) —
//! the column-61 label convention shared with RINEX and the
//! `START OF`/`END OF` section labels.

/// Parsed ANTEX fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Antex {
    /// Version string from columns 1–20.
    pub version: Option<String>,
    /// System letter from columns 21–40 (`G`, `R`, `E`, `M`, …).
    pub system: Option<String>,
    /// `START OF ANTENNA` section count.
    pub antennas: usize,
    /// `START OF FREQUENCY` count.
    pub frequencies: usize,
    /// `TYPE / SERIAL NO` label count.
    pub serials: usize,
    /// `true` when `END OF HEADER` was seen.
    pub header_closed: bool,
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

fn label(line: &[u8]) -> &str {
    let Some(l) = line.get(60..) else {
        return "";
    };
    core::str::from_utf8(l).ok().map(str::trim).unwrap_or("")
}

/// Parse an ANTEX file; `None` unless the first card carries
/// `ANTEX VERSION / SYST` in the label field.
pub fn parse(d: &[u8]) -> Option<Antex> {
    let mut lines = d.split(|&b| b == b'\n');
    let first = lines.next()?;
    if label(first) != "ANTEX VERSION / SYST" {
        return None;
    }
    let version = field(first, 0, 20);
    let system = field(first, 20, 40);
    let mut antennas = 0usize;
    let mut frequencies = 0usize;
    let mut serials = 0usize;
    let mut header_closed = false;
    for l in lines {
        let l = l.strip_suffix(b"\r").unwrap_or(l);
        match label(l) {
            "END OF HEADER" => header_closed = true,
            "START OF ANTENNA" => antennas += 1,
            "START OF FREQUENCY" => frequencies += 1,
            "TYPE / SERIAL NO" => serials += 1,
            _ => {}
        }
    }
    Some(Antex {
        version,
        system,
        antennas,
        frequencies,
        serials,
        header_closed,
    })
}

/// `true` if the buffer looks like an ANTEX file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(content: &str, lab: &str, f: &mut Vec<u8>) {
        let mut line = vec![b' '; 60];
        line[..content.len()].copy_from_slice(content.as_bytes());
        line.extend_from_slice(lab.as_bytes());
        f.extend_from_slice(&line);
        f.push(b'\n');
    }

    fn doc() -> Vec<u8> {
        let mut f = Vec::new();
        card("     1\x2e4            G", "ANTEX VERSION / SYST", &mut f);
        card("", "END OF HEADER", &mut f);
        card("", "START OF ANTENNA", &mut f);
        card("TRM59800\x2e80     NONE", "TYPE / SERIAL NO", &mut f);
        card("", "START OF FREQUENCY", &mut f);
        f
    }

    #[test]
    fn parses() {
        let a = parse(&doc()).unwrap();
        assert_eq!(a.version.as_deref(), Some("1\x2e4"));
        assert_eq!(a.system.as_deref(), Some("G"));
        assert_eq!(a.antennas, 1);
        assert_eq!(a.frequencies, 1);
        assert_eq!(a.serials, 1);
        assert!(a.header_closed);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"ANTEX\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&doc()));
        let mut bad = vec![b' '; 60];
        bad.extend_from_slice(b"ANTEX VERSION / SYSX");
        assert!(!detect(&bad));
    }
}

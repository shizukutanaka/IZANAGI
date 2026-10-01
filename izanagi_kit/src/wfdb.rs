//! PhysioNet WFDB header (`*.hea`) census.
//!
//! First line `record_name/num_signals fs num_samples`, followed by one
//! signal-spec line per signal (`file fmtXgain ...`) plus `#` comments.
//!
//! ```
//! let s = b"100/2 360 650000\n100.dat 212x200 11 1024 995 16 MLII\n101.dat 212x200 11 1024 1011 16 V5\n# comment\n";
//! assert!(izanagi_kit::wfdb::detect(s));
//! let w = izanagi_kit::wfdb::Wfdb::parse(s).unwrap();
//! assert_eq!(w.num_signals, 2);
//! assert_eq!(w.fs, 360);
//! assert_eq!(w.num_samples, 650000);
//! assert_eq!(w.signal_lines, 2);
//! assert_eq!(w.comments, 1);
//! ```

/// Parsed census of a WFDB header file.
#[derive(Debug, Clone)]
pub struct Wfdb {
    /// `num_signals` from the first record line, or 0.
    pub num_signals: usize,
    /// Sampling frequency `fs` (integer part), or 0.
    pub fs: usize,
    /// `num_samples` from the record line, or 0.
    pub num_samples: usize,
    /// Signal-spec lines after the record line.
    pub signal_lines: usize,
    /// `file.dat` references.
    pub dat_files: usize,
    /// Format codes (`212`, `16`, `80`, ...).
    pub formats: usize,
    /// `x`-prefixed gain fields.
    pub gains: usize,
    /// `init_value`/`checksum` trailing integers.
    pub checksums: usize,
    /// Units (non-numeric last tokens).
    pub units: usize,
    /// `#` comment lines.
    pub comments: usize,
    /// `record` aliases in position 0 containing `/`.
    pub record_lines: usize,
    /// Segments `~` markers (multi-segment headers).
    pub segments: usize,
    /// `null`/`~` signal entries.
    pub null_signals: usize,
}

fn record_ok(l: &str) -> bool {
    let w: Vec<&str> = l.split_whitespace().collect();
    w.len() >= 2
        && w[0].contains('/')
        && w[1].bytes().all(|c| c.is_ascii_digit())
        && w[1].len() <= 6
        && w.iter()
            .skip(2)
            .all(|v| v.bytes().all(|c| c.is_ascii_digit() || c == b'.'))
}

/// Reports whether `b` looks like a WFDB header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .find(|l| !l.trim().is_empty())
        .is_some_and(record_ok)
}

impl Wfdb {
    /// Parses `b` as a WFDB header, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut w = Wfdb {
            num_signals: 0,
            fs: 0,
            num_samples: 0,
            signal_lines: 0,
            dat_files: 0,
            formats: 0,
            gains: 0,
            checksums: 0,
            units: 0,
            comments: 0,
            record_lines: 0,
            segments: 0,
            null_signals: 0,
        };
        let mut first = true;
        for l in t.lines() {
            let l = l.trim_end();
            if l.trim().is_empty() {
                continue;
            }
            if l.trim_start().starts_with('#') {
                w.comments += 1;
                continue;
            }
            if first {
                first = false;
                w.record_lines += 1;
                let toks: Vec<&str> = l.split_whitespace().collect();
                if let Some(rec) = toks.first() {
                    if let Some((_, nsig)) = rec.split_once('/') {
                        w.num_signals = nsig.trim().parse().unwrap_or(0);
                    }
                }
                if let Some(fs) = toks.get(1) {
                    w.fs = fs.split('.').next().unwrap_or("0").parse().unwrap_or(0);
                }
                if let Some(ns) = toks.get(2) {
                    w.num_samples = ns.parse().unwrap_or(0);
                }
                continue;
            }
            let toks: Vec<&str> = l.split_whitespace().collect();
            if toks.len() >= 2 {
                w.signal_lines += 1;
                let file = toks[0];
                if file.contains('.') {
                    w.dat_files += 1;
                }
                if file == "~" {
                    w.segments += 1;
                    w.null_signals += 1;
                }
                let spec = toks[1];
                let (fmt, gain) = match spec.split_once('x') {
                    Some((f, g)) => (f, Some(g)),
                    None => (spec, None),
                };
                if fmt.bytes().all(|c| c.is_ascii_digit()) && !fmt.is_empty() {
                    w.formats += 1;
                }
                if gain.is_some_and(|g| g.bytes().all(|c| c.is_ascii_digit() || c == b'.')) {
                    w.gains += 1;
                }
                if let Some(last) = toks.last() {
                    if last.bytes().all(|c| c.is_ascii_alphabetic()) {
                        w.units += 1;
                    } else if last.bytes().all(|c| c.is_ascii_digit() || c == b'-') {
                        w.checksums += 1;
                    }
                }
            }
        }
        Some(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"100/2 360 650000\n100.dat 212x200 11 1024 995 16 MLII\n101.dat 212x200 11 1024 1011 16 V5\n# comment\n";

    #[test]
    fn parses_wfdb() {
        assert!(detect(S));
        let w = Wfdb::parse(S).unwrap();
        assert_eq!(w.num_signals, 2);
        assert_eq!(w.fs, 360);
        assert_eq!(w.num_samples, 650000);
        assert_eq!(w.signal_lines, 2);
        assert_eq!(w.comments, 1);
    }

    #[test]
    fn rejects_non_wfdb() {
        assert!(!detect(b"just text"));
        assert!(Wfdb::parse(b"").is_none());
    }
}

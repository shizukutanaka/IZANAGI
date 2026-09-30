//! NoteWorthy Composer text export (`.nwctxt`) — `!NoteWorthyComposer` banner
//! plus `|Type|Field:…` object lines.
//!
//! ```
//! let d = b"!NoteWorthyComposer(2\x2e751)\n|SongInfo|Title:\"Etude\"\n|AddStaff|\n|Note|Dur:4th|Pos:0\n|Bar|\n|Rest|Dur:Half\n";
//! let s = izanagi_kit::nwc::parse(d).unwrap();
//! assert_eq!(s.version, "2\x2e751");
//! assert_eq!(s.staves, 1);
//! assert_eq!(s.notes, 1);
//! assert_eq!(s.rests, 1);
//! assert!(izanagi_kit::nwc::detect(d));
//! ```

/// A parsed `.nwctxt` document summary.
#[derive(Debug, Clone)]
pub struct Nwc {
    /// Banner version string, e.g. `2.751`.
    pub version: String,
    /// `|AddStaff|` count.
    pub staves: usize,
    /// `|Note|` count.
    pub notes: usize,
    /// `|Rest|` count.
    pub rests: usize,
    /// `|Chord|` count.
    pub chords: usize,
    /// `|Bar|` count.
    pub bars: usize,
    /// `|Clef|` count.
    pub clefs: usize,
    /// `|Key|` count.
    pub keys: usize,
    /// Other `|Type|…` lines not enumerated above.
    pub other: usize,
}

/// Detects NWCtxt: the `!NoteWorthyComposer(` banner.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.starts_with(b"!NoteWorthyComposer")
}

fn count_type(t: &str, name: &str) -> usize {
    t.lines().filter(|l| l.starts_with(name)).count()
}

/// Parses `.nwctxt`; `None` when the banner is missing.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nwc> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let first = t.lines().next()?;
    let version = first
        .trim_start_matches("!NoteWorthyComposer")
        .trim_start_matches('(')
        .trim_end_matches(')')
        .to_string();
    let mut s = Nwc {
        version,
        staves: count_type(t, "|AddStaff|"),
        notes: count_type(t, "|Note|"),
        rests: count_type(t, "|Rest|"),
        chords: count_type(t, "|Chord|"),
        bars: count_type(t, "|Bar|"),
        clefs: count_type(t, "|Clef|"),
        keys: count_type(t, "|Key|"),
        other: 0,
    };
    let counted = s.staves + s.notes + s.rests + s.chords + s.bars + s.clefs + s.keys;
    let all = t
        .lines()
        .filter(|l| l.starts_with('|') && l[1..].contains('|'))
        .count();
    s.other = all.saturating_sub(counted);
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"!NoteWorthyComposer(2\x2e751)\n|SongInfo|Title:\"X\"\n|AddStaff|\n|Clef|Type:Treble\n|Key|Signature:F#\n|Note|Dur:4th\n|Bar|\n|Chord|Dur:8th\n|Rest|Dur:Whole\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.version, "2\x2e751");
        assert_eq!(s.staves, 1);
        assert_eq!(s.notes, 1);
        assert_eq!(s.rests, 1);
        assert_eq!(s.chords, 1);
        assert_eq!(s.bars, 1);
        assert_eq!(s.clefs, 1);
        assert_eq!(s.keys, 1);
        assert_eq!(s.other, 1); // SongInfo
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"|Note|"));
        assert!(!detect(b"!Other(1)"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain").is_none());
    }
}

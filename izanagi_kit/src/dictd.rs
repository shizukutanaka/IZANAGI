//! dict.org `dictd` `.index` — the sorted text index paired with a
//! `.dict`/`.dict.dz` body: one line per headword,
//! `headword <TAB> b64offset <TAB> b64length`. Offsets use the
//! dictd base-64-ish alphabet. Optionally the headword is
//! `"quoted"`.
//!
//! ```
//! use izanagi_kit::dictd::{detect, parse};
//!
//! let d = b"apple\tQUJD\tREVG\nbanana\tR0hJ\tSktM\n";
//! assert!(detect(d));
//! let x = parse(d).unwrap();
//! assert_eq!(x.words, 2);
//! assert_eq!(x.first.as_deref(), Some("apple"));
//! ```

/// Parsed dictd `.index` census.
#[derive(Debug, Clone, PartialEq)]
pub struct Dictd {
    /// Valid `word TAB b64 TAB b64` lines.
    pub words: u32,
    /// First headword (quotes stripped).
    pub first: Option<String>,
    /// Last headword (quotes stripped).
    pub last: Option<String>,
    /// `true` when at least one headword is `"quoted"`.
    pub quoted: bool,
    /// Malformed lines (non-empty but not three tab fields).
    pub bad_lines: u32,
}

fn b64ish(tok: &str) -> bool {
    !tok.is_empty()
        && tok
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '-' | '_'))
}

fn index_line(t: &str) -> Option<(&str, bool)> {
    // trailing whitespace is not part of the rightmost b64 field
    let mut it = t.trim_end().split('\t');
    let w = it.next()?;
    if w.is_empty() {
        return None;
    }
    let off = it.next()?;
    let len = it.next()?;
    if it.next().is_some() || !b64ish(off) || !b64ish(len) {
        return None;
    }
    let quoted = w.starts_with('"') && w.ends_with('"') && w.len() >= 2;
    Some((w.trim_matches('"'), quoted))
}

/// `true` when ≥2 of the first 8 non-empty lines are valid index rows.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut ok = 0u32;
    for line in s.lines().filter(|l| !l.trim().is_empty()).take(8) {
        if index_line(line).is_some() {
            ok += 1;
        }
    }
    ok >= 2
}

/// Census; `None` without valid index rows.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dictd> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut x = Dictd {
        words: 0,
        first: None,
        last: None,
        quoted: false,
        bad_lines: 0,
    };
    for line in s.lines() {
        let t = line.trim_end();
        if t.is_empty() {
            continue;
        }
        match index_line(t) {
            Some((w, q)) => {
                x.words += 1;
                if x.first.is_none() {
                    x.first = Some(w.to_string());
                }
                x.last = Some(w.to_string());
                x.quoted |= q;
            }
            None => x.bad_lines += 1,
        }
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"apple\tQUJD\tREVG\nbanana\tR0hJ\tSktM\n\"fig leaf\"\tTW5P\tUXJz\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"apple\tQUJD\tREVG\n"));
        assert!(!detect(b"plain"));
        assert!(!detect(b"a b c\nd e f\n"));
    }

    #[test]
    fn parses() {
        let x = parse(D).unwrap();
        assert_eq!(x.words, 3);
        assert_eq!(x.first.as_deref(), Some("apple"));
        assert_eq!(x.last.as_deref(), Some("fig leaf"));
        assert!(x.quoted);
        assert_eq!(x.bad_lines, 0);
    }

    #[test]
    fn counts_bad_lines() {
        let x = parse(b"ok\tAA\tBB\nbad line\nok2\tCC\tDD\n").unwrap();
        assert_eq!(x.words, 2);
        assert_eq!(x.bad_lines, 1);
        assert!(!x.quoted);
    }

    #[test]
    fn trailing_space_is_not_field_content() {
        // editor drift: a space after the last b64 field
        assert!(detect(b"apple\tQUJD\tREVG \nbanana\tR0hJ\tSktM\t \n"));
        let x = parse(b"apple\tQUJD\tREVG \nbanana\tR0hJ\tSktM \n").unwrap();
        assert_eq!(x.words, 2);
        assert_eq!(x.bad_lines, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

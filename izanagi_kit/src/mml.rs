//! Music Macro Language (MML) — `t120 o4 l8 cdefgab` sequences with octave,
//! length, and accidental commands.
//!
//! ```
//! let d = b"t120 o4 l8 cdefgab >c< r4\n";
//! let m = izanagi_kit::mml::parse(d).unwrap();
//! assert_eq!(m.tempo, 120);
//! assert_eq!(m.octave, 4);
//! assert_eq!(m.default_len, 8);
//! assert_eq!(m.notes, 8);
//! assert!(izanagi_kit::mml::detect(d));
//! ```

/// A parsed MML text.
#[derive(Debug, Clone)]
pub struct Mml {
    /// `tNNN` tempo (last one wins; 0 when absent).
    pub tempo: u32,
    /// `oN` octave (last one wins; 0 when absent).
    pub octave: u8,
    /// `lN` default note length (last one wins; 0 when absent).
    pub default_len: u8,
    /// `cdefgab` note tokens (with or without length suffix).
    pub notes: usize,
    /// `r`/`p` rest tokens.
    pub rests: usize,
    /// Octave shifts `>`/`<`.
    pub octave_shifts: usize,
    /// Accidentals `+`/`#`/`-` applied to notes.
    pub accidentals: usize,
    /// Ties/slurs `&`/`^`.
    pub ties: usize,
    /// Volume commands `vN`.
    pub volumes: usize,
    /// Loops `[`/`]` pairs.
    pub loops: usize,
}

/// Tokenizes while skipping whitespace and `'`/`"` quoted segments.
fn tokens(t: &str) -> Vec<char> {
    t.chars()
        .filter(|c| !c.is_whitespace() && *c != '\'' && *c != '"')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn is_note(c: char) -> bool {
    matches!(c, 'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g')
}

/// Detects MML: a *pure* MML-token stream — `t`/`o`/`l`/`v`+digit
/// directives, `cdefgab` notes, `r`/`p` rests, `>`/`<` shifts, `[`/`]`
/// loops, `&`/`^` ties, `+`/`#`/`-` accidentals and digits — with no
/// foreign characters at all (letters outside the music alphabet,
/// punctuation, markup or braces reject). A bare note run without a
/// directive is not MML: `cdefg` is just a word.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let toks = tokens(t);
    let mut directives = 0;
    let mut notes = 0;
    let mut others = 0;
    let mut i = 0;
    while i < toks.len() {
        match toks[i] {
            't' | 'o' | 'l' | 'v' if toks.get(i + 1).is_some_and(|c| c.is_ascii_digit()) => {
                directives += 1;
            }
            c if is_note(c) => notes += 1,
            'r' | 'p' | '>' | '<' | '[' | ']' | '&' | '^' | '+' | '#' | '-' | '0'..='9' => {}
            _ => others += 1,
        }
        i += 1;
    }
    directives >= 1 && notes >= 3 && others == 0
}

/// Parses MML text; `None` on non-UTF-8 or no notes/directives.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mml> {
    if !detect(b) {
        return None;
    }
    let toks = tokens(std::str::from_utf8(b).ok()?);
    let mut s = Mml {
        tempo: 0,
        octave: 0,
        default_len: 0,
        notes: 0,
        rests: 0,
        octave_shifts: 0,
        accidentals: 0,
        ties: 0,
        volumes: 0,
        loops: 0,
    };
    let mut i = 0;
    while i < toks.len() {
        // directives `t/o/l/v + digits`: capture value, then skip the digits
        if matches!(toks[i], 't' | 'o' | 'l' | 'v')
            && toks.get(i + 1).is_some_and(|c| c.is_ascii_digit())
        {
            let cmd = toks[i];
            let mut n = 0u32;
            i += 1;
            while i < toks.len() && toks[i].is_ascii_digit() {
                n = n
                    .saturating_mul(10)
                    .saturating_add(u32::from(toks[i] as u8 - b'0'));
                i += 1;
            }
            match cmd {
                't' => s.tempo = n,
                'o' => s.octave = n as u8,
                'l' => s.default_len = n as u8,
                'v' => s.volumes += 1,
                _ => {}
            }
            continue;
        }
        match toks[i] {
            '>' | '<' => s.octave_shifts += 1,
            '+' | '#' | '-' => s.accidentals += 1,
            '&' | '^' => s.ties += 1,
            '[' => s.loops += 1,
            c if is_note(c) => s.notes += 1,
            'r' | 'p' => s.rests += 1,
            _ => {}
        }
        i += 1;
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"t120 o4 l8 cdefgab >c< r4 [cd]2\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.tempo, 120);
        assert_eq!(s.octave, 4);
        assert_eq!(s.default_len, 8);
        assert_eq!(s.notes, 10); // c d e f g a b c c d
        assert_eq!(s.rests, 1);
        assert_eq!(s.octave_shifts, 2);
        assert_eq!(s.loops, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"t90 cdef"));
        // a bare note run is not MML — could be any word
        assert!(!detect(b"cdefg"));
        assert!(!detect(b"12345"));
        assert!(!detect(b""));
        // foreign characters (braces, `=`, prose letters) reject
        assert!(!detect(b"t120 o4 {\"json\": true}"));
        assert!(!detect(b"v1.2.3 abcdefg"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"xyzzy xyzzy").is_none());
    }
}

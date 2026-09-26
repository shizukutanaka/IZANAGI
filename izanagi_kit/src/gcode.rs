//! Minimal reader for G-code (RS-274 / RepRap flavour).
//!
//! Lines carry letter+value words (`G1 X10 Y20 E1.5`); `;` starts a
//! comment, `(...)` is an inline comment, `*` after words begins a
//! checksum field (ignored). Values stay verbatim strings — no float
//! parsing (determinism).
//!
//! ```
//! use izanagi_kit::gcode::parse;
//!
//! let g = parse(b"; hello\nG28\nG1 X10.5 Y-3 E1.1 F1500 ; move\n").unwrap();
//! assert_eq!(g.lines.len(), 2);
//! assert_eq!(g.lines[1].get('X').unwrap(), "10.5");
//! ```

/// One logical line's words, in order (`('G', "1")`, `('X', "10.5")`, ...).
#[derive(Debug, Default)]
pub struct Line {
    /// `N` line-number word when present.
    pub n: Option<u64>,
    /// `(letter, verbatim value)` pairs.
    pub words: Vec<(char, String)>,
}

impl Line {
    /// First value for `letter` (case-sensitive — files are conventionally
    /// uppercase).
    pub fn get(&self, letter: char) -> Option<&str> {
        self.words
            .iter()
            .find(|(c, _)| *c == letter)
            .map(|(_, v)| v.as_str())
    }

    /// `G`/`M` command word value, e.g. `Some(1)` for `G1`.
    pub fn command(&self) -> Option<(&str, u64)> {
        for (c, v) in &self.words {
            if *c == 'G' || *c == 'M' {
                if let Ok(n) = v.parse() {
                    return Some((if *c == 'G' { "G" } else { "M" }, n));
                }
            }
        }
        None
    }
}

/// A parsed G-code program.
#[derive(Debug)]
pub struct Gcode {
    /// Non-empty, non-comment lines.
    pub lines: Vec<Line>,
}

fn words(src: &str) -> Option<Line> {
    let mut line = Line::default();
    let mut i = 0;
    let b = src.as_bytes();
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            return None; // bare value / stray punctuation
        }
        let letter = c as char;
        i += 1;
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && !b[i].is_ascii_alphabetic() {
            i += 1;
        }
        let v = &src[start..i];
        if v.is_empty() {
            return None; // bare letter
        }
        if letter == 'N' {
            line.n = v.parse().ok();
        }
        line.words.push((letter, v.to_string()));
    }
    Some(line)
}

/// Parse a whole program. `None` on malformed lines (bare letters, stray
/// digits); empty files yield zero lines.
pub fn parse(data: &[u8]) -> Option<Gcode> {
    let text = std::str::from_utf8(data).ok()?;
    let mut lines = Vec::new();
    for raw in text.lines() {
        // strip `;` comment
        let code = raw.split(';').next().unwrap_or("");
        // strip `( ... )` comments (may appear mid-line)
        let mut cleaned = String::with_capacity(code.len());
        let mut depth = 0usize;
        for ch in code.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                '*' => break, // checksum field — stop
                _ => {
                    if depth == 0 {
                        cleaned.push(ch);
                    }
                }
            }
        }
        let code = cleaned.trim();
        if code.is_empty() || code == "%" {
            continue;
        }
        lines.push(words(code)?);
    }
    Some(Gcode { lines })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROG: &[u8] =
        b"%\n; comment\nN10 G90\nN20 G1 X10.5 Y-3.0 E1.1 (arc) F1500\nM104 S200 ; hotend\n";

    #[test]
    fn parses() {
        let g = parse(PROG).unwrap();
        assert_eq!(g.lines.len(), 3);
        let l1 = &g.lines[0];
        assert_eq!(l1.n, Some(10));
        assert_eq!(l1.command(), Some(("G", 90)));
        let l2 = &g.lines[1];
        assert_eq!(l2.get('X'), Some("10.5"));
        assert_eq!(l2.get('Y'), Some("-3.0"));
        assert_eq!(l2.get('F'), Some("1500"));
        assert_eq!(l2.get('Z'), None);
        assert_eq!(g.lines[2].command(), Some(("M", 104)));
        assert_eq!(g.lines[2].get('S'), Some("200"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"G\n").is_none()); // bare letter
        assert!(parse(b"123\n").is_none()); // bare value
        assert!(parse(&[0xFF]).is_none()); // non-UTF8
    }

    #[test]
    fn empty_ok() {
        assert!(parse(b"").unwrap().lines.is_empty());
        assert!(parse(b"; only comments\n(paren)\n%\n")
            .unwrap()
            .lines
            .is_empty());
    }
}

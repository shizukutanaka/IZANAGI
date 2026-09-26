//! FIGlet — the `flf` font files behind ASCII-art banners. Header
//! line `flf2a$ h b len layout comments [dir [full_layout
//! [codetag_count]]]`: the char after `flf2a` is the *hardblank* used
//! inside glyphs where a real space would be smushed away. The
//! comment block follows, then 95+ glyphs of `height` rows, each row
//! terminated by one or two endmark characters (by convention `@`).
//!
//! [`parse`] collects the header and the ASCII-32..126 glyph set;
//! [`render`] concatenates glyphs row-wise and turns hardblanks into
//! spaces.
//!
//! ```
//! use izanagi_kit::figlet::{parse, render};
//!
//! let mut s = String::from("flf2a$ 2 0 8 -1 1\ncomment\n");
//! for c in 32..=126u8 {
//!     let ch = c as char;
//!     s.push_str(&format!("{ch}@\n{ch}@@\n"));
//! }
//! let f = parse(&s).unwrap();
//! assert_eq!(f.height, 2);
//! assert_eq!(render(&f, "AB").unwrap(), "AB\nAB");
//! ```

use std::string::String;
use std::vec::Vec;

/// Number of standard glyphs (space through `~`).
pub const STANDARD: usize = 95;

/// A parsed FIGlet font.
#[derive(Clone, Debug, PartialEq)]
pub struct Figlet {
    /// The hardblank character (header's 6th byte).
    pub hardblank: char,
    /// Glyph height in rows.
    pub height: usize,
    /// Baseline row count.
    pub baseline: usize,
    /// Longest line the font allows.
    pub max_len: usize,
    /// Layout mode word (negative = full-width per old spec).
    pub old_layout: i32,
    /// Comment lines skipped after the header.
    pub comment_lines: usize,
    /// Right-to-left when `1` (0 default; only v2 headers carry it).
    pub print_direction: u8,
    /// Glyphs for ASCII 32..=126, each `height` rows.
    pub glyphs: Vec<Vec<String>>,
}

/// Parse a `.flf` file. `None` on a bad signature, truncated glyph
/// table, or rows that never carry the endmark.
pub fn parse(s: &str) -> Option<Figlet> {
    let mut lines = s.lines();
    let header = lines.next()?;
    if header.len() < 6 || &header[..5] != "flf2a" {
        return None;
    }
    let hardblank = header.as_bytes()[5] as char;
    let rest = header.get(7..).unwrap_or("");
    let mut parts = rest.split_whitespace();
    let height: usize = parts.next()?.parse().ok()?;
    let baseline: usize = parts.next()?.parse().ok()?;
    let max_len: usize = parts.next()?.parse().ok()?;
    let old_layout: i32 = parts.next()?.parse().ok()?;
    let comment_lines: usize = parts.next()?.parse().ok()?;
    let print_direction = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    if height == 0 {
        return None;
    }
    for _ in 0..comment_lines {
        lines.next()?;
    }
    let mut glyphs = Vec::with_capacity(STANDARD);
    for _ in 0..STANDARD {
        let mut rows = Vec::with_capacity(height);
        for _ in 0..height {
            let line = lines.next()?;
            // rows end with one endmark char, doubled on the last row
            let mark = line.chars().last()?;
            let mut row = line;
            while row.ends_with(mark) {
                row = &row[..row.len() - mark.len_utf8()];
            }
            rows.push(row.to_string());
        }
        glyphs.push(rows);
    }
    Some(Figlet {
        hardblank,
        height,
        baseline,
        max_len,
        old_layout,
        comment_lines,
        print_direction,
        glyphs,
    })
}

impl Figlet {
    /// Glyph rows for `c` (ASCII printable only).
    pub fn glyph(&self, c: char) -> Option<&[String]> {
        let i = (c as usize).checked_sub(32)?;
        self.glyphs.get(i).map(Vec::as_slice)
    }
}

/// Concatenate glyphs horizontally. `None` if any character lacks a
/// glyph; hardblanks become spaces in the output.
pub fn render(f: &Figlet, text: &str) -> Option<String> {
    let mut rows = vec![String::new(); f.height];
    for c in text.chars() {
        let g = f.glyph(c)?;
        for (r, row) in rows.iter_mut().enumerate() {
            row.push_str(g.get(r).map(String::as_str).unwrap_or(""));
        }
    }
    let mut out = String::new();
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&row.replace(f.hardblank, " "));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> String {
        let mut s = String::from("flf2a$ 2 0 8 -1 1\ncomment\n");
        for c in 32..=126u8 {
            let ch = c as char;
            s.push_str(&format!("{ch}@\n{ch}@@\n"));
        }
        s
    }

    #[test]
    fn header_and_glyphs() {
        let f = parse(&font()).unwrap();
        assert_eq!(
            (f.hardblank, f.height, f.baseline, f.max_len, f.old_layout),
            ('$', 2, 0, 8, -1)
        );
        assert_eq!(f.comment_lines, 1);
        assert_eq!(f.print_direction, 0);
        let g = f.glyph('A').unwrap();
        assert_eq!(g, &["A".to_string(), "A".to_string()]);
        assert!(f.glyph('\u{7F}').is_none());
        assert!(f.glyph('あ').is_none());
    }

    #[test]
    fn render_text() {
        let f = parse(&font()).unwrap();
        assert_eq!(render(&f, "Hi").unwrap(), "Hi\nHi");
        assert!(render(&f, "あ").is_none());
    }

    #[test]
    fn hardblank_becomes_space() {
        let mut s = String::from("flf2a$ 1 0 8 -1 0\n");
        for c in 32..=126u8 {
            let ch = c as char;
            if ch == 'X' {
                s.push_str("a$b@\n");
            } else {
                s.push_str(&format!("{ch}@\n"));
            }
        }
        let f = parse(&s).unwrap();
        assert_eq!(render(&f, "X").unwrap(), "a b");
    }

    #[test]
    fn rejects() {
        assert!(parse("").is_none());
        assert!(parse("flf2a$ 2 0 8 -1 1\n").is_none()); // no comment/glyphs
        assert!(parse("nope 1 1 1 1 0\n").is_none());
        // truncated glyph table
        let mut s = String::from("flf2a$ 1 0 8 -1 0\n");
        s.push_str("a@\n");
        assert!(parse(&s).is_none());
    }
}

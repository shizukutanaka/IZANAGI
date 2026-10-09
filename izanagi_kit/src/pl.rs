//! PL (property list — human-readable TFM, tftopl/pltotf):
//! parenthesized `(NAME arg …)` forms — `FAMILY`,
//! `CODINGSCHEME`, `DESIGNSIZE R`, `CHECKSUM O|H|D`,
//! `SEVENBITSAFEFLAG`, `FONTDIMEN` (`SLANT`/`SPACE`/`XHEIGHT`…),
//! `LIGTABLE` (`LABEL`/`LIG`/`KRN`/`STOP`/`SKIP`),
//! `CHARACTER` (`CHARWD`/`CHARHT`/`CHARDP`/`CHARIC`/`COMMENT`).
//! Numbers carry a radix tag: `C` char, `D` decimal, `O`
//! octal, `H` hex, `R` real, `F` face.
//!
//! ```
//! let d = b"(FAMILY cmr)\n(CODINGSCHEME TEX TEXT)\n(DESIGNSIZE R 10)\n\
//! (CHARACTER C a (CHARWD R 5))\n";
//! let p = izanagi_kit::pl::parse(d).unwrap();
//! assert_eq!(p.forms, 5);
//! assert_eq!(p.characters, 1);
//! assert_eq!(p.design_size_raw, Some(10));
//! assert!(izanagi_kit::pl::detect(d));
//! ```

use crate::textutil::strip_bom;
/// Census of a PL file.
#[derive(Debug, Clone, PartialEq)]
pub struct Pl {
    /// Top-level/inner `(NAME …)` forms opened.
    pub forms: u32,
    /// `(CHARACTER …)` forms.
    pub characters: u32,
    /// `LIG`/`KRN`/`KRNH` instructions inside `LIGTABLE`.
    pub lig_kerns: u32,
    /// `LABEL`/`STOP`/`SKIP` lig-kern directives.
    pub lig_labels: u32,
    /// `FONTDIMEN` parameter names (SLANT, SPACE, …).
    pub font_params: u32,
    /// `COMMENT` forms.
    pub comments: u32,
    /// `DESIGNSIZE R <n>` integer part.
    pub design_size_raw: Option<u32>,
    /// `CHECKSUM` value seen.
    pub has_checksum: bool,
    /// Number-tag census: `R` real, `D` decimal, `O` octal,
    /// `H` hex, `C` char, `F` face.
    pub r_nums: u32,
    /// `D` decimal numbers.
    pub d_nums: u32,
    /// `O` octal numbers.
    pub o_nums: u32,
    /// `H` hex numbers.
    pub h_nums: u32,
    /// `C` character refs.
    pub c_refs: u32,
    /// `F` face codes.
    pub f_codes: u32,
}

const FORM_NAMES: &[&str] = &[
    "FAMILY",
    "CODINGSCHEME",
    "DESIGNSIZE",
    "CHECKSUM",
    "SEVENBITSAFEFLAG",
    "HEADER",
    "FONTDIMEN",
    "LIGTABLE",
    "CHARACTER",
    "COMMENT",
    "BOUNDARYCHAR",
    "VTITLE",
    "TYPEFACE",
    "LABEL",
    "LIG",
    "KRN",
    "STOP",
    "SKIP",
    "CHARWD",
    "CHARHT",
    "CHARDP",
    "CHARIC",
    "NEXTLARGER",
    "SMALLER",
    "VARCHAR",
    "MAP",
    "SELECTFONT",
    "SETCHAR",
    "MOVERIGHT",
    "MOVELEFT",
    "MOVEDOWN",
    "MOVEUP",
    "PUSH",
    "POP",
    "SPECIAL",
    "PARAMETER",
];

fn forms_of(s: &str) -> impl Iterator<Item = &str> {
    s.split('(').skip(1).map(|rest| {
        rest.trim_start()
            .split(|c: char| c.is_ascii_whitespace() || c == ')')
            .next()
            .unwrap_or("")
    })
}

/// `true` when a recognizable `(NAME` form leads the file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let s = strip_bom(s);
    let t = s.trim_start();
    t.starts_with('(')
        && forms_of(t)
            .next()
            .is_some_and(|w| FORM_NAMES.contains(&w.to_ascii_uppercase().as_str()))
}

fn uint_arg(stmt: &str, from: usize) -> Option<u32> {
    // (DESIGNSIZE R 10…) — digits of the argument after tag.
    let rest = stmt.get(from..)?;
    let mut it = rest.split_whitespace();
    let _tag = it.next()?;
    let val = it.next()?;
    val.split(['.', ')']).next().and_then(|v| v.parse().ok())
}

/// Census; `None` without a leading known form.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Pl> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let s = strip_bom(s);
    let mut p = Pl {
        forms: 0,
        characters: 0,
        lig_kerns: 0,
        lig_labels: 0,
        font_params: 0,
        comments: 0,
        design_size_raw: None,
        has_checksum: false,
        r_nums: 0,
        d_nums: 0,
        o_nums: 0,
        h_nums: 0,
        c_refs: 0,
        f_codes: 0,
    };
    let mut depth = 0u32;
    let mut in_ligtable = false;
    let mut in_fontdimen = false;
    // Token scan: '(' opens a form, ')' closes; bare words carry radix tags.
    let mut i = 0;
    let bs = s.as_bytes();
    while i < bs.len() {
        match bs[i] {
            b'(' => {
                let start = i + 1;
                let mut j = start;
                while j < bs.len() && !bs[j].is_ascii_whitespace() && bs[j] != b')' {
                    j += 1;
                }
                let name = s[start..j].to_ascii_uppercase();
                p.forms += 1;
                depth += 1;
                match name.as_str() {
                    "LIGTABLE" => in_ligtable = true,
                    "FONTDIMEN" => in_fontdimen = true,
                    "CHARACTER" => p.characters += 1,
                    "COMMENT" => p.comments += 1,
                    "LIG" | "KRN" => {
                        if in_ligtable {
                            p.lig_kerns += 1;
                        }
                    }
                    "LABEL" | "STOP" | "SKIP" | "KRNH" => {
                        if in_ligtable {
                            p.lig_labels += 1;
                        }
                    }
                    "DESIGNSIZE" => {
                        p.design_size_raw = uint_arg(&s[start..], name.len());
                    }
                    "CHECKSUM" => p.has_checksum = true,
                    _ => {
                        if in_fontdimen && !in_ligtable {
                            p.font_params += 1;
                        }
                    }
                }
                i = j;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    in_ligtable = false;
                    in_fontdimen = false;
                }
                i += 1;
            }
            _ => {
                // bare word — radix tag?
                if bs[i].is_ascii_alphabetic()
                    && (i + 1 >= bs.len() || bs[i + 1].is_ascii_whitespace())
                    && depth > 0
                {
                    let tag = bs[i] as char;
                    // next non-space must be a value char, not ')'
                    let mut j = i + 1;
                    while j < bs.len() && bs[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    let has_val = j < bs.len() && bs[j] != b')' && bs[j] != b'(';
                    if has_val {
                        match tag.to_ascii_uppercase() {
                            'R' => p.r_nums += 1,
                            'D' => p.d_nums += 1,
                            'O' => p.o_nums += 1,
                            'H' => p.h_nums += 1,
                            'C' => p.c_refs += 1,
                            'F' => p.f_codes += 1,
                            _ => {}
                        }
                    }
                }
                i += 1;
            }
        }
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"(COMMENT this is a property list)\n\
(FAMILY cmr)\n\
(CODINGSCHEME TEX TEXT)\n\
(DESIGNSIZE R 10)\n\
(CHECKSUM O 35176672345)\n\
(FONTDIMEN\n\
   (SLANT R 0)\n\
   (SPACE R 3)\n\
   (XHEIGHT R 4)\n\
   )\n\
(LIGTABLE\n\
   (LABEL C f)\n\
   (LIG C i C f)\n\
   (KRN C e R 1)\n\
   (STOP)\n\
   )\n\
(CHARACTER C f\n\
   (CHARWD R 5)\n\
   (CHARHT R 6)\n\
   (COMMENT tall)\n\
   )\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"(FONTDIMEN (SLANT R 0))"));
        assert!(!detect(b"(UNKNOWN x)"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn parses() {
        let p = parse(D).unwrap();
        assert_eq!(p.characters, 1);
        assert_eq!(p.comments, 2);
        assert_eq!(p.lig_kerns, 2);
        assert_eq!(p.lig_labels, 2); // LABEL + STOP
        assert_eq!(p.font_params, 3);
        assert_eq!(p.design_size_raw, Some(10));
        assert!(p.has_checksum);
        assert!(p.c_refs >= 4);
        assert!(p.r_nums >= 5);
        assert_eq!(p.o_nums, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"cmr10").is_none());
        assert!(parse(b"").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

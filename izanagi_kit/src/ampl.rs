//! AMPL `.mod` model files: `set`, `param`, `var`, `maximize`/
//! `minimize <name>:` objectives, `subject to <name>:`/`s.t.`
//! constraints, `#` comments, `:=` assignments, `option`/
//! `solve`/`display` statements.
//!
//! ```
//! use izanagi_kit::ampl::{detect, parse};
//!
//! let d = b"# model\nset I;\nparam c{I};\nvar x{I} >= 0;\n\
//! maximize profit: sum{i in I} c[i]*x[i];\n\
//! subject to cap: sum{i in I} x[i] <= 10;\nsolve;\n";
//! assert!(detect(d));
//! let a = parse(d).unwrap();
//! assert_eq!(a.sets, 1);
//! assert_eq!(a.constraints, 1);
//! ```

/// Parsed AMPL model census.
#[derive(Debug, Clone, PartialEq)]
pub struct Ampl {
    /// `set` declarations.
    pub sets: u32,
    /// `param` declarations.
    pub params: u32,
    /// `var` declarations.
    pub vars: u32,
    /// `maximize`/`minimize` objectives.
    pub objectives: u32,
    /// `subject to`/`s.t.`/`subj to` constraint declarations.
    pub constraints: u32,
    /// `sum{…}` aggregate uses.
    pub sums: u32,
    /// `forall`/`exists` quantifiers.
    pub quantifiers: u32,
    /// `:=` assignments.
    pub assignments: u32,
    /// `option` statements.
    pub options: u32,
    /// `solve` statements.
    pub solves: u32,
    /// `display`/`printf` statements.
    pub displays: u32,
    /// `#` comment lines.
    pub comments: u32,
}

fn kw_count(s: &str, kw: &str) -> u32 {
    let mut n = 0u32;
    for (i, _) in s.match_indices(kw) {
        let before_ok = i == 0
            || s[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace() || c == ';' || c == '\n');
        let after_ok = s[i + kw.len()..].chars().next().map_or(true, |c| {
            c.is_whitespace() || c == '{' || c == ':' || c == '(' || c == ';'
        });
        if before_ok && after_ok {
            n += 1;
        }
    }
    n
}

/// `true` on `var`/`param`/`maximize`/`subject to` AMPL keywords.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    kw_count(s, "var") + kw_count(s, "param") >= 1
        && (kw_count(s, "maximize") + kw_count(s, "minimize") >= 1
            || s.contains("subject to")
            || s.contains("s.t."))
}

/// Census; `None` without AMPL keywords.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ampl> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    Some(Ampl {
        sets: kw_count(s, "set"),
        params: kw_count(s, "param"),
        vars: kw_count(s, "var"),
        objectives: kw_count(s, "maximize") + kw_count(s, "minimize"),
        constraints: u32::try_from(
            s.matches("subject to").count()
                + s.matches("s.t.").count()
                + s.matches("subj to").count(),
        )
        .unwrap_or(u32::MAX),
        sums: kw_count(s, "sum"),
        quantifiers: kw_count(s, "forall") + kw_count(s, "exists"),
        assignments: u32::try_from(s.matches(":=").count()).unwrap_or(u32::MAX),
        options: kw_count(s, "option"),
        solves: kw_count(s, "solve"),
        displays: kw_count(s, "display") + kw_count(s, "printf"),
        comments: u32::try_from(
            s.lines()
                .filter(|l| l.trim_start().starts_with('#'))
                .count(),
        )
        .unwrap_or(u32::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"# model\nset I;\nparam c{I};\nvar x{I} >= 0;\nvar y;\n\
maximize profit: sum{i in I} c[i]*x[i];\n\
subject to cap: sum{i in I} x[i] <= 10;\n\
s.t. bound: y <= 5;\noption solver cbc;\nsolve;\ndisplay x;\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"set only;\n"));
    }

    #[test]
    fn parses() {
        let a = parse(D).unwrap();
        assert_eq!(a.sets, 1);
        assert_eq!(a.params, 1);
        assert_eq!(a.vars, 2);
        assert_eq!(a.objectives, 1);
        assert_eq!(a.constraints, 2);
        assert_eq!(a.sums, 2);
        assert_eq!(a.options, 1);
        assert_eq!(a.solves, 1);
        assert_eq!(a.displays, 1);
        assert_eq!(a.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

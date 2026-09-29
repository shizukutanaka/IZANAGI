//! LP file — the CPLEX-style text format: `\` comments,
//! `maximize`/`minimize` objective, `subject to`/`such that`/`st`
//! constraints with `<=`/`>=`/`=` senses, `bounds`, `general`/
//! `generals`/`gen`, `binaries`/`binary`, `end`.
//!
//! ```
//! use izanagi_kit::lp::{detect, parse};
//!
//! let d = b"\\ comment\nMaximize\n obj: x + 2 y\nSubject To\n\
//!  c1: x + y <= 10\nBounds\n 0 <= x <= 4\nBinaries\n y\nEnd\n";
//! assert!(detect(d));
//! let l = parse(d).unwrap();
//! assert_eq!(l.direction.as_deref(), Some("maximize"));
//! assert_eq!(l.constraints, 1);
//! ```

/// Parsed LP-file census.
#[derive(Debug, Clone, PartialEq)]
pub struct Lp {
    /// `"maximize"`/`"minimize"`/`"max"`/`"min"` seen.
    pub direction: Option<String>,
    /// Constraint lines under `subject to` (`<`/`>`/`=` sense ops).
    pub constraints: u32,
    /// `<=` senses.
    pub le: u32,
    /// `>=` senses.
    pub ge: u32,
    /// `=` senses.
    pub eq: u32,
    /// `bounds` section lines.
    pub bound_lines: u32,
    /// `general`/`gen` variable lines.
    pub general_lines: u32,
    /// `binaries`/`binary` variable lines.
    pub binary_lines: u32,
    /// `free` bound declarations.
    pub free_bounds: u32,
    /// `end` card seen.
    pub has_end: bool,
    /// `\` comment lines.
    pub comments: u32,
}

/// `true` on `max`/`min` + `subject to`-style skeleton.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let low = s.to_ascii_lowercase();
    (low.contains("maximize") || low.contains("minimize") || low.contains("\nmax "))
        && (low.contains("subject to")
            || low.contains("such that")
            || low.contains("\nst\n")
            || low.contains("\nst "))
}

/// Census; `None` without an LP skeleton.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Lp> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let low = s.to_ascii_lowercase();
    let mut l = Lp {
        direction: None,
        constraints: 0,
        le: 0,
        ge: 0,
        eq: 0,
        bound_lines: 0,
        general_lines: 0,
        binary_lines: 0,
        free_bounds: 0,
        has_end: false,
        comments: 0,
    };
    for (dir, key) in [
        ("maximize", "maximize"),
        ("minimize", "minimize"),
        ("max", "\nmax"),
        ("min", "\nmin"),
    ] {
        if low.contains(key) {
            l.direction = Some(dir.to_string());
            break;
        }
    }
    let mut section = "";
    for line in low.lines() {
        let t = line.trim();
        if t.starts_with('\\') {
            l.comments += 1;
            continue;
        }
        let head = t.split_whitespace().next().unwrap_or("");
        match head {
            "maximize" | "minimize" | "max" | "min" => section = "obj",
            "subject" | "such" | "st" => section = "st",
            "bounds" => section = "bounds",
            "general" | "generals" | "gen" => {
                section = "gen";
                l.general_lines += 1;
            }
            "binaries" | "binary" => {
                section = "bin";
                l.binary_lines += 1;
            }
            "end" => {
                l.has_end = true;
                section = "";
            }
            _ => match section {
                "st" if t.contains('<') || t.contains('>') || t.contains('=') => {
                    l.constraints += 1;
                    if t.contains("<=") {
                        l.le += 1;
                    } else if t.contains(">=") {
                        l.ge += 1;
                    } else if t.contains('=') {
                        l.eq += 1;
                    }
                }
                "bounds" if !t.is_empty() => {
                    l.bound_lines += 1;
                    if t.contains("free") {
                        l.free_bounds += 1;
                    }
                }
                "gen" if !t.is_empty() => l.general_lines += 1,
                "bin" if !t.is_empty() => l.binary_lines += 1,
                _ => {}
            },
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"\\ comment\nMaximize\n obj: x + 2 y\nSubject To\n\
 c1: x + y <= 10\n c2: x >= 1\nBounds\n 0 <= x <= 4\n x free\nGenerals\n x\nBinaries\n y\nEnd\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"maximize nothing"));
    }

    #[test]
    fn parses() {
        let l = parse(D).unwrap();
        assert_eq!(l.direction.as_deref(), Some("maximize"));
        assert_eq!(l.constraints, 2);
        assert_eq!(l.le, 1);
        assert_eq!(l.ge, 1);
        assert_eq!(l.bound_lines, 2);
        assert_eq!(l.free_bounds, 1);
        assert_eq!(l.general_lines, 2);
        assert_eq!(l.binary_lines, 2);
        assert!(l.has_end);
        assert_eq!(l.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}

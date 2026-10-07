//! CPPLINT `CPPLINT.cfg` census.
//!
//! `CPPLINT.cfg` is a per-directory `key=value` file: `filter=` (+/-
//! category globs, repeatable), `linelength=`, `root=`, `headers=`,
//! `exclude_files=`, `extensions=`, `includeorder=`, `counting=`,
//! `quiet`, `recursive`, `verbose=#`, `output=`, `timing`/`toplevel`.
//!
//! ```rust
//! let c = izanagi_kit::cpplint::Cpplint::parse(b"linelength=100\nfilter=-build/include\n").unwrap();
//! assert_eq!(c.settings, 2);
//! ```

/// `CPPLINT.cfg` census.
#[derive(Debug, Clone)]
pub struct Cpplint {
    /// `key=value` entries.
    pub settings: usize,
    /// Individual `+`/`-` tokens across all `filter=` values.
    pub filters: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "filter",
    "linelength",
    "root",
    "headers",
    "exclude_files",
    "extensions",
    "includeorder",
    "counting",
    "quiet",
    "recursive",
    "verbose",
    "output",
    "timing",
    "toplevel",
    "output_format",
    "quiet_files",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a `CPPLINT.cfg` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .filter(|l| {
            let s = l.trim();
            s.split('=')
                .next()
                .map(|k| KEYS.contains(&k.trim()))
                .unwrap_or(false)
        })
        .count()
        >= 2
}

impl Cpplint {
    /// Parse a `CPPLINT.cfg` file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            settings: 0,
            filters: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some((k, v)) = s.split_once('=') {
                c.settings += 1;
                if k.trim() == "filter" {
                    for tok in v.split(',') {
                        let tok = tok.trim();
                        if tok.starts_with('+') || tok.starts_with('-') {
                            c.filters += 1;
                        }
                    }
                }
            } else if KEYS.contains(&s) {
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpplint_cfg() {
        let b = concat!(
            "# cpplint\n",
            "linelength=100\n",
            "filter=-build/include,-build/header_guard,+build/forward_decl\n",
            "root=src\n",
            "headers=h,hpp\n",
            "exclude_files=test\n",
            "counting=total\n",
        );
        let c = Cpplint::parse(b.as_bytes()).unwrap();
        assert_eq!(c.settings, 6);
        assert_eq!(c.filters, 3);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Cpplint::parse(b"foo=1\nbar=2").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

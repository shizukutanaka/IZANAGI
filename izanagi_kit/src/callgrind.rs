//! Valgrind Callgrind output (`callgrind.out.*`) parser.
//!
//! Detects `events:`/`fl=`/`fn=`/`cfl=`/`cfn=`/`calls=`/`summary:` key
//! lines and counts declared event columns, file specs, function specs,
//! call arcs, and `<position> <cost…>` cost rows.
//!
//! ```
//! let b = br#"events: Ir Dr Dw
//! fl=(2) app.c
//! fn=(5) main
//! 20 4 1 1
//! 30 2 0 1
//! cfl=(3) lib.c
//! cfn=(7) helper
//! calls=2 30
//! 30 6 0 2
//! fl=(3) lib.c
//! fn=(7) helper
//! 30 6 0 2
//! "#;
//! assert!(izanagi_kit::callgrind::detect(b));
//! let c = izanagi_kit::callgrind::Callgrind::parse(b).unwrap();
//! assert_eq!(c.events, 3);
//! assert_eq!(c.functions, 2);
//! assert_eq!(c.calls, 1);
//! ```

/// Parsed callgrind output summary.
#[derive(Debug, Clone)]
pub struct Callgrind {
    /// Event columns from `events:`/`event:` lines.
    pub events: usize,
    /// `fl=`/`fl=(` file specs seen.
    pub files: usize,
    /// `fn=`/`fn=(` function specs seen.
    pub functions: usize,
    /// `calls=N` call arcs seen.
    pub calls: usize,
    /// `<pos> <cost…>` numeric cost rows.
    pub cost_rows: usize,
    /// `summary:` present.
    pub summary: bool,
}

fn is_cost_row(tr: &str) -> bool {
    // `20 4 1 1` / `+2 3` — one position then 1+ integer costs.
    let mut it = tr.split_whitespace();
    let Some(first) = it.next() else { return false };
    let mut toks = 0usize;
    for tok in it {
        let t = tok.trim_start_matches(['+', '-', '*']);
        if t.is_empty() || t.chars().any(|c| !c.is_ascii_digit()) {
            return false;
        }
        toks += 1;
    }
    let p = first.trim_start_matches(['+', '-', '*']);
    !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && toks >= 1
}

/// Whether the buffer looks like callgrind output.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut ev = false;
    let mut fns = 0usize;
    for l in t.lines().take(4096) {
        let tr = l.trim();
        if tr.starts_with("events:") || tr.starts_with("event:") {
            ev = true;
        }
        if tr.starts_with("fn=") || tr.starts_with("fn=(") {
            fns += 1;
        }
    }
    ev && fns >= 2
}

impl Callgrind {
    /// Parses a callgrind output summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            events: 0,
            files: 0,
            functions: 0,
            calls: 0,
            cost_rows: 0,
            summary: false,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() || tr.starts_with('#') {
                continue;
            }
            if let Some(rest) = tr
                .strip_prefix("events:")
                .or_else(|| tr.strip_prefix("event:"))
            {
                c.events += rest.split_whitespace().count();
            } else if tr.starts_with("fl=") || tr.starts_with("fl=(") {
                c.files += 1;
            } else if tr.starts_with("fn=") || tr.starts_with("fn=(") {
                c.functions += 1;
            } else if tr.starts_with("calls=") {
                c.calls += 1;
            } else if tr.starts_with("summary:") {
                c.summary = true;
            } else if is_cost_row(tr) {
                c.cost_rows += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"# callgrind format
version: 1
creator: callgrind
positions: line
events: Ir Dr Dw
fl=(2) app.c
fn=(5) main
20 4 1 1
30 2 0 1
cfl=(3) lib.c
cfn=(7) helper
calls=2 30
30 6 0 2
fl=(3) lib.c
fn=(7) helper
30 6 0 2
totals: 18 1 4
summary: 18 1 4
"#;
        assert!(detect(b));
        let c = Callgrind::parse(b).unwrap();
        assert_eq!(c.events, 3);
        assert_eq!(c.files, 2);
        assert_eq!(c.functions, 2);
        assert_eq!(c.calls, 1);
        assert_eq!(c.cost_rows, 4);
        assert!(c.summary);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"1 2\n3 4\n"));
        assert!(!detect(b"fl=a\nfn=b\n"));
        assert!(Callgrind::parse(b"events: Ir\nfn=x\n").is_none());
    }
}

//! gcov text coverage (`.gcov`) — `count:line:code` rows.
//!
//! Count field: a number (executions), `-` (non-executable), `#####`
//! (never executed), `=====` (unexecuted basic block), `$$$$$` (never
//! executed branch). `-:0:tag:` pseudo-lines carry metadata like
//! `Source:`/`Graph:`.
//!
//! ```
//! use izanagi_kit::gcov::parse;
//!
//! let g = parse(b"        -:    0:Source:a.c\n        5:    1:int main(){\n    #####:    2:return 1;\n").unwrap();
//! assert_eq!(g.source.as_deref(), Some("a.c"));
//! assert_eq!(g.lines[0].exec, Some(5));
//! assert!(g.lines[1].uncovered);
//! ```

/// One `count:line:code` row.
#[derive(Clone, Debug)]
pub struct GcovLine {
    /// Source line number.
    pub no: u32,
    /// Execution count (`None` for `-`/`#####`/`$$$$$`/`=====`).
    pub exec: Option<u64>,
    /// `true` when the count field is `#####`/`$$$$$`/`=====`
    /// (executable but never run).
    pub uncovered: bool,
    /// Code text after the second colon.
    pub text: String,
}

/// A parsed `.gcov` file.
#[derive(Clone, Debug)]
pub struct Gcov {
    /// `Source:` value from a `-:0:` metadata line.
    pub source: Option<String>,
    /// Metadata lines (`tag`, `value`) — `Source`, `Graph`, `Data`…
    pub meta: Vec<(String, String)>,
    /// Data rows (line number > 0).
    pub lines: Vec<GcovLine>,
}

/// Parse `.gcov` text. `None` when no `count:line:` row exists or one
/// is malformed.
pub fn parse(d: &[u8]) -> Option<Gcov> {
    let text = std::str::from_utf8(d).ok()?;
    let mut source = None;
    let mut meta = Vec::new();
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (cnt, rest) = line.split_once(':')?;
        let cnt = cnt.trim();
        let (no_s, code) = rest.split_once(':')?;
        let no: u32 = no_s.trim().parse().ok()?;
        let code = code.to_string();
        if no == 0 {
            // metadata line: `code` is `Tag:value`
            let (k, v) = code.split_once(':').unwrap_or((code.trim(), ""));
            meta.push((k.trim().to_string(), v.trim().to_string()));
            if k.trim() == "Source" {
                source = Some(v.trim().to_string());
            }
            continue;
        }
        let (exec, uncovered) = match cnt {
            "-" => (None, false),
            "#####" | "$$$$$" | "=====" => (None, true),
            n => (Some(n.parse().ok()?), false),
        };
        lines.push(GcovLine {
            no,
            exec,
            uncovered,
            text: code,
        });
    }
    if lines.is_empty() && meta.is_empty() {
        return None;
    }
    Some(Gcov {
        source,
        meta,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let g = parse(b"        -:    0:Source:a.c\n        -:    0:Graph:a.gcno\n        3:    1:fn f()\n    #####:    2:{\n").unwrap();
        assert_eq!(g.source.as_deref(), Some("a.c"));
        assert_eq!(g.meta.len(), 2);
        assert_eq!(g.lines[0].exec, Some(3));
        assert!(g.lines[1].uncovered);
        assert_eq!(g.lines[1].no, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"not a row\n").is_none());
        assert!(parse(b"   x:   1:code\n").is_none());
    }
}

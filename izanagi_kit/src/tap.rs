//! TAP — the Test Anything Protocol (TAP13/TAP14), the text protocol
//! Perl's test harness pioneered and node's `node-tap` still emits.
//! Grammar per testanything.org: an optional `TAP version 14` line, a
//! plan `1..N` (possibly with a `# SKIP` reason) appearing once either
//! before or after the points, `ok`/`not ok` test points with
//! optional numbers, descriptions and `# TODO`/`# SKIP` directives,
//! `#` comments, `Bail out!` lines, 4-space-indented subtests, and
//! 2-space-indented `---`/`...` YAML diagnostic blocks attached to
//! the preceding point.
//!
//! Parsing is tolerant by spec — unknown lines are kept as
//! [`Line::Unknown`] so a stream is never refused. [`Tap::ok`] applies
//! the harness rules: no bailout, no non-TODO failure, and the plan
//! (if present) matches the top-level point count.
//!
//! ```
//! use izanagi_kit::tap::parse;
//!
//! let t = parse("TAP version 14\n1..3\nok 1 - boots\nnot ok 2 - later # TODO\nok 3\n");
//! assert_eq!(t.version(), Some(14));
//! let s = t.summary();
//! assert_eq!((s.points, s.passed, s.failed, s.todo), (3, 2, 1, 1));
//! assert!(t.ok()); // a TODO failure doesn't fail the stream
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// A `# TODO`/`# SKIP` directive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Directive {
    /// `# TODO reason` — failure expected.
    Todo(String),
    /// `# SKIP reason` — point not applicable.
    Skip(String),
}

/// One parsed line of a TAP stream.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// `TAP version N`.
    Version(u32),
    /// `lo..hi` plan with optional `# comment` (a `SKIP` directive
    /// means the whole file is skipped).
    Plan {
        /// First test number (always 1 in TAP14).
        lo: u32,
        /// Planned point count's upper bound (`0` → `1..0` skip-all).
        hi: u32,
        /// `# SKIP reason` or free `# comment`.
        comment: Option<String>,
    },
    /// `ok` / `not ok` point.
    Point {
        /// Explicit point number when written on the wire.
        num: Option<u32>,
        /// `ok` vs `not ok`.
        ok: bool,
        /// Description text (`- ` prefix stripped).
        desc: String,
        /// `# TODO`/`# SKIP` suffix.
        directive: Option<Directive>,
        /// Leading-space indent (4 = TAP14 subtest, any other for
        /// forward compatibility).
        indent: usize,
    },
    /// `# ...` comment (or a TAP12 pragma line).
    Comment(String),
    /// A line inside a `  ---` … `  ...` YAML diagnostic block
    /// (content includes the indent).
    Yaml(String),
    /// `Bail out!` with its reason.
    BailOut(String),
    /// Any line the grammar doesn't cover — kept verbatim.
    Unknown(String),
}

/// A parsed TAP stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Tap {
    /// Every line in order.
    pub lines: Vec<Line>,
}

/// Counts over the stream's points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Summary {
    /// `ok`/`not ok` points seen (including subtest-indented ones).
    pub points: usize,
    /// `ok` points.
    pub passed: usize,
    /// `not ok` points (TODO or not).
    pub failed: usize,
    /// Failed points carrying `# TODO`.
    pub todo: usize,
    /// Points carrying `# SKIP`.
    pub skipped: usize,
    /// Points at indent ≥ 4 (subtest results).
    pub subtests: usize,
}

fn split_comment(s: &str) -> (&str, Option<String>) {
    match s.split_once('#') {
        Some((body, c)) => (body.trim_end(), Some(c.trim().to_string())),
        None => (s.trim_end(), None),
    }
}

fn directive_of(c: &str) -> Option<Directive> {
    let up = c.to_ascii_uppercase();
    for (word, mk) in [
        ("TODO", Directive::Todo as fn(String) -> Directive),
        ("SKIP", Directive::Skip as fn(String) -> Directive),
    ] {
        if let Some(r) = up.strip_prefix(word) {
            if r.is_empty() || r.starts_with(' ') || r.starts_with(':') {
                return Some(mk(c[word.len()..].trim().to_string()));
            }
        }
    }
    None
}

/// Parse a plan body `1..N` — returns `(hi, comment)`; `lo` is always
/// 1 per TAP14 but accepted as written.
fn plan(body: &str) -> Option<(u32, Option<String>)> {
    let (range, comment) = split_comment(body);
    let (lo, hi) = range.split_once("..")?;
    let lo: u32 = lo.trim().parse().ok()?;
    let hi: u32 = hi.trim().parse().ok()?;
    if lo != 1 {
        return None;
    }
    Some((hi, comment))
}

fn point(ok_word: &str, rest: &str, indent: usize) -> Line {
    let ok = ok_word == "ok";
    let (body, comment) = split_comment(rest);
    let mut body = body.trim();
    let mut num = None;
    // Optional leading number.
    let digits: usize = body.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        num = body[..digits].parse().ok();
        body = body[digits..].trim_start();
    }
    if let Some(d) = body.strip_prefix('-') {
        body = d.trim_start();
    }
    let directive = comment.as_deref().and_then(directive_of);
    Line::Point {
        num,
        ok,
        desc: body.to_string(),
        directive,
        indent,
    }
}

/// Parse a TAP stream. Always succeeds — the spec has harnesses
/// tolerate malformed producers, so bad lines surface as
/// [`Line::Unknown`], never a parse failure.
pub fn parse(input: &str) -> Tap {
    let mut lines = Vec::new();
    let mut in_yaml = false;
    let mut yaml_depth_indent = 0usize;
    for raw in input.lines() {
        let indent = raw.len() - raw.trim_start().len();
        let s = raw.trim();
        if in_yaml {
            if s == "..." || s == "....." {
                lines.push(Line::Yaml(raw.to_string()));
                in_yaml = false;
                continue;
            }
            if indent >= yaml_depth_indent && !s.is_empty() {
                lines.push(Line::Yaml(raw.to_string()));
                continue;
            }
            in_yaml = false;
        }
        if s.is_empty() {
            continue;
        }
        if let Some(v) = s.strip_prefix("TAP version ") {
            match v.trim().parse() {
                Ok(n) => lines.push(Line::Version(n)),
                Err(_) => lines.push(Line::Unknown(raw.to_string())),
            }
            continue;
        }
        if let Some(r) = s.strip_prefix("Bail out!") {
            lines.push(Line::BailOut(r.trim().to_string()));
            continue;
        }
        if let Some(c) = s.strip_prefix('#') {
            lines.push(Line::Comment(c.trim().to_string()));
            continue;
        }
        if s.contains("..") {
            if let Some((hi, comment)) = plan(s) {
                lines.push(Line::Plan { lo: 1, hi, comment });
                continue;
            }
        }
        if s == "---" && indent >= 2 {
            lines.push(Line::Yaml(raw.to_string()));
            in_yaml = true;
            yaml_depth_indent = indent;
            continue;
        }
        // "ok"/"not ok" test point?
        let (word, rest) = match s.split_once(' ') {
            Some((w, r)) => (w, r),
            None => (s, ""),
        };
        if word == "ok" || (word == "not" && (rest == "ok" || rest.starts_with("ok "))) {
            let rest = if word == "not" {
                rest[2..].trim_start()
            } else {
                rest
            };
            lines.push(point(word, rest, indent));
            continue;
        }
        lines.push(Line::Unknown(raw.to_string()));
    }
    Tap { lines }
}

impl Tap {
    /// The `TAP version N` line's N when present.
    pub fn version(&self) -> Option<u32> {
        self.lines.iter().find_map(|l| match l {
            Line::Version(n) => Some(*n),
            _ => None,
        })
    }

    /// The plan line, `(lo, hi)` — appears at most once per spec.
    pub fn plan(&self) -> Option<(u32, u32)> {
        self.lines.iter().find_map(|l| match l {
            Line::Plan { lo, hi, .. } => Some((*lo, *hi)),
            _ => None,
        })
    }

    /// `true` when the stream begins `1..0` — the whole-file skip.
    pub fn skipped_all(&self) -> bool {
        self.plan().is_some_and(|(lo, hi)| lo == 1 && hi == 0)
    }

    /// The `Bail out!` reason when the stream aborted.
    pub fn bail_out(&self) -> Option<&str> {
        self.lines.iter().find_map(|l| match l {
            Line::BailOut(r) => Some(r.as_str()),
            _ => None,
        })
    }

    /// Aggregate counts over all points.
    pub fn summary(&self) -> Summary {
        let mut s = Summary::default();
        for l in &self.lines {
            if let Line::Point {
                ok,
                directive,
                indent,
                ..
            } = l
            {
                s.points += 1;
                if *ok {
                    s.passed += 1;
                } else {
                    s.failed += 1;
                    if matches!(directive, Some(Directive::Todo(_))) {
                        s.todo += 1;
                    }
                }
                if matches!(directive, Some(Directive::Skip(_))) {
                    s.skipped += 1;
                }
                if *indent >= 4 {
                    s.subtests += 1;
                }
            }
        }
        s
    }

    /// Harness verdict: no bailout, no failure outside `# TODO`, and
    /// the plan (when declared) matches the top-level point count.
    pub fn ok(&self) -> bool {
        if self.bail_out().is_some() {
            return false;
        }
        let mut top = 0usize;
        for l in &self.lines {
            if let Line::Point {
                ok,
                directive,
                indent,
                ..
            } = l
            {
                if *indent == 0 {
                    top += 1;
                    if !ok && !matches!(directive, Some(Directive::Todo(_))) {
                        return false;
                    }
                } else if !ok && !matches!(directive, Some(Directive::Todo(_))) {
                    return false;
                }
            }
        }
        if let Some((_, hi)) = self.plan() {
            if hi == 0 {
                return top == 0;
            }
            return usize::try_from(hi).is_ok_and(|h| h == top);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_stream() {
        let t = parse(
            "TAP version 14\n\
             # heading\n\
             1..4\n\
             ok 1 - parse works\n\
             ok 2\n\
             not ok 3 - flaky # TODO known bug\n\
             ok 4 - empty # SKIP no fs\n\
             # done\n",
        );
        assert_eq!(t.version(), Some(14));
        assert_eq!(t.plan(), Some((1, 4)));
        let s = t.summary();
        assert_eq!(s.points, 4);
        assert_eq!(s.failed, 1);
        assert_eq!(s.todo, 1);
        assert_eq!(s.skipped, 1);
        assert!(t.ok());
        // descriptions: "- " stripped
        assert!(matches!(&t.lines[2], Line::Plan { .. }));
        let Line::Point { num, desc, .. } = &t.lines[3] else {
            unreachable!()
        };
        assert_eq!(*num, Some(1));
        assert_eq!(desc, "parse works");
    }

    #[test]
    fn plan_after_points_and_bail() {
        let t = parse("ok 1\nnot ok 2\nBail out! no db\n");
        assert_eq!(t.bail_out(), Some("no db"));
        assert!(!t.ok());
        let t2 = parse("ok\nok\n1..2\n");
        assert!(t2.ok()); // trailing plan accepted
        let t3 = parse("ok\n1..2\n"); // plan mismatch
        assert!(!t3.ok());
    }

    #[test]
    fn skip_all_and_yaml_and_subtests() {
        let t = parse("1..0 # skip no network\n");
        assert!(t.skipped_all());
        assert!(t.ok());
        let t = parse(
            "TAP version 14\n\
             1..2\n\
             not ok 1 - bad\n\
             \x20\x20---\n\
             \x20\x20message: nope\n\
             \x20\x20...\n\
             ok 2 - outer\n\
             \x20\x20\x20\x20ok 1 - sub\n",
        );
        let s = t.summary();
        assert_eq!(s.subtests, 1);
        assert!(!t.ok());
        assert!(t
            .lines
            .iter()
            .any(|l| matches!(l, Line::Yaml(y) if y.contains("message"))));
    }

    #[test]
    fn unknowns_are_kept() {
        let t = parse("nonsense line\nok\n");
        assert!(matches!(t.lines[0], Line::Unknown(_)));
        let s = t.summary();
        assert_eq!(s.points, 1);
        assert!(t.ok());
    }

    #[test]
    fn directives_case_insensitive() {
        let t = parse("not ok 1 - x # todo later\nok 2 - y # skip me\n");
        let s = t.summary();
        assert_eq!(s.todo, 1);
        assert_eq!(s.skipped, 1);
        assert!(t.ok());
    }
}

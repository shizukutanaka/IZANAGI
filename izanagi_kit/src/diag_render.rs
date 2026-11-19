//! Span-based, human-readable diagnostic rendering (C9).
//!
//! [`crate::diag_json`] serializes the pipeline's [`Diagnostic`] values for
//! machines; [`Diagnostic::render`] prints a one-line report with a single
//! caret for humans. Neither can express what a compiler's diagnostic
//! printer can: a finding that *underlines a range*, points at two places
//! at once, cross-references a related location, and tells the reader how
//! to fix it. [`RichDiagnostic`] is that richer model, and
//! [`RichDiagnostic::render`] is its text renderer — hand-written,
//! zero-dependency, in the style of the rustc diagnostic printer (the
//! `error[CODE]: message` header, the `--> file:line:col` locator, the
//! numbered source block with `^^^` primary and `---` secondary
//! underlines, and the trailing `= note:` / `= help:` lines). The format
//! owes its shape to rustc's emitter and to the ariadne / miette family
//! of diagnostic renderers; see `izanagi_kit/RESEARCH.md` section C9.
//!
//! # Format
//!
//! Given a diagnostic with code `E0103`, a primary span under a bad color
//! literal, and a help line, the output is:
//!
//! ```text
//! error[E0103]: invalid color literal
//!  --> dungeon.game:4:12
//!   |
//! 4 | tile wall #GGGGGG solid
//!   |            ^^^^^^^ only hex digits are allowed
//!   |
//!   = help: use a color like #00C4CC
//! ```
//!
//! The exact rules (all pinned by golden tests below):
//!
//! - The header is `{severity}: {message}`, or
//!   `{severity}[{code}]: {message}` when a code is present.
//! - The locator line names the primary span (or the first label when
//!   there is no primary span). Related notes with their own span get a
//!   `:::` locator, the marker rustc uses for secondary files.
//! - Source lines are shown once per line that carries a span, in
//!   ascending line order, with one underline row per label on that line.
//!   A bare gutter row separates the locator from the first source line,
//!   each pair of source lines, and the source blocks from the closing
//!   `=` lines.
//!   Rows on one line are ordered by start column, primary before
//!   secondary on ties. The gutter width fits the largest line number
//!   shown, so every `|` in one report shares a column.
//! - Columns count **chars, not bytes** — the same convention the parser
//!   uses — so an underline stays aligned on lines containing multi-byte
//!   characters. A span that starts past the end of its line renders as a
//!   single mark just past the last char; a span that ends past the line
//!   is clamped to the line's end. A span whose line is not in the source
//!   at all contributes no source row.
//! - Plain notes and the help line come last, as `= note:` / `= help:`
//!   rows, in the order they were added.
//! - Rendering is a pure function of the diagnostic, the file label and
//!   the source text: the same inputs always produce the same bytes.
//!
//! Spans are single-line. The `.game` pipeline is line-based (one
//! statement per line, bounded line length), so no diagnostic it can
//! produce needs to underline across a line break; a multi-line span
//! model would be machinery without a caller.

use std::collections::BTreeMap;

use crate::content::{Diagnostic, Severity};

/// A single-line source range: 1-based `line`, and the 1-based char
/// columns `start..end` (end exclusive) on that line.
///
/// Columns count chars, not bytes — the convention [`crate::parser`]
/// already uses for [`Diagnostic::col`], so a span and a diagnostic
/// column are directly comparable. An empty span (`start == end`) is a
/// point: it renders as a single mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// 1-based source line.
    pub line: usize,
    /// 1-based char column of the first char in the span.
    pub start: usize,
    /// 1-based char column one past the last char in the span.
    pub end: usize,
}

impl Span {
    /// A span over `start..end` (end exclusive) on `line`. An `end`
    /// before `start` is clamped up to `start`, so the range is never
    /// inverted.
    pub fn new(line: usize, start: usize, end: usize) -> Self {
        Self {
            line,
            start,
            end: end.max(start),
        }
    }

    /// A one-char span at `col` — the rich form of a bare
    /// [`Diagnostic::col`].
    pub fn point(line: usize, col: usize) -> Self {
        Self {
            line,
            start: col,
            end: col.saturating_add(1),
        }
    }

    /// Number of chars the span covers (`0` for a point span).
    pub fn len_chars(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// `true` when the span covers no chars (a point).
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

/// A secondary underline: a [`Span`] plus the text printed beside it.
///
/// Labels render with `-` marks (the primary span uses `^`), following
/// the rustc convention for secondary spans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// Where the label points.
    pub span: Span,
    /// Text printed after the underline; may be empty for a bare mark.
    pub message: String,
}

impl Label {
    /// A label on `span` with the given `message`.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// A related note: context for a diagnostic, optionally tied to its own
/// source location.
///
/// A note with a span renders as its own source block under a `:::`
/// locator; a note without one renders as an `= note:` line after the
/// main block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The note text.
    pub message: String,
    /// The location the note refers to, if it has one.
    pub span: Option<Span>,
}

impl Note {
    /// A note with no source location (an `= note:` line).
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    /// A note anchored at `span`, rendered as its own source block.
    pub fn at(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
        }
    }
}

/// A diagnostic with spans, labels, related notes and a help line.
///
/// The rich counterpart of [`Diagnostic`]: where `Diagnostic` carries a
/// single line/column, `RichDiagnostic` carries a primary [`Span`] with
/// an optional label, any number of secondary [`Label`]s and [`Note`]s,
/// an optional diagnostic code (rendered as `error[E0103]`), and an
/// optional help line. Construct with [`RichDiagnostic::error`] /
/// [`RichDiagnostic::warning`], refine with the `with_*` / `add_*`
/// chainable setters, or convert an existing pipeline finding with
/// [`RichDiagnostic::from_diagnostic`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichDiagnostic {
    /// Error or warning.
    pub severity: Severity,
    /// Optional diagnostic code, rendered as `error[E0103]`.
    pub code: Option<String>,
    /// Human-readable description.
    pub message: String,
    /// The primary span, underlined with `^`.
    pub span: Option<Span>,
    /// Text printed beside the primary underline.
    pub label: Option<String>,
    /// Secondary labels, underlined with `-`.
    pub labels: Vec<Label>,
    /// Related notes, in the order they were added.
    pub notes: Vec<Note>,
    /// Optional fix-it guidance, rendered as the final `= help:` line.
    pub help: Option<String>,
}

/// The underline rows gathered for one source line during rendering:
/// `(span, label text, is-primary)`.
type LineRows<'a> = Vec<(Span, Option<&'a str>, bool)>;

impl RichDiagnostic {
    /// An error diagnostic with no code, span, labels, notes or help.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: None,
            message: message.into(),
            span: None,
            label: None,
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    /// A warning diagnostic with no code, span, labels, notes or help.
    pub fn warning(message: impl Into<String>) -> Self {
        let mut d = Self::error(message);
        d.severity = Severity::Warning;
        d
    }

    /// Convert a pipeline [`Diagnostic`] to its rich form.
    ///
    /// A diagnostic with a known column becomes a one-char primary span
    /// at that column. A diagnostic with a line but no column (a
    /// semantic check) gets no span: pointing a caret at a fabricated
    /// column would claim a precision the finding does not have — the
    /// same reason [`crate::diag_json::diag_sarif`] omits the region for
    /// position-less diagnostics.
    pub fn from_diagnostic(d: &Diagnostic) -> Self {
        let mut rich = match d.severity {
            Severity::Error => Self::error(d.message.clone()),
            Severity::Warning => Self::warning(d.message.clone()),
        };
        if d.line > 0 && d.col > 0 {
            rich.span = Some(Span::point(d.line, d.col));
        }
        rich
    }

    /// Set the diagnostic code (rendered as `error[E0103]`).
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set the primary span.
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Set the text printed beside the primary underline.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add a secondary label.
    pub fn add_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Add a related note.
    pub fn add_note(mut self, note: Note) -> Self {
        self.notes.push(note);
        self
    }

    /// Set the help line.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// `true` if this diagnostic is error-severity.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Render the diagnostic as rustc-style text against `source`,
    /// labelled with `file`. See the module docs for the exact format.
    ///
    /// The output has no trailing newline; callers printing several
    /// diagnostics join or terminate them as they see fit (the same
    /// contract as [`Diagnostic::render`]).
    pub fn render(&self, file: &str, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let displayable = |span: &Span| span.line >= 1 && span.line <= lines.len();

        // Gutter width: digits of the largest line number actually
        // shown, so every block in this report aligns. One when no
        // source row is shown at all.
        let mut width = 1usize;
        let mut consider = |span: &Span| {
            if displayable(span) {
                width = width.max(digit_count(span.line));
            }
        };
        if let Some(span) = &self.span {
            consider(span);
        }
        for label in &self.labels {
            consider(&label.span);
        }
        for note in &self.notes {
            if let Some(span) = &note.span {
                consider(span);
            }
        }

        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = match &self.code {
            Some(code) => format!("{sev}[{code}]: {}", self.message),
            None => format!("{sev}: {}", self.message),
        };

        // The locator names the primary span, falling back to the
        // earliest label when the diagnostic itself has no position.
        let locator = self.span.or_else(|| {
            self.labels
                .iter()
                .map(|l| l.span)
                .min_by_key(|s| (s.line, s.start))
        });
        if let Some(span) = locator {
            if span.line >= 1 {
                out.push('\n');
                out.push_str(&" ".repeat(width));
                out.push_str(&format!("--> {file}:{}:{}", span.line, span.start));
            }
        }

        let mut shown_block = false;

        // Main block: the primary span and every secondary label,
        // grouped by line.
        let mut groups: BTreeMap<usize, LineRows<'_>> = BTreeMap::new();
        if let Some(span) = &self.span {
            if displayable(span) {
                groups
                    .entry(span.line)
                    .or_default()
                    .push((*span, self.label.as_deref(), true));
            }
        }
        for label in &self.labels {
            if displayable(&label.span) {
                groups.entry(label.span.line).or_default().push((
                    label.span,
                    Some(label.message.as_str()),
                    false,
                ));
            }
        }
        for (line_no, mut rows) in groups {
            // One deterministic order per line: by start column, then
            // primary before secondary, then wider spans first, then
            // message text — so two labels on the same span still have
            // exactly one rendering.
            rows.sort_by(|a, b| {
                a.0.start
                    .cmp(&b.0.start)
                    .then(b.2.cmp(&a.2))
                    .then(b.0.end.cmp(&a.0.end))
                    .then(a.1.cmp(&b.1))
            });
            push_separator(&mut out, width);
            shown_block = true;
            push_source_line(&mut out, width, line_no, lines[line_no - 1]);
            for (span, text, primary) in rows {
                push_underline(&mut out, width, lines[line_no - 1], span, primary, text);
            }
        }

        // Related notes with a location get their own block under a
        // `:::` locator, in the order they were added.
        for note in &self.notes {
            if let Some(span) = &note.span {
                out.push('\n');
                out.push_str(&format!("note: {}", note.message));
                if span.line >= 1 {
                    out.push('\n');
                    out.push_str(&" ".repeat(width));
                    out.push_str(&format!("::: {file}:{}:{}", span.line, span.start));
                }
                if displayable(span) {
                    shown_block = true;
                    push_separator(&mut out, width);
                    push_source_line(&mut out, width, span.line, lines[span.line - 1]);
                    push_underline(&mut out, width, lines[span.line - 1], *span, false, None);
                }
            }
        }

        // Plain notes and the help line close the report.
        let has_plain_notes = self.notes.iter().any(|n| n.span.is_none());
        if has_plain_notes || self.help.is_some() {
            if shown_block {
                push_separator(&mut out, width);
            }
            for note in &self.notes {
                if note.span.is_none() {
                    out.push('\n');
                    out.push_str(&" ".repeat(width + 1));
                    out.push_str(&format!("= note: {}", note.message));
                }
            }
            if let Some(help) = &self.help {
                out.push('\n');
                out.push_str(&" ".repeat(width + 1));
                out.push_str(&format!("= help: {help}"));
            }
        }

        out
    }
}

/// Number of decimal digits in `n` (`n >= 1` at every call site).
fn digit_count(mut n: usize) -> usize {
    let mut digits = 1;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    digits
}

/// Append a gutter separator row (`  |` at the shared column).
fn push_separator(out: &mut String, width: usize) {
    out.push('\n');
    out.push_str(&" ".repeat(width + 1));
    out.push('|');
}

/// Append a numbered source row (` 4 | text`).
fn push_source_line(out: &mut String, width: usize, line_no: usize, text: &str) {
    out.push('\n');
    out.push_str(&format!("{line_no:>width$} | {text}"));
}

/// Append one underline row for `span` on `text`.
///
/// `primary` selects `^` marks over `-`. The mark run is computed in
/// chars: it starts at the span's first char, is clamped to the end of
/// the line, and is at least one mark wide — a span that starts past
/// the end of the line gets a single mark just past the last char.
fn push_underline(
    out: &mut String,
    width: usize,
    text: &str,
    span: Span,
    primary: bool,
    label: Option<&str>,
) {
    let line_len = text.chars().count();
    let first = span.start.saturating_sub(1);
    let (offset, count) = if first >= line_len {
        (line_len, 1)
    } else {
        let upto = span.end.saturating_sub(1).min(line_len);
        (first, upto.saturating_sub(first).max(1))
    };
    out.push('\n');
    out.push_str(&" ".repeat(width + 1));
    out.push_str("| ");
    out.push_str(&" ".repeat(offset));
    let mark = if primary { '^' } else { '-' };
    for _ in 0..count {
        out.push(mark);
    }
    if let Some(text) = label {
        if !text.is_empty() {
            out.push(' ');
            out.push_str(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "prefab orc\n  glyph o\ntile wall #GGGGGG solid\nlevel x 3x2\n";

    #[test]
    fn test_span_new_clamps_inverted_range() {
        let span = Span::new(2, 9, 4);
        assert_eq!(span.end, 9);
        assert!(span.is_empty());
        assert_eq!(span.len_chars(), 0);
    }

    #[test]
    fn test_span_point_covers_one_char() {
        let span = Span::point(3, 7);
        assert_eq!(span.len_chars(), 1);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_single_label_with_code_and_help_is_pinned() {
        let d = RichDiagnostic::error("invalid color literal")
            .with_code("E0103")
            .with_span(Span::new(3, 12, 19))
            .with_label("only hex digits are allowed")
            .with_help("use a color like #00C4CC");
        let expected = "\
error[E0103]: invalid color literal
 --> dungeon.game:3:12
  |
3 | tile wall #GGGGGG solid
  |            ^^^^^^^ only hex digits are allowed
  |
  = help: use a color like #00C4CC";
        assert_eq!(d.render("dungeon.game", SRC), expected);
    }

    #[test]
    fn test_multiple_labels_on_one_line_are_pinned() {
        // Rows order by start column; the primary span keeps `^`, the
        // secondary labels take `-`, regardless of insertion order.
        let d = RichDiagnostic::error("row has extra cells")
            .with_span(Span::new(3, 6, 10))
            .with_label("this tile")
            .add_label(Label::new(Span::new(3, 18, 23), "not a flag"))
            .add_label(Label::new(Span::new(3, 12, 18), "bad color"));
        let expected = "\
error: row has extra cells
 --> dungeon.game:3:6
  |
3 | tile wall #GGGGGG solid
  |      ^^^^ this tile
  |            ------ bad color
  |                  ----- not a flag";
        assert_eq!(d.render("dungeon.game", SRC), expected);
    }

    #[test]
    fn test_labels_across_lines_are_pinned() {
        let d = RichDiagnostic::warning("glyph reused")
            .with_span(Span::new(2, 10, 11))
            .with_label("used here")
            .add_label(Label::new(Span::new(4, 1, 6), "level name"));
        let expected = "\
warning: glyph reused
 --> dungeon.game:2:10
  |
2 |   glyph o
  |          ^ used here
  |
4 | level x 3x2
  | ----- level name";
        assert_eq!(d.render("dungeon.game", SRC), expected);
    }

    #[test]
    fn test_related_notes_and_help_are_pinned() {
        let d = RichDiagnostic::error("duplicate prefab name")
            .with_code("E0201")
            .with_span(Span::new(1, 8, 11))
            .with_label("redefined here")
            .add_note(Note::at(
                Span::new(1, 1, 7),
                "the first definition is above",
            ))
            .add_note(Note::new("prefab names must be unique per file"))
            .with_help("rename one of the prefabs");
        // The spanned note points back at the same line here, so its
        // block follows the main block directly.
        let expected = "\
error[E0201]: duplicate prefab name
 --> dungeon.game:1:8
  |
1 | prefab orc
  |        ^^^ redefined here
note: the first definition is above
 ::: dungeon.game:1:1
  |
1 | prefab orc
  | ------
  |
  = note: prefab names must be unique per file
  = help: rename one of the prefabs";
        assert_eq!(d.render("dungeon.game", SRC), expected);
    }

    #[test]
    fn test_multibyte_line_keeps_caret_on_the_char() {
        // Columns count chars: the span covers the two kana after
        // "tile " — byte counting would push the marks three columns
        // too far right per char.
        let src = "tile 壁画 #A1B2C3\n";
        let d = RichDiagnostic::error("glyph must be one character")
            .with_span(Span::new(1, 6, 8))
            .with_label("two characters wide");
        let expected = "\
error: glyph must be one character
 --> dungeon.game:1:6
  |
1 | tile 壁画 #A1B2C3
  |      ^^ two characters wide";
        assert_eq!(d.render("dungeon.game", src), expected);
    }

    #[test]
    fn test_span_past_line_end_is_clamped() {
        // A span ending past the line stops at the last char; a span
        // starting past the line becomes one mark just past the end.
        let src = "level x 3x2\n";
        let d = RichDiagnostic::error("level header is malformed")
            .with_span(Span::new(1, 9, 99))
            .with_label("runs off the line")
            .add_label(Label::new(Span::new(1, 30, 34), "beyond the text"));
        let expected = "\
error: level header is malformed
 --> dungeon.game:1:9
  |
1 | level x 3x2
  |         ^^^ runs off the line
  |            - beyond the text";
        assert_eq!(d.render("dungeon.game", src), expected);
    }

    #[test]
    fn test_no_span_renders_header_and_help_only() {
        let d = RichDiagnostic::warning("file has no levels").with_help("add a level section");
        let expected = "\
warning: file has no levels
  = help: add a level section";
        assert_eq!(d.render("dungeon.game", SRC), expected);
    }

    #[test]
    fn test_span_on_missing_line_shows_no_source_row() {
        let d = RichDiagnostic::error("stale position")
            .with_span(Span::new(99, 1, 4))
            .with_label("nowhere");
        let expected = "\
error: stale position
 --> dungeon.game:99:1";
        assert_eq!(d.render("dungeon.game", SRC), expected);
    }

    #[test]
    fn test_empty_label_text_leaves_no_trailing_space() {
        let d = RichDiagnostic::error("bad").with_span(Span::point(4, 7));
        let rendered = d.render("dungeon.game", SRC);
        let expected = "\
error: bad
 --> dungeon.game:4:7
  |
4 | level x 3x2
  |       ^";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn test_from_diagnostic_with_column_becomes_point_span() {
        let d = Diagnostic::error_at(3, 12, "invalid color literal");
        let rich = RichDiagnostic::from_diagnostic(&d);
        assert!(rich.is_error());
        let expected = "\
error: invalid color literal
 --> dungeon.game:3:12
  |
3 | tile wall #GGGGGG solid
  |            ^";
        assert_eq!(rich.render("dungeon.game", SRC), expected);
    }

    #[test]
    fn test_from_diagnostic_without_column_has_no_span() {
        let d = Diagnostic::warning(0, "unused prefab");
        let rich = RichDiagnostic::from_diagnostic(&d);
        assert_eq!(rich.span, None);
        assert_eq!(rich.render("dungeon.game", SRC), "warning: unused prefab");
    }

    #[test]
    fn test_render_is_deterministic() {
        let d = RichDiagnostic::error("invalid color literal")
            .with_code("E0103")
            .with_span(Span::new(3, 12, 19))
            .with_label("only hex digits are allowed")
            .add_label(Label::new(Span::new(3, 6, 10), "this tile"))
            .add_note(Note::new("colors are case-insensitive"));
        assert_eq!(d.render("dungeon.game", SRC), d.render("dungeon.game", SRC));
    }

    #[test]
    fn test_wide_line_numbers_widen_the_gutter() {
        let src = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\ntile wall\n";
        let d = RichDiagnostic::error("bad tile").with_span(Span::new(12, 6, 10));
        let expected = "\
error: bad tile
  --> dungeon.game:12:6
   |
12 | tile wall
   |      ^^^^";
        assert_eq!(d.render("dungeon.game", src), expected);
    }
}

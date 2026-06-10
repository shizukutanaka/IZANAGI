//! Machine-readable JSON serialization of pipeline diagnostics (P4).
//!
//! `diag_json` serializes [`Diagnostic`] values into a compact JSON object
//! consumable by CI tools, editors, and LSP clients — no external dependencies,
//! pure hand-written JSON.
//!
//! Output schema:
//! ```json
//! {
//!   "file": "levels/world.game",
//!   "diagnostics": [
//!     {"severity": "error", "line": 3, "col": 5, "message": "unexpected token"},
//!     {"severity": "warning", "line": 7, "col": 0, "message": "unused prefab"}
//!   ],
//!   "errors": 1,
//!   "warnings": 1
//! }
//! ```
//!
//! `col` is 1-based; `0` means column unknown (semantic checks with no source
//! position).

use crate::content::{Diagnostic, Severity};

/// Escape `s` for use inside a JSON string literal.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Count errors and warnings in a single pass over `diags`.
/// Returns `(errors, warnings)`. Saves the double-filter boilerplate of
/// `severity_filter(…, Error).len()` + `severity_filter(…, Warning).len()`
/// Returns `true` when `diags` contains at least one error-severity diagnostic.
/// Thin wrapper around `diag_count` for the common "abort if any errors" CI gate.
#[inline]
pub fn has_errors(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.is_error())
}

/// when both counts are needed at once (e.g. for a CI summary line like
/// "3 errors, 2 warnings").
pub fn diag_count(diags: &[Diagnostic]) -> (usize, usize) {
    let errors = diags.iter().filter(|d| d.is_error()).count();
    (errors, diags.len() - errors)
}

/// Return only the diagnostics matching `severity`, preserving input order.
///
/// Useful for splitting a combined diagnostic list into errors-only or
/// warnings-only lists before rendering or routing to separate outputs.
pub fn severity_filter(diags: &[Diagnostic], severity: Severity) -> Vec<Diagnostic> {
    diags
        .iter()
        .filter(|d| d.severity == severity)
        .cloned()
        .collect()
}

/// Serialize `diags` to a JSON object string for `file`.
///
/// The returned string is a self-contained JSON object (no trailing newline).
/// Collect parse and validate diagnostics with `.chain()` before calling, or
/// call twice and merge the arrays in the output if you need separate sections.
pub fn diag_json(file: &str, diags: &[Diagnostic]) -> String {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for d in diags {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
        }
    }

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"file\": \"{}\",\n", json_escape(file)));
    out.push_str("  \"diagnostics\": [");

    for (i, d) in diags.iter().enumerate() {
        let sev = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        if i == 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "    {{\"severity\": \"{sev}\", \"line\": {}, \"col\": {}, \"message\": \"{}\"}}",
            d.line,
            d.col,
            json_escape(&d.message),
        ));
        if i + 1 < diags.len() {
            out.push_str(",\n");
        } else {
            out.push('\n');
        }
    }

    if diags.is_empty() {
        out.push_str("],\n");
    } else {
        out.push_str("  ],\n");
    }
    out.push_str(&format!("  \"errors\": {errors},\n"));
    out.push_str(&format!("  \"warnings\": {warnings}\n"));
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Diagnostic;

    fn parse_json_field<'a>(json: &'a str, key: &str) -> &'a str {
        let needle = format!("\"{key}\": ");
        let start = json.find(&needle).unwrap() + needle.len();
        let rest = &json[start..];
        // Return up to the next comma or newline (strip quotes if present).
        let end = rest.find([',', '\n', '}']).unwrap_or(rest.len());
        rest[..end].trim().trim_matches('"')
    }

    #[test]
    fn test_empty_diags_produces_valid_json() {
        let json = diag_json("test.game", &[]);
        assert!(json.contains("\"file\": \"test.game\""));
        assert!(json.contains("\"diagnostics\": []"));
        assert!(json.contains("\"errors\": 0"));
        assert!(json.contains("\"warnings\": 0"));
    }

    #[test]
    fn test_single_error() {
        let diags = vec![Diagnostic::error(3, "unexpected token")];
        let json = diag_json("a.game", &diags);
        assert!(json.contains("\"severity\": \"error\""));
        assert!(json.contains("\"line\": 3"));
        assert!(json.contains("\"message\": \"unexpected token\""));
        assert!(json.contains("\"errors\": 1"));
        assert!(json.contains("\"warnings\": 0"));
    }

    #[test]
    fn test_single_warning() {
        let diags = vec![Diagnostic::warning(7, "unused prefab")];
        let json = diag_json("b.game", &diags);
        assert!(json.contains("\"severity\": \"warning\""));
        assert!(json.contains("\"errors\": 0"));
        assert!(json.contains("\"warnings\": 1"));
    }

    #[test]
    fn test_mixed_diags_counts() {
        let diags = vec![
            Diagnostic::error(1, "bad glyph"),
            Diagnostic::warning(2, "unused tile"),
            Diagnostic::error(3, "dup prefab"),
        ];
        let json = diag_json("c.game", &diags);
        assert!(json.contains("\"errors\": 2"));
        assert!(json.contains("\"warnings\": 1"));
    }

    #[test]
    fn test_col_included() {
        let diags = vec![Diagnostic::error_at(5, 12, "bad value")];
        let json = diag_json("d.game", &diags);
        assert!(json.contains("\"col\": 12"));
    }

    #[test]
    fn test_col_zero_when_unknown() {
        let diags = vec![Diagnostic::error(0, "semantic error")];
        let json = diag_json("e.game", &diags);
        assert!(json.contains("\"col\": 0"));
    }

    #[test]
    fn test_message_with_special_chars_is_escaped() {
        let diags = vec![Diagnostic::error(1, "has \"quotes\" and \\backslash")];
        let json = diag_json("f.game", &diags);
        assert!(json.contains("\\\"quotes\\\""));
        assert!(json.contains("\\\\backslash"));
    }

    #[test]
    fn test_file_with_special_chars_is_escaped() {
        let json = diag_json("path/with \"spaces\".game", &[]);
        assert!(json.contains("\\\"spaces\\\""));
    }

    #[test]
    fn test_output_starts_with_brace() {
        let json = diag_json("x.game", &[]);
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_multiple_diags_ordering_preserved() {
        let diags = vec![
            Diagnostic::error(1, "first"),
            Diagnostic::warning(2, "second"),
            Diagnostic::error(3, "third"),
        ];
        let json = diag_json("g.game", &diags);
        let first_pos = json.find("first").unwrap();
        let second_pos = json.find("second").unwrap();
        let third_pos = json.find("third").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < third_pos);
    }

    #[test]
    fn test_file_field_appears_before_diagnostics() {
        let json = diag_json("h.game", &[]);
        let file_pos = json.find("\"file\"").unwrap();
        let diags_pos = json.find("\"diagnostics\"").unwrap();
        assert!(file_pos < diags_pos);
    }

    #[test]
    fn test_newline_in_message_escaped() {
        let diags = vec![Diagnostic::error(1, "line\nnewline")];
        let json = diag_json("i.game", &diags);
        assert!(json.contains("\\n"));
        assert!(!json[json.find("line").unwrap()..].starts_with("line\n"));
    }

    #[test]
    fn test_parse_file_value() {
        let json = diag_json("myfile.game", &[]);
        assert_eq!(parse_json_field(&json, "file"), "myfile.game");
    }

    #[test]
    fn test_error_count_matches() {
        let diags: Vec<Diagnostic> = (0..5).map(|i| Diagnostic::error(i, "e")).collect();
        let json = diag_json("j.game", &diags);
        assert!(json.contains("\"errors\": 5"));
    }

    // --- severity_filter ---

    #[test]
    fn test_severity_filter_errors_only() {
        let diags = vec![
            Diagnostic::error(1, "e1"),
            Diagnostic::warning(2, "w1"),
            Diagnostic::error(3, "e2"),
        ];
        let errors = severity_filter(&diags, Severity::Error);
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn test_severity_filter_warnings_only() {
        let diags = vec![
            Diagnostic::error(1, "e"),
            Diagnostic::warning(2, "w1"),
            Diagnostic::warning(3, "w2"),
        ];
        let warnings = severity_filter(&diags, Severity::Warning);
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().all(|d| d.severity == Severity::Warning));
    }

    #[test]
    fn test_severity_filter_empty_input_returns_empty() {
        let result = severity_filter(&[], Severity::Error);
        assert!(result.is_empty());
    }

    #[test]
    fn test_severity_filter_preserves_order() {
        let diags = vec![
            Diagnostic::error(5, "last"),
            Diagnostic::warning(1, "skip"),
            Diagnostic::error(3, "middle"),
            Diagnostic::error(1, "first"),
        ];
        let errors = severity_filter(&diags, Severity::Error);
        assert_eq!(errors[0].line, 5);
        assert_eq!(errors[1].line, 3);
        assert_eq!(errors[2].line, 1);
    }

    #[test]
    fn test_diag_count_empty_is_zero_zero() {
        assert_eq!(diag_count(&[]), (0, 0));
    }

    #[test]
    fn test_diag_count_returns_errors_and_warnings() {
        let diags = vec![
            Diagnostic::error(1, "e1"),
            Diagnostic::warning(2, "w1"),
            Diagnostic::error(3, "e2"),
        ];
        assert_eq!(diag_count(&diags), (2, 1));
    }

    #[test]
    fn test_diag_count_all_warnings() {
        let diags = vec![Diagnostic::warning(1, "a"), Diagnostic::warning(2, "b")];
        let (errors, warnings) = diag_count(&diags);
        assert_eq!(errors, 0);
        assert_eq!(warnings, 2);
    }

    #[test]
    fn test_has_errors_true_when_error_present() {
        let diags = vec![Diagnostic::error(1, "oops"), Diagnostic::warning(2, "meh")];
        assert!(has_errors(&diags));
    }

    #[test]
    fn test_has_errors_false_when_only_warnings() {
        let diags = vec![Diagnostic::warning(1, "meh")];
        assert!(!has_errors(&diags));
    }

    #[test]
    fn test_has_errors_false_on_empty() {
        assert!(!has_errors(&[]));
    }
}

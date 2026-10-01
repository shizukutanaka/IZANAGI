//! Mocha JSON reporter output census.
//!
//! `mocha --reporter json` emits `{ "stats": { "suites", "tests", "passes",
//! "pending", "failures", "duration" }, "tests": [ { "title", "fullTitle",
//! "duration", "currentRetry", "speed"/"result", "err" } ], "pending": [...],
//! "failures": [...], "passes": [...] }`. `parse` counts test entries and the
//! entries of each result array.
//!
//! ```rust
//! let m = br#"{
//!   "stats": { "suites": 1, "tests": 2, "passes": 1, "failures": 1 },
//!   "tests": [
//!     { "title": "a", "fullTitle": "s a", "duration": 1, "err": {} },
//!     { "title": "b", "fullTitle": "s b", "duration": 2, "err": { "message": "x" } }
//!   ],
//!   "passes": [ { "title": "a", "fullTitle": "s a" } ],
//!   "failures": [ { "title": "b", "fullTitle": "s b" } ],
//!   "pending": []
//! }"#;
//! let c = izanagi_kit::mochajson::Mochajson::parse(m).unwrap();
//! assert_eq!(c.tests, 2);
//! assert_eq!(c.passes, 1);
//! assert_eq!(c.failures, 1);
//! ```

/// Mocha JSON reporter census counts.
#[derive(Debug, Clone)]
pub struct Mochajson {
    /// `"stats"` object present (0 or 1).
    pub stats: usize,
    /// Entries of the top-level `"tests"` array.
    pub tests: usize,
    /// `"fullTitle"` occurrences across all arrays.
    pub full_titles: usize,
    /// `"duration":` entries.
    pub durations: usize,
    /// Entries of the `"passes"` array.
    pub passes: usize,
    /// Entries of the `"failures"` array.
    pub failures: usize,
    /// Entries of the `"pending"` array.
    pub pending: usize,
    /// Non-empty `"err"` objects.
    pub errors: usize,
}

fn array_len(t: &str, key: &str) -> usize {
    // Top-level arrays appear after `stats`, whose copy of each key
    // would match first — take the last occurrence of `"key"`.
    let Some(pos) = t.rfind(&format!("\"{key}\"")) else {
        return 0;
    };
    let Some(open) = t[pos..].find('[').map(|o| pos + o) else {
        return 0;
    };
    let mut depth = 0usize;
    let mut items = 0usize;
    let mut seen_item = false;
    let mut in_str = false;
    for ch in t[open..].chars() {
        if in_str {
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_str = true;
                if depth == 1 {
                    seen_item = true;
                }
            }
            '[' | '{' => {
                depth += 1;
                if depth == 2 {
                    items += 1;
                    seen_item = true;
                }
            }
            ']' | '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && ch == ']' {
                    break;
                }
            }
            _ if !ch.is_whitespace() && depth == 1 => seen_item = true,
            _ => {}
        }
    }
    if items == 0 && seen_item {
        1
    } else {
        items
    }
}

/// Whether the buffer looks like Mocha JSON reporter output.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"stats\"") && t.contains("\"fullTitle\"")
}

impl Mochajson {
    /// Parse Mocha JSON reporter output into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let err_objs = t.matches("\"err\": {").count();
        let err_empty = t.matches("\"err\": {}").count();
        Some(Self {
            stats: t.matches("\"stats\"").count(),
            tests: array_len(t, "tests"),
            full_titles: t.matches("\"fullTitle\"").count(),
            durations: t.matches("\"duration\":").count(),
            passes: array_len(t, "passes"),
            failures: array_len(t, "failures"),
            pending: array_len(t, "pending"),
            errors: err_objs.saturating_sub(err_empty),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_report() {
        let b = br#"{
          "stats": { "suites": 1, "tests": 3, "passes": 1, "pending": 1, "failures": 1 },
          "tests": [
            { "title": "a", "fullTitle": "s a", "duration": 1, "err": {} },
            { "title": "b", "fullTitle": "s b", "duration": 2, "err": { "m": 1 } },
            { "title": "c", "fullTitle": "s c" }
          ],
          "pending": [ { "title": "c", "fullTitle": "s c" } ],
          "failures": [ { "title": "b", "fullTitle": "s b" } ],
          "passes": [ { "title": "a", "fullTitle": "s a" } ]
        }"#;
        let c = Mochajson::parse(b).unwrap();
        assert_eq!(c.tests, 3);
        assert_eq!(c.full_titles, 6);
        assert_eq!(c.durations, 2);
        assert_eq!(c.passes, 1);
        assert_eq!(c.failures, 1);
        assert_eq!(c.pending, 1);
        assert_eq!(c.errors, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Mochajson::parse(b"{ \"stats\": {} }").is_none());
    }
}

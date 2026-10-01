//! Maven Surefire TXT summary (`*.txt`) census.
//!
//! Surefire text reports contain `Test set: com.example.XTest` headers and
//! `Tests run: N, Failures: N, Errors: N, Skipped: N, Time elapsed: N s`
//! summary lines, with `<<< FAILURE!` markers on failing sets. `parse` sums
//! the numeric fields across all summary lines.
//!
//! ```rust
//! let t = b"Test set: com.example.XTest\n\
//!   Tests run: 3, Failures: 1, Errors: 0, Skipped: 0, Time elapsed: 2 s\n\
//!   Test set: com.example.YTest <<< FAILURE!\n\
//!   Tests run: 2, Failures: 1, Errors: 0, Skipped: 1, Time elapsed: 1 s\n";
//! let c = izanagi_kit::surefire::Surefire::parse(t).unwrap();
//! assert_eq!(c.test_sets, 2);
//! assert_eq!(c.total_tests, 5);
//! assert_eq!(c.total_failures, 2);
//! ```

/// Surefire TXT report census.
#[derive(Debug, Clone)]
pub struct Surefire {
    /// `Test set:` header lines.
    pub test_sets: usize,
    /// `Tests run:` summary lines.
    pub runs: usize,
    /// Summed `Tests run:` values.
    pub total_tests: usize,
    /// Summed `Failures:` values.
    pub total_failures: usize,
    /// Summed `Errors:` values.
    pub total_errors: usize,
    /// Summed `Skipped:` values.
    pub total_skipped: usize,
    /// Lines containing `FAILURE` markers.
    pub failure_lines: usize,
}

fn field(line: &str, key: &str) -> usize {
    let Some(pos) = line.find(key) else {
        return 0;
    };
    let mut n = 0usize;
    let mut seen = false;
    for ch in line[pos + key.len()..].chars() {
        if ch.is_ascii_digit() {
            n = n * 10 + (ch as usize - '0' as usize);
            seen = true;
        } else {
            break;
        }
    }
    if seen {
        n
    } else {
        0
    }
}

/// Whether the buffer looks like a Surefire TXT report.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("Tests run:")
        && (t.contains("Test set:") || t.contains("FAILURE") || t.contains("Time elapsed"))
}

impl Surefire {
    /// Parse a Surefire TXT report into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            test_sets: 0,
            runs: 0,
            total_tests: 0,
            total_failures: 0,
            total_errors: 0,
            total_skipped: 0,
            failure_lines: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.starts_with("Test set:") {
                c.test_sets += 1;
            }
            if l.contains("FAILURE") {
                c.failure_lines += 1;
            }
            if l.contains("Tests run:") {
                c.runs += 1;
                c.total_tests += field(l, "Tests run: ");
                c.total_failures += field(l, "Failures: ");
                c.total_errors += field(l, "Errors: ");
                c.total_skipped += field(l, "Skipped: ");
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_report() {
        let b = b"-------------------------------------------------------------------------------
Test set: com.example.AppTest
-------------------------------------------------------------------------------
Tests run: 2, Failures: 0, Errors: 0, Skipped: 0, Time elapsed: 1 s
Test set: com.example.BadTest <<< FAILURE!
Tests run: 3, Failures: 2, Errors: 1, Skipped: 1, Time elapsed: 4 s
";
        let c = Surefire::parse(b).unwrap();
        assert_eq!(c.test_sets, 2);
        assert_eq!(c.runs, 2);
        assert_eq!(c.total_tests, 5);
        assert_eq!(c.total_failures, 2);
        assert_eq!(c.total_errors, 1);
        assert_eq!(c.total_skipped, 1);
        assert_eq!(c.failure_lines, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Surefire::parse(b"hello").is_none());
    }
}

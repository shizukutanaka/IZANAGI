//! CTRF (Common Test Report Format) JSON census.
//!
//! A CTRF document is `{ "results": { "summary": { ... }, "tests": [
//! { "name": ..., "status": "passed"|"failed"|"skipped"|"pending"|"other",
//!   "duration": N, ... } ], "environment": { ... } } }`. `parse` counts test
//! entries by status plus the `summary`/`environment`/`suite`/`extra` keys.
//!
//! ```rust
//! let r = br#"{
//!   "results": {
//!     "summary": { "tests": 2, "passed": 1, "failed": 1 },
//!     "tests": [
//!       { "name": "a", "status": "passed", "duration": 10 },
//!       { "name": "b", "status": "failed", "duration": 20, "flaky": false }
//!     ],
//!     "environment": { "os": "linux" }
//!   }
//! }"#;
//! let c = izanagi_kit::ctrf::Ctrf::parse(r).unwrap();
//! assert_eq!(c.tests, 2);
//! assert_eq!(c.passed, 1);
//! assert_eq!(c.failed, 1);
//! ```

/// CTRF JSON census counts.
#[derive(Debug, Clone)]
pub struct Ctrf {
    /// Test entries (`"status":` inside the `tests` array).
    pub tests: usize,
    /// `"status": "passed"` entries.
    pub passed: usize,
    /// `"status": "failed"` entries.
    pub failed: usize,
    /// `"status": "skipped"` entries.
    pub skipped: usize,
    /// `"status": "pending"` entries.
    pub pending: usize,
    /// Entries with any other status (`tests - passed - failed - skipped - pending`).
    pub other: usize,
    /// `"flaky":` entries.
    pub flaky: usize,
    /// `"suite"` keys present.
    pub suites: usize,
    /// `"environment"` keys present.
    pub environments: usize,
    /// `"extra"` keys present.
    pub extras: usize,
}

/// Whether the buffer looks like a CTRF report.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"results\"") && t.contains("\"summary\"") && t.contains("\"tests\"")
}

impl Ctrf {
    /// Parse a CTRF report into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let tests = t.matches("\"status\":").count();
        let passed = t.matches("\"status\": \"passed\"").count();
        let failed = t.matches("\"status\": \"failed\"").count();
        let skipped = t.matches("\"status\": \"skipped\"").count();
        let pending = t.matches("\"status\": \"pending\"").count();
        Some(Self {
            tests,
            passed,
            failed,
            skipped,
            pending,
            other: tests.saturating_sub(passed + failed + skipped + pending),
            flaky: t.matches("\"flaky\":").count(),
            suites: t.matches("\"suite\"").count(),
            environments: t.matches("\"environment\"").count(),
            extras: t.matches("\"extra\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_report() {
        let b = br#"{
          "results": {
            "summary": { "tests": 4 },
            "tests": [
              { "name": "a", "status": "passed" },
              { "name": "b", "status": "failed", "flaky": true },
              { "name": "c", "status": "skipped" },
              { "name": "d", "status": "pending" }
            ],
            "suite": "s",
            "environment": { "os": "mac" },
            "extra": { "ci": true }
          }
        }"#;
        let c = Ctrf::parse(b).unwrap();
        assert_eq!(c.tests, 4);
        assert_eq!(c.passed, 1);
        assert_eq!(c.failed, 1);
        assert_eq!(c.skipped, 1);
        assert_eq!(c.pending, 1);
        assert_eq!(c.other, 0);
        assert_eq!(c.flaky, 1);
        assert_eq!(c.suites, 1);
        assert_eq!(c.environments, 1);
        assert_eq!(c.extras, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Ctrf::parse(b"{ \"tests\": [] }").is_none());
    }
}

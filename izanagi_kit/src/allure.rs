//! Allure result / container JSON (`*-result.json`, `*-container.json`) census.
//!
//! An Allure result object carries `uuid`, `name`, `status`
//! (`passed`/`failed`/`broken`/`skipped`), `stage`, `steps`, `attachments`,
//! `labels` and `historyId`; a container object carries `uuid`, `children`
//! plus `befores`/`afters` fixtures. `parse` counts the key occurrences and
//! each status value.
//!
//! ```rust
//! let a = br#"{
//!   "uuid": "u1",
//!   "name": "test",
//!   "status": "passed",
//!   "stage": "finished",
//!   "steps": [ { "name": "s", "status": "passed", "stage": "finished" } ],
//!   "attachments": [ { "name": "shot", "type": "image/png" } ],
//!   "labels": [ { "name": "suite", "value": "x" } ],
//!   "historyId": "h1"
//! }"#;
//! let c = izanagi_kit::allure::Allure::parse(a).unwrap();
//! assert_eq!(c.statuses, 2);
//! assert_eq!(c.passed, 2);
//! ```
//! A `container` file has no status but `children`/`befores`/`afters` blocks.

/// Allure JSON census counts.
#[derive(Debug, Clone)]
pub struct Allure {
    /// Total `"uuid"` occurrences (root object plus nested entries).
    pub uuids: usize,
    /// Total `"status":` entries including step statuses.
    pub statuses: usize,
    /// `"status": "passed"` entries.
    pub passed: usize,
    /// Non-passed status entries (`statuses - passed`).
    pub other_status: usize,
    /// `"steps":` array keys.
    pub steps: usize,
    /// `"attachments":` array keys.
    pub attachments: usize,
    /// `"labels":` array keys.
    pub labels: usize,
    /// `"historyId"` keys.
    pub history_ids: usize,
    /// `"children"`/`"befores"`/`"afters"` container keys.
    pub container_keys: usize,
}

/// Whether the buffer looks like an Allure result or container JSON.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"uuid\"")
        && (t.contains("\"historyId\"") || t.contains("\"children\"") || t.contains("\"steps\""))
}

impl Allure {
    /// Parse an Allure result or container JSON into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let statuses = t.matches("\"status\":").count();
        let passed = t.matches("\"status\": \"passed\"").count()
            + t.matches("\"status\":\"passed\"").count();
        Some(Self {
            uuids: t.matches("\"uuid\"").count(),
            statuses,
            passed,
            other_status: statuses.saturating_sub(passed),
            steps: t.matches("\"steps\":").count(),
            attachments: t.matches("\"attachments\":").count(),
            labels: t.matches("\"labels\":").count(),
            history_ids: t.matches("\"historyId\"").count(),
            container_keys: t.matches("\"children\"").count()
                + t.matches("\"befores\":").count()
                + t.matches("\"afters\":").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_result() {
        let b = br#"{
          "uuid": "u1", "name": "t", "status": "passed", "stage": "finished",
          "steps": [
            { "name": "a", "status": "passed", "stage": "finished" },
            { "name": "b", "status": "failed", "stage": "finished" }
          ],
          "attachments": [ {"name": "s.png"} ],
          "labels": [ {"name": "x", "value": "y"} ],
          "historyId": "h"
        }"#;
        let c = Allure::parse(b).unwrap();
        assert_eq!(c.statuses, 3);
        assert_eq!(c.passed, 2);
        assert_eq!(c.other_status, 1);
        assert_eq!(c.steps, 1);
        assert_eq!(c.container_keys, 0);
    }

    #[test]
    fn parses_container() {
        let b = br#"{ "uuid": "c1", "children": ["u1"], "befores": [], "afters": [] }"#;
        let c = Allure::parse(b).unwrap();
        assert_eq!(c.statuses, 0);
        assert_eq!(c.container_keys, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Allure::parse(b"{ \"id\": 1 }").is_none());
    }
}

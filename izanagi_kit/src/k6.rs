//! k6 JSON summary census.
//!
//! `k6 run --summary-export` writes `{ "root_group": { "name", "path", "id",
//! "groups": [...], "checks": [...] }, "metrics": { "<name>": { "type":
//! "counter|gauge|rate|trend", "contains": ..., "values" / "thresholds": ... }
//! } }`. `parse` counts metrics, distinct metric types, thresholds, groups
//! and checks.
//!
//! ```rust
//! let k = br#"{
//!   "root_group": { "name": "", "path": "", "id": "r",
//!     "groups": [ { "name": "g", "checks": [ {"name": "c"} ] } ],
//!     "checks": [ {"name": "ok"} ] },
//!   "metrics": {
//!     "http_reqs": { "type": "counter", "contains": "default", "values": {} },
//!     "http_req_duration": { "type": "trend", "contains": "time", "values": {},
//!       "thresholds": { "p(95)<500": {} } },
//!     "errors": { "type": "rate", "contains": "default", "values": {} }
//!   }
//! }"#;
//! let c = izanagi_kit::k6::K6::parse(k).unwrap();
//! assert_eq!(c.metrics, 3);
//! assert_eq!(c.metric_types, 3);
//! assert_eq!(c.thresholds, 1);
//! ```

/// k6 summary census counts.
#[derive(Debug, Clone)]
pub struct K6 {
    /// Metric entries (`"type":` inside `metrics`).
    pub metrics: usize,
    /// Distinct metric types (`counter`/`gauge`/`rate`/`trend`).
    pub metric_types: usize,
    /// `"thresholds":` keys.
    pub thresholds: usize,
    /// `"groups":` keys.
    pub groups: usize,
    /// `"checks":` keys.
    pub checks: usize,
    /// `"state":` / `"stages":` keys (options/state).
    pub states: usize,
}

/// Whether the buffer looks like a k6 JSON summary.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"metrics\"") && t.contains("\"root_group\"") && t.contains("\"type\"")
}

impl K6 {
    /// Parse a k6 JSON summary into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut types: Vec<&str> = Vec::new();
        let mut metrics = 0usize;
        let mut rest = t;
        while let Some(pos) = rest.find("\"type\":") {
            rest = &rest[pos + 7..];
            let v = rest.trim_start().trim_start_matches('"');
            let name = v.split('"').next().unwrap_or("");
            metrics += 1;
            if !types.contains(&name) {
                types.push(name);
            }
        }
        Some(Self {
            metrics,
            metric_types: types.len(),
            thresholds: t.matches("\"thresholds\":").count(),
            groups: t.matches("\"groups\":").count(),
            checks: t.matches("\"checks\":").count(),
            states: t.matches("\"state\":").count() + t.matches("\"stages\":").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_summary() {
        let b = br#"{
          "root_group": { "name": "", "groups": [ {"name": "g", "checks": []} ], "checks": [] },
          "metrics": {
            "vus": { "type": "gauge", "contains": "default" },
            "http_reqs": { "type": "counter", "contains": "default" },
            "http_req_duration": { "type": "trend", "contains": "time",
              "thresholds": { "p<500": {} } },
            "checks_rate": { "type": "rate", "contains": "default",
              "thresholds": { "rate>0": {} } }
          }
        }"#;
        let c = K6::parse(b).unwrap();
        assert_eq!(c.metrics, 4);
        assert_eq!(c.metric_types, 4);
        assert_eq!(c.thresholds, 2);
        assert_eq!(c.groups, 1);
        assert_eq!(c.checks, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(K6::parse(b"{ \"metrics\": {} }").is_none());
    }
}

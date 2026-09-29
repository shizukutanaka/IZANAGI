//! Criterion.rs result JSON parser (`estimates.json`, `sample.json`,
//! `benchmark.json`).
//!
//! Detects `"point_estimate"`/`"confidence_interval"` estimate blocks,
//! `"iters"`/`"times"` sample arrays or `"group_id"`/`"full_id"`
//! benchmark metadata, counting statistics keys, intervals and samples.
//!
//! ```
//! let b = concat!(
//!     "{\"mean\":{\"point_estimate\":10,\"standard_error\":1,",
//!     "\"confidence_interval\":{\"confidence_level\":9,",
//!     "\"lower_bound\":8,\"upper_bound\":12}},",
//!     "\"median\":{\"point_estimate\":9}}"
//! ).as_bytes();
//! assert!(izanagi_kit::criterion::detect(b));
//! let c = izanagi_kit::criterion::Criterion::parse(b).unwrap();
//! assert_eq!(c.statistics, 2);
//! assert_eq!(c.confidence_intervals, 1);
//! ```

/// Parsed Criterion.rs JSON document summary.
#[derive(Debug, Clone)]
pub struct Criterion {
    /// Statistic blocks (`mean`, `median`, `median_abs_dev`, `slope`,
    /// `std_dev`, `throughput`).
    pub statistics: usize,
    /// `"point_estimate"` values.
    pub point_estimates: usize,
    /// `"confidence_interval"` objects.
    pub confidence_intervals: usize,
    /// `"standard_error"` values.
    pub standard_errors: usize,
    /// `"iters"`/`"times"` sample arrays (sample.json form).
    pub samples: usize,
    /// `"group_id"`/`"full_id"`/`"function_id"` metadata fields
    /// (benchmark.json form).
    pub metadata: usize,
}

const STATS: &[&str] = &[
    "\"mean\"",
    "\"median\"",
    "\"median_abs_dev\"",
    "\"slope\"",
    "\"std_dev\"",
    "\"throughput\"",
];

const META: &[&str] = &[
    "\"group_id\"",
    "\"function_id\"",
    "\"full_id\"",
    "\"value_str\"",
];

fn count_key(t: &str, key: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        n += 1;
        off += i + key.len();
    }
    n
}

/// Whether the buffer looks like a Criterion.rs result document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if !t.trim_start().starts_with('{') {
        return false;
    }
    (t.contains("\"point_estimate\"") && t.contains("\"confidence_interval\""))
        || (t.contains("\"iters\"") && t.contains("\"times\""))
        || (t.contains("\"group_id\"") && t.contains("\"full_id\""))
}

impl Criterion {
    /// Parses a Criterion.rs JSON document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let statistics = STATS.iter().map(|k| count_key(t, k)).sum();
        let metadata = META.iter().map(|k| count_key(t, k)).sum();
        Some(Self {
            statistics,
            point_estimates: count_key(t, "\"point_estimate\""),
            confidence_intervals: count_key(t, "\"confidence_interval\""),
            standard_errors: count_key(t, "\"standard_error\""),
            samples: count_key(t, "\"iters\"") + count_key(t, "\"times\""),
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts_estimates() {
        let b = concat!(
            "{\"mean\":{\"point_estimate\":10,\"standard_error\":1,",
            "\"confidence_interval\":{\"confidence_level\":9,",
            "\"lower_bound\":8,\"upper_bound\":12}},",
            "\"median\":{\"point_estimate\":9,\"standard_error\":0,",
            "\"confidence_interval\":{\"confidence_level\":9,",
            "\"lower_bound\":9,\"upper_bound\":10}},",
            "\"median_abs_dev\":null,\"slope\":null,\"std_dev\":{\"point_estimate\":1}}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Criterion::parse(b).unwrap();
        assert_eq!(c.statistics, 5);
        assert_eq!(c.point_estimates, 3);
        assert_eq!(c.confidence_intervals, 2);
        assert_eq!(c.standard_errors, 2);
    }

    #[test]
    fn detects_sample_and_metadata() {
        let s = br#"{"iters":[1,2,3],"times":[10,11,12],"throughput":{"Bytes":3}}"#;
        assert!(detect(s));
        let c = Criterion::parse(s).unwrap();
        assert_eq!(c.samples, 2);
        let m = br#"{"group_id":"g","function_id":null,"value_str":"1","full_id":"g/f/1"}"#;
        assert!(detect(m));
        assert_eq!(Criterion::parse(m).unwrap().metadata, 4);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"mean\": 1}"));
        assert!(Criterion::parse(b"{}").is_none());
    }
}

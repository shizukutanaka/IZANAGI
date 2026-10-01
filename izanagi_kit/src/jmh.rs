//! JMH (Java Microbenchmark Harness) JSON result parser.
//!
//! Detects `[{"benchmark":…,"primaryMetric":{"scoreUnit":…}}]` documents
//! and counts benchmarks, distinct modes and score units, raw-data rows,
//! score confidence intervals and secondary metrics.
//!
//! ```
//! let b = concat!(
//!     "[{\"benchmark\":\"x.Bench.a\",\"mode\":\"avgt\",\"forks\":1,",
//!     "\"primaryMetric\":{\"score\":10,\"scoreUnit\":\"ns/op\",",
//!     "\"scoreCI\":[9,11],\"rawData\":[[1,2],[3,4]]},",
//!     "\"secondaryMetrics\":{}}]"
//! ).as_bytes();
//! assert!(izanagi_kit::jmh::detect(b));
//! let c = izanagi_kit::jmh::Jmh::parse(b).unwrap();
//! assert_eq!(c.benchmarks, 1);
//! assert_eq!(c.modes, 1);
//! ```

/// Parsed JMH JSON document summary.
#[derive(Debug, Clone)]
pub struct Jmh {
    /// Result entries (`"benchmark"` keys).
    pub benchmarks: usize,
    /// Distinct `"mode"` values (`avgt`, `thrpt`, `ss`, `sample`).
    pub modes: usize,
    /// Distinct `"scoreUnit"` values.
    pub units: usize,
    /// `"rawData"` raw-data arrays.
    pub raw_rows: usize,
    /// `"scoreCI"` intervals.
    pub score_cis: usize,
    /// `"secondaryMetrics"` objects.
    pub secondary_metrics: usize,
}

fn count_key(t: &str, key: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        n += 1;
        off += i + key.len();
    }
    n
}

fn str_val<'a>(seg: &'a str, key: &str) -> Option<&'a str> {
    let i = seg.find(key)?;
    let rest = &seg[i + key.len()..];
    let c1 = rest.find('"')?;
    let c2 = rest[c1 + 1..].find('"')? + c1 + 1;
    Some(&rest[c1 + 1..c2])
}

fn collect_vals(t: &str, key: &str, out: &mut Vec<String>) {
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        if let Some(v) = str_val(&t[off + i..], key) {
            if !out.iter().any(|u| u == v) {
                out.push(v.to_string());
            }
        }
        off += i + key.len();
    }
}

/// Whether the buffer looks like a JMH JSON result document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let tr = t.trim_start();
    if !tr.starts_with('[') && !tr.starts_with('{') {
        return false;
    }
    t.contains("\"primaryMetric\"") && (t.contains("\"benchmark\"") || t.contains("\"scoreUnit\""))
}

impl Jmh {
    /// Parses a JMH JSON document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut modes = Vec::new();
        let mut units = Vec::new();
        collect_vals(t, "\"mode\"", &mut modes);
        collect_vals(t, "\"scoreUnit\"", &mut units);
        Some(Self {
            benchmarks: count_key(t, "\"benchmark\""),
            modes: modes.len(),
            units: units.len(),
            raw_rows: count_key(t, "\"rawData\""),
            score_cis: count_key(t, "\"scoreCI\""),
            secondary_metrics: count_key(t, "\"secondaryMetrics\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "[{\"benchmark\":\"x.B.a\",\"mode\":\"avgt\",\"forks\":1,",
            "\"primaryMetric\":{\"score\":10,\"scoreUnit\":\"ns/op\",",
            "\"scoreCI\":[9,11],\"rawData\":[[1,2],[3,4]]},",
            "\"secondaryMetrics\":{}},",
            "{\"benchmark\":\"x.B.b\",\"mode\":\"thrpt\",\"forks\":2,",
            "\"primaryMetric\":{\"score\":5,\"scoreUnit\":\"ops/ms\",",
            "\"scoreCI\":[4,6],\"rawData\":[[1,2],[3,4]]},",
            "\"secondaryMetrics\":{\"mem\":{\"score\":1}}}]"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Jmh::parse(b).unwrap();
        assert_eq!(c.benchmarks, 2);
        assert_eq!(c.modes, 2);
        assert_eq!(c.units, 2);
        assert_eq!(c.raw_rows, 2);
        assert_eq!(c.score_cis, 2);
        assert_eq!(c.secondary_metrics, 2);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"benchmark\": \"x\"}"));
        assert!(!detect(b"[1]"));
        assert!(Jmh::parse(b"{}").is_none());
    }
}

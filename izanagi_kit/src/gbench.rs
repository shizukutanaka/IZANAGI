//! Google Benchmark JSON result parser (`benchmark/BM_…` output).
//!
//! Detects `{"context":{…},"benchmarks":[{…}]}` documents and counts
//! benchmark entries, iteration vs aggregate `run_type` values, distinct
//! `time_unit` values, `family_index` entries and per-second counters.
//!
//! ```
//! let b = concat!(
//!     "{\"context\":{\"num_cpus\":8},",
//!     "\"benchmarks\":[",
//!     "{\"name\":\"BM_a\",\"run_type\":\"iteration\",\"iterations\":100,",
//!     "\"real_time\":50,\"cpu_time\":40,\"time_unit\":\"ns\"},",
//!     "{\"name\":\"BM_b\",\"run_type\":\"aggregate\",\"time_unit\":\"us\",",
//!     "\"bytes_per_second\":1024}]}").as_bytes();
//! assert!(izanagi_kit::gbench::detect(b));
//! let c = izanagi_kit::gbench::Gbench::parse(b).unwrap();
//! assert_eq!(c.benchmarks, 2);
//! assert_eq!(c.iteration_runs, 1);
//! ```

/// Parsed Google Benchmark document summary.
#[derive(Debug, Clone)]
pub struct Gbench {
    /// Total benchmark entries (`"name"` in `benchmarks`).
    pub benchmarks: usize,
    /// `"run_type":"iteration"` entries.
    pub iteration_runs: usize,
    /// `"run_type":"aggregate"` entries.
    pub aggregate_runs: usize,
    /// `"time_unit"` values present.
    pub time_units: usize,
    /// `"family_index"` entries.
    pub families: usize,
    /// `*_per_second` counter keys.
    pub counters: usize,
    /// Sum of `"iterations"` values.
    pub iterations: u64,
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

fn int_after(t: &str, key: &str) -> Option<u64> {
    let i = t.find(key)?;
    let rest = t[i + key.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let digits: usize = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    rest[..digits].parse().ok()
}

/// Whether the buffer looks like a Google Benchmark JSON document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"benchmarks\"")
        && (t.contains("\"run_type\"") || t.contains("\"real_time\"") || t.contains("\"cpu_time\""))
}

impl Gbench {
    /// Parses a Google Benchmark document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            benchmarks: 0,
            iteration_runs: 0,
            aggregate_runs: 0,
            time_units: 0,
            families: 0,
            counters: 0,
            iterations: 0,
        };
        c.benchmarks = count_key(t, "\"run_type\"").max(count_key(t, "\"name\""));
        c.iteration_runs = count_key(t, "\"iteration\"");
        c.aggregate_runs = count_key(t, "\"aggregate\"");
        c.time_units = count_key(t, "\"time_unit\"");
        c.families = count_key(t, "\"family_index\"");
        c.counters = count_key(t, "_per_second\"");
        let mut off = 0;
        while let Some(i) = t[off..].find("\"iterations\"") {
            if let Some(v) = int_after(&t[off + i..], "\"iterations\"") {
                c.iterations += v;
            }
            off += i + 12;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"context\":{\"num_cpus\":8,\"mhz_per_cpu\":2400},",
            "\"benchmarks\":[",
            "{\"name\":\"BM_a\",\"family_index\":0,\"run_type\":\"iteration\",\"iterations\":100,",
            "\"real_time\":50,\"cpu_time\":40,\"time_unit\":\"ns\"},",
            "{\"name\":\"BM_b\",\"run_type\":\"iteration\",\"iterations\":25,",
            "\"real_time\":9,\"cpu_time\":9,\"time_unit\":\"us\",\"bytes_per_second\":1024},",
            "{\"name\":\"BM_a_mean\",\"run_type\":\"aggregate\",\"time_unit\":\"ns\"}]}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Gbench::parse(b).unwrap();
        assert_eq!(c.benchmarks, 3);
        assert_eq!(c.iteration_runs, 2);
        assert_eq!(c.aggregate_runs, 1);
        assert_eq!(c.time_units, 3);
        assert_eq!(c.families, 1);
        assert_eq!(c.counters, 1);
        assert_eq!(c.iterations, 125);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"benchmarks\": []}"));
        assert!(!detect(b"[{\"run_type\":\"iteration\"}]"));
        assert!(Gbench::parse(b"{}").is_none());
    }
}

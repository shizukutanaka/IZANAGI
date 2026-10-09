//! pytest-benchmark JSON result parser (`--benchmark-json` output).
//!
//! Detects `{"machine_info":…,"benchmarks":[{…,"stats":{…}}]}` documents
//! and counts benchmarks, distinct groups, `stats` keys and totals for
//! `rounds`, `iterations` and `ops`.
//!
//! ```
//! let b = concat!(
//!     "{\"machine_info\":{\"cpu\":{}},\"commit_info\":{\"id\":\"x\"},",
//!     "\"benchmarks\":[{\"group\":\"g\",\"name\":\"test_a\",",
//!     "\"stats\":{\"min\":1,\"max\":2,\"mean\":1,\"stddev\":0,",
//!     "\"median\":1,\"ops\":100,\"rounds\":10,\"iterations\":5}}]}"
//! ).as_bytes();
//! assert!(izanagi_kit::pytestbench::detect(b));
//! let c = izanagi_kit::pytestbench::Pytestbench::parse(b).unwrap();
//! assert_eq!(c.benchmarks, 1);
//! assert_eq!(c.rounds_total, 10);
//! ```

use crate::textutil::strip_bom;
/// Parsed pytest-benchmark JSON document summary.
#[derive(Debug, Clone)]
pub struct Pytestbench {
    /// Benchmark entries (`"stats"` objects).
    pub benchmarks: usize,
    /// Distinct `"group"` values.
    pub groups: usize,
    /// Total `stats` numeric keys counted.
    pub stats_keys: usize,
    /// Sum of `"rounds"` values.
    pub rounds_total: u64,
    /// Sum of `"iterations"` values.
    pub iterations_total: u64,
    /// Sum of `"ops"` values.
    pub ops_total: u64,
}

const STATS_KEYS: &[&str] = &[
    "\"min\"",
    "\"max\"",
    "\"mean\"",
    "\"stddev\"",
    "\"median\"",
    "\"iqr\"",
    "\"q1\"",
    "\"q3\"",
    "\"ops\"",
    "\"total\"",
    "\"rounds\"",
    "\"iterations\"",
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

fn sum_ints(t: &str, key: &str) -> u64 {
    let mut sum = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        let rest = t[off + i + key.len()..].trim_start();
        let rest = rest.strip_prefix(':').map(str::trim_start).unwrap_or("");
        let digits: usize = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 {
            if let Ok(v) = rest[..digits].parse::<u64>() {
                sum += v;
            }
        }
        off += i + key.len();
    }
    sum
}

/// Whether the buffer looks like a pytest-benchmark JSON document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"benchmarks\"")
        && t.contains("\"stats\"")
        && (t.contains("\"machine_info\"") || t.contains("\"ops\"") || t.contains("\"rounds\""))
}

impl Pytestbench {
    /// Parses a pytest-benchmark JSON document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut groups: Vec<String> = Vec::new();
        let mut off = 0;
        while let Some(i) = t[off..].find("\"group\"") {
            let seg = &t[off + i + 7..];
            if let Some(c1) = seg.find('"') {
                if let Some(c2) = seg[c1 + 1..].find('"') {
                    let g = &seg[c1 + 1..c1 + 1 + c2];
                    if !g.is_empty() && !groups.iter().any(|x| x == g) {
                        groups.push(g.to_string());
                    }
                }
            }
            off += i + 7;
        }
        let stats_keys = STATS_KEYS.iter().map(|k| count_key(t, k)).sum();
        Some(Self {
            benchmarks: count_key(t, "\"stats\""),
            groups: groups.len(),
            stats_keys,
            rounds_total: sum_ints(t, "\"rounds\""),
            iterations_total: sum_ints(t, "\"iterations\""),
            ops_total: sum_ints(t, "\"ops\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"machine_info\":{\"cpu\":{}},\"commit_info\":{\"id\":\"x\"},",
            "\"benchmarks\":[",
            "{\"group\":\"g1\",\"name\":\"test_a\",\"fullname\":\"f::a\",",
            "\"stats\":{\"min\":1,\"max\":2,\"mean\":1,\"stddev\":0,",
            "\"median\":1,\"iqr\":0,\"q1\":1,\"q3\":1,\"ops\":100,",
            "\"total\":10,\"rounds\":10,\"iterations\":5}},",
            "{\"group\":\"g2\",\"name\":\"test_b\",",
            "\"stats\":{\"min\":2,\"max\":3,\"mean\":2,\"stddev\":0,",
            "\"median\":2,\"iqr\":0,\"q1\":2,\"q3\":2,\"ops\":50,",
            "\"total\":6,\"rounds\":5,\"iterations\":3}}]}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Pytestbench::parse(b).unwrap();
        assert_eq!(c.benchmarks, 2);
        assert_eq!(c.groups, 2);
        assert_eq!(c.stats_keys, 24);
        assert_eq!(c.rounds_total, 15);
        assert_eq!(c.iterations_total, 8);
        assert_eq!(c.ops_total, 150);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"benchmarks\": []}"));
        assert!(!detect(b"{\"stats\": {}}"));
        assert!(Pytestbench::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

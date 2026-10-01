//! Go `benchstat`/benchfmt benchmark text parser (`BenchmarkName-N …`).
//!
//! Detects `key: value` metadata plus `BenchmarkFoo-8  N  v1 unit  v2
//! unit` result lines and counts records, distinct benchmark names,
//! distinct unit columns and metadata keys.
//!
//! ```
//! let b = concat!(
//!     "goos: linux\n",
//!     "BenchmarkFoo-8  100  50 ns/op  64 B/op  1 allocs/op\n",
//!     "BenchmarkBar-8   50  90 ns/op  64 B/op  1 allocs/op\n"
//! ).as_bytes();
//! assert!(izanagi_kit::benchstat::detect(b));
//! let c = izanagi_kit::benchstat::Benchstat::parse(b).unwrap();
//! assert_eq!(c.records, 2);
//! assert_eq!(c.units, 3);
//! ```

/// Parsed benchstat/benchfmt document summary.
#[derive(Debug, Clone)]
pub struct Benchstat {
    /// `Benchmark*` result lines.
    pub records: usize,
    /// Distinct benchmark names (before `-N` suffix).
    pub names: usize,
    /// Distinct unit columns (`ns/op`, `B/op`, `allocs/op`, …).
    pub units: usize,
    /// `key: value` metadata lines.
    pub metadata: usize,
    /// Sum of repetition counts (`BenchmarkFoo-8` `N` column).
    pub repetitions: u64,
}

fn is_record(l: &str) -> Option<(&str, u64)> {
    if !l.starts_with("Benchmark") {
        return None;
    }
    let mut w = l.split_whitespace();
    let name = w.next()?;
    let dash = name.rfind('-')?;
    if !name[dash + 1..].bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let reps: u64 = w.next()?.parse().ok()?;
    Some((&name[..dash], reps))
}

/// Whether the buffer looks like benchstat/benchfmt text output.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().filter(|l| is_record(l.trim()).is_some()).count() >= 1
}

impl Benchstat {
    /// Parses a benchstat/benchfmt document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            records: 0,
            names: 0,
            units: 0,
            metadata: 0,
            repetitions: 0,
        };
        let mut names: Vec<String> = Vec::new();
        let mut units: Vec<String> = Vec::new();
        for l in t.lines() {
            let tr = l.trim();
            if let Some((name, reps)) = is_record(tr) {
                c.records += 1;
                c.repetitions += reps;
                if !names.iter().any(|n| n == name) {
                    names.push(name.to_string());
                }
                let w = tr.split_whitespace().skip(2);
                for tok in w {
                    let is_num = tok.bytes().all(|c| c.is_ascii_digit() || c == b'.')
                        && tok.bytes().any(|c| c.is_ascii_digit());
                    if !is_num && !units.iter().any(|u| u == tok) {
                        units.push(tok.to_string());
                    }
                }
            } else if tr.contains(": ") && !tr.is_empty() {
                c.metadata += 1;
            }
        }
        c.names = names.len();
        c.units = units.len();
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "goos: linux\n",
            "goarch: amd64\n",
            "pkg: example\n",
            "BenchmarkFoo-8   100   50 ns/op   64 B/op   1 allocs/op\n",
            "BenchmarkFoo-4   200   55 ns/op   64 B/op   1 allocs/op\n",
            "BenchmarkBar-8    50   90 ns/op  128 B/op   2 allocs/op\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Benchstat::parse(b).unwrap();
        assert_eq!(c.records, 3);
        assert_eq!(c.names, 2);
        assert_eq!(c.units, 3);
        assert_eq!(c.metadata, 3);
        assert_eq!(c.repetitions, 350);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"BenchmarkFoo\n"));
        assert!(!detect(b"key: value\n"));
        assert!(Benchstat::parse(b"nope").is_none());
    }
}

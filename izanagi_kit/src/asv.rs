//! airspeed velocity (asv) benchmark result JSON parser.
//!
//! Detects `{"results":{…,"benchmark_version":…,"started_at":…}}`
//! documents and counts benchmark entries, machine/params keys, result
//! rows and profile attachments.
//!
//! ```
//! let b = concat!(
//!     "{\"results\":{\"machine\":\"m\",\"python\":\"3\",",
//!     "\"started_at\":100,\"duration\":50,\"benchmark_version\":2,",
//!     "\"bench.a\":{\"code\":\"f()\",\"params\":[[1]],",
//!     "\"param_names\":[\"n\"],\"result\":[{\"time\":7}]}}}"
//! ).as_bytes();
//! assert!(izanagi_kit::asv::detect(b));
//! let c = izanagi_kit::asv::Asv::parse(b).unwrap();
//! assert_eq!(c.benchmarks, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed asv result document summary.
#[derive(Debug, Clone)]
pub struct Asv {
    /// `"result"` benchmark entries.
    pub benchmarks: usize,
    /// `"code"` snippet fields.
    pub codes: usize,
    /// `"param_names"` parameter name arrays.
    pub param_names: usize,
    /// `"result"` entries containing a `"time"` value.
    pub timed: usize,
    /// `"profile"` attachment fields.
    pub profiles: usize,
    /// `"started_at"`/`"duration"`/`"ended_at"` timestamp fields.
    pub timestamps: usize,
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

/// Whether the buffer looks like an asv result document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    t.contains("\"results\"")
        && (t.contains("\"benchmark_version\"")
            || (t.contains("\"started_at\"") && t.contains("\"machine\"")))
}

impl Asv {
    /// Parses an asv result document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        Some(Self {
            benchmarks: count_key(t, "\"result\""),
            codes: count_key(t, "\"code\""),
            param_names: count_key(t, "\"param_names\""),
            timed: count_key(t, "\"time\""),
            profiles: count_key(t, "\"profile\""),
            timestamps: count_key(t, "\"started_at\"")
                + count_key(t, "\"ended_at\"")
                + count_key(t, "\"duration\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "{\"results\":{\"machine\":\"m\",\"python\":\"3\",",
            "\"started_at\":100,\"ended_at\":200,\"duration\":100,",
            "\"benchmark_version\":2,",
            "\"bench.a\":{\"code\":\"f()\",\"name\":\"a\",\"params\":[[1]],",
            "\"param_names\":[\"n\"],\"result\":[{\"time\":7,\"number\":1,",
            "\"repeat\":1,\"profile\":\"p\"}]},",
            "\"bench.b\":{\"code\":\"g()\",\"param_names\":[],",
            "\"result\":[{\"time\":9}]}}}"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Asv::parse(b).unwrap();
        assert_eq!(c.benchmarks, 2);
        assert_eq!(c.timed, 2);
        assert_eq!(c.codes, 2);
        assert_eq!(c.param_names, 2);
        assert_eq!(c.profiles, 1);
        assert_eq!(c.timestamps, 3);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"results\": {}}"));
        assert!(!detect(b"[1]"));
        assert!(Asv::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

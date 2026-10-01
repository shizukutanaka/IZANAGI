//! Chrome Trace Event Format parser (`chrome://tracing`, `about:tracing`,
//! `chrome trace` JSON).
//!
//! Detects `{"traceEvents":[ … ]}` objects and counts trace events by
//! their `"ph"` phase letter (duration `B`/`E`, complete `X`, instant
//! `i`/`I`, counter `C`, async `b`/`n`/`e`, flow `s`/`t`/`f`, metadata
//! `M`, sample `P`, object `N`/`O`/`D`), plus `"pid"`/`"tid"` carriers
//! and `"cat"` categories.
//!
//! ```
//! let b = br#"{"traceEvents":[
//! {"name":"main","cat":"work","ph":"B","ts":0,"pid":1,"tid":1},
//! {"name":"main","cat":"work","ph":"E","ts":9,"pid":1,"tid":1},
//! {"name":"tick","cat":"ctr","ph":"C","ts":4,"pid":1,"tid":2}
//! ]}"#;
//! assert!(izanagi_kit::chrometrace::detect(b));
//! let c = izanagi_kit::chrometrace::Chrometrace::parse(b).unwrap();
//! assert_eq!(c.events, 3);
//! assert_eq!(c.duration_events, 2);
//! ```

/// Parsed Chrome Trace Event summary.
#[derive(Debug, Clone)]
pub struct Chrometrace {
    /// Total events carrying a `"ph"` phase letter.
    pub events: usize,
    /// Distinct phase letters seen (`B`,`E`,`X`,`C`,`M`,`P`,…).
    pub phases: usize,
    /// Duration events (`B`/`E`/`X` complete).
    pub duration_events: usize,
    /// Instant/metadata events (`i`/`I`/`M`).
    pub instant_events: usize,
    /// Counter events (`C`).
    pub counter_events: usize,
    /// `"pid":` fields seen.
    pub pids: usize,
    /// `"cat":` categories seen.
    pub cats: usize,
    /// `"displayTimeUnit"` present.
    pub display_time_unit: bool,
}

/// Extracts the phase letter after `"ph": "X"` in one JSON segment.
fn phase_of(seg: &str) -> Option<u8> {
    let i = seg.find("\"ph\"")?;
    let rest = &seg[i + 4..];
    let c1 = rest.find('"')?;
    let c2 = rest[c1 + 1..].find('"')? + c1 + 1;
    let inner = &rest[c1 + 1..c2];
    (inner.len() == 1).then(|| inner.as_bytes()[0])
}

fn count_key(seg: &str, key: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = seg[off..].find(key) {
        n += 1;
        off += i + key.len();
    }
    n
}

/// Whether the buffer looks like a Chrome Trace Event document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let tr = t.trim_start();
    if !tr.starts_with('{') && !tr.starts_with('[') {
        return false;
    }
    if t.contains("\"traceEvents\"") {
        return true;
    }
    // Array-only form is legal too: `[{"ph":"B","ts":…}, …]`.
    let mut ev = 0;
    for seg in t.split('{') {
        if phase_of(seg).is_some() && seg.contains("\"ts\"") {
            ev += 1;
        }
    }
    tr.starts_with('[') && ev >= 2 || ev >= 3
}

impl Chrometrace {
    /// Parses a Chrome Trace Event summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            events: 0,
            phases: 0,
            duration_events: 0,
            instant_events: 0,
            counter_events: 0,
            pids: 0,
            cats: 0,
            display_time_unit: t.contains("\"displayTimeUnit\""),
        };
        let mut seen = 0u64;
        for seg in t.split('{') {
            let Some(ph) = phase_of(seg) else { continue };
            c.events += 1;
            let bit = if ph.is_ascii_alphabetic() {
                1u64 << (ph - b'A')
            } else {
                0
            };
            if bit != 0 && seen & bit == 0 {
                seen |= bit;
                c.phases += 1;
            }
            match ph {
                b'B' | b'E' | b'X' => c.duration_events += 1,
                b'i' | b'I' | b'M' => c.instant_events += 1,
                b'C' => c.counter_events += 1,
                _ => {}
            }
            c.pids += count_key(seg, "\"pid\"");
            c.cats += count_key(seg, "\"cat\"");
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"{
  "traceEvents": [
    {"name":"main","cat":"work","ph":"B","ts":0,"pid":1,"tid":1,"args":{}},
    {"name":"io","cat":"work","ph":"X","ts":2,"dur":5,"pid":1,"tid":1},
    {"name":"mark","cat":"ui","ph":"i","ts":7,"pid":1,"tid":1,"s":"g"},
    {"name":"tick","cat":"ctr","ph":"C","ts":4,"pid":1,"tid":2,"args":{"v":3}},
    {"name":"meta","ph":"M","ts":0,"pid":1,"tid":1},
    {"name":"flow","cat":"net","ph":"s","ts":1,"pid":2,"tid":4,"id":9}
  ],
  "displayTimeUnit": "ms"
}"#;
        assert!(detect(b));
        let c = Chrometrace::parse(b).unwrap();
        assert_eq!(c.events, 6);
        assert_eq!(c.phases, 6);
        assert_eq!(c.duration_events, 2);
        assert_eq!(c.instant_events, 2);
        assert_eq!(c.counter_events, 1);
        assert_eq!(c.pids, 6);
        assert_eq!(c.cats, 5);
        assert!(c.display_time_unit);
    }

    #[test]
    fn detects_array_form() {
        let b =
            br#"[{"ph":"B","ts":0,"pid":1},{"ph":"E","ts":1,"pid":1},{"ph":"X","ts":2,"pid":2}]"#;
        assert!(detect(b));
        let c = Chrometrace::parse(b).unwrap();
        assert_eq!(c.events, 3);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"key\": \"value\"}"));
        assert!(!detect(b"[1, 2, 3]"));
        assert!(Chrometrace::parse(b"{}").is_none());
    }
}

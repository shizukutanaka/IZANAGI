//! Hyperfine benchmark JSON result parser (`hyperfine --export-json`).
//!
//! Detects `[{"command":…,"mean":…,"times":[…]}]` result documents and
//! counts results, timing samples, exit codes, parameter sets and warmup
//! fields.
//!
//! ```
//! let b = concat!(
//!     "[{\"command\":\"sleep 0\",\"mean\":10,\"stddev\":1,\"median\":9,",
//!     "\"min\":8,\"max\":12,\"times\":[8,9,10,11,12],",
//!     "\"exit_codes\":[0,0,0,0,0],\"parameters\":{\"n\":1}}]"
//! ).as_bytes();
//! assert!(izanagi_kit::hyperfine::detect(b));
//! let c = izanagi_kit::hyperfine::Hyperfine::parse(b).unwrap();
//! assert_eq!(c.results, 1);
//! assert_eq!(c.times, 5);
//! ```

/// Parsed hyperfine JSON document summary.
#[derive(Debug, Clone)]
pub struct Hyperfine {
    /// Result objects (`"command"` keys).
    pub results: usize,
    /// Total timing samples across `"times"` arrays.
    pub times: usize,
    /// `"exit_codes"` array entries.
    pub exit_codes: usize,
    /// Zero exit codes.
    pub exits_zero: usize,
    /// Non-zero exit codes.
    pub exits_nonzero: usize,
    /// `"parameters"` objects.
    pub parameters: usize,
    /// `"warmup"` fields.
    pub warmups: usize,
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

fn array_ints(after_key: &str) -> usize {
    let Some(open) = after_key.find('[') else {
        return 0;
    };
    let mut depth = 0usize;
    let mut end = after_key.len();
    for (i, ch) in after_key[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    let inner = &after_key[open + 1..end];
    inner
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .filter(|s| {
            s.trim()
                .bytes()
                .all(|c| c.is_ascii_digit() || c == b'.' || c == b'-')
        })
        .count()
}

fn sum_arrays(t: &str, key: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(key) {
        n += array_ints(&t[off + i + key.len()..]);
        off += i + key.len();
    }
    n
}

fn count_exit_codes(t: &str) -> (usize, usize) {
    let (mut z, mut nz) = (0usize, 0usize);
    let mut off = 0;
    while let Some(i) = t[off..].find("\"exit_codes\"") {
        let seg = &t[off + i..];
        if let Some(open) = seg.find('[') {
            if let Some(close) = seg[open..].find(']') {
                for v in seg[open + 1..open + close].split(',') {
                    match v.trim() {
                        "0" => z += 1,
                        s if !s.is_empty() => nz += 1,
                        _ => {}
                    }
                }
            }
        }
        off += i + 12;
    }
    (z, nz)
}

/// Whether the buffer looks like a hyperfine JSON result document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let tr = t.trim_start();
    if !tr.starts_with('[') && !tr.starts_with('{') {
        return false;
    }
    t.contains("\"command\"") && t.contains("\"times\"")
        || (t.contains("\"command\"") && t.contains("\"mean\"") && t.contains("\"stddev\""))
}

impl Hyperfine {
    /// Parses a hyperfine JSON document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let (z, nz) = count_exit_codes(t);
        Some(Self {
            results: count_key(t, "\"command\""),
            times: sum_arrays(t, "\"times\""),
            exit_codes: z + nz,
            exits_zero: z,
            exits_nonzero: nz,
            parameters: count_key(t, "\"parameters\""),
            warmups: count_key(t, "\"warmup\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = concat!(
            "[{\"command\":\"a\",\"mean\":10,\"stddev\":1,\"median\":9,",
            "\"user\":5,\"system\":4,\"min\":8,\"max\":12,",
            "\"times\":[8,9,10,11,12],\"exit_codes\":[0,0,0,0,0],",
            "\"parameters\":{\"n\":1}},",
            "{\"command\":\"b\",\"mean\":3,\"stddev\":0,\"median\":3,",
            "\"min\":3,\"max\":4,\"times\":[3,3,4],",
            "\"exit_codes\":[0,1,0],\"warmup\":2}]"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Hyperfine::parse(b).unwrap();
        assert_eq!(c.results, 2);
        assert_eq!(c.times, 8);
        assert_eq!(c.exit_codes, 8);
        assert_eq!(c.exits_zero, 7);
        assert_eq!(c.exits_nonzero, 1);
        assert_eq!(c.parameters, 1);
        assert_eq!(c.warmups, 1);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"command\": \"x\"}"));
        assert!(!detect(b"[1,2,3]"));
        assert!(Hyperfine::parse(b"{}").is_none());
    }
}

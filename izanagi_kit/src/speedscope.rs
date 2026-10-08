//! Speedscope profile document parser (`.speedscope.json`, `imported from
//! …` JSON).
//!
//! Detects `{"shared":{"frames":[…]},"profiles":[…]}` documents and
//! counts profile entries by `"type":"sampled"|"evented"|"list"`, frame
//! `"name"` entries, sample-group / weight arrays, and unit fields.
//!
//! ```
//! let b = br#"{"$schema":"https://www.speedscope.app/file-format-schema.json",
//! "shared":{"frames":[{"name":"a"},{"name":"b"}]},
//! "profiles":[{"type":"sampled","name":"p","unit":"microseconds",
//! "startValue":0,"endValue":10,"samples":[[0,1],[1]],"weights":[5,5]}],
//! "exporter":"x"}"#;
//! assert!(izanagi_kit::speedscope::detect(b));
//! let c = izanagi_kit::speedscope::Speedscope::parse(b).unwrap();
//! assert_eq!(c.profiles, 1);
//! assert_eq!(c.frames, 2);
//! ```

/// Parsed speedscope document summary.
#[derive(Debug, Clone)]
pub struct Speedscope {
    /// Total profile entries (`"type":"…"` in `profiles`).
    pub profiles: usize,
    /// `sampled` profile entries.
    pub sampled: usize,
    /// `evented` profile entries.
    pub evented: usize,
    /// `list` / other profile entries.
    pub listed: usize,
    /// Frame `"name"` entries under `shared.frames`.
    pub frames: usize,
    /// `"samples":[` groups.
    pub sample_groups: usize,
    /// `"weights":[` arrays.
    pub weights: usize,
    /// `"unit":"…"` fields.
    pub units: usize,
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

fn type_of(seg: &str) -> Option<&str> {
    let i = seg.find("\"type\"")?;
    let rest = &seg[i + 6..];
    let c1 = rest.find('"')?;
    let c2 = rest[c1 + 1..].find('"')? + c1 + 1;
    Some(&rest[c1 + 1..c2])
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer looks like a speedscope document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    if !t.trim_start().starts_with('{') {
        return false;
    }
    if t.contains("speedscope") {
        return true;
    }
    t.contains("\"profiles\"")
        && t.contains("\"shared\"")
        && t.contains("\"frames\"")
        && (t.contains("\"sampled\"") || t.contains("\"evented\"") || t.contains("\"list\""))
}

impl Speedscope {
    /// Parses a speedscope document summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            profiles: 0,
            sampled: 0,
            evented: 0,
            listed: 0,
            frames: 0,
            sample_groups: 0,
            weights: 0,
            units: 0,
        };
        for seg in t.split('{') {
            if let Some(ty) = type_of(seg) {
                match ty {
                    "sampled" | "evented" | "list" => {
                        c.profiles += 1;
                        if ty == "sampled" {
                            c.sampled += 1;
                        } else if ty == "evented" {
                            c.evented += 1;
                        } else {
                            c.listed += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(fs) = t.find("\"frames\"") {
            let tail = &t[fs..];
            if let Some(open) = tail.find('[') {
                let mut depth = 0usize;
                let mut end = tail.len();
                for (i, ch) in tail[open..].char_indices() {
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
                c.frames = count_key(&tail[open..end], "\"name\"");
            }
        }
        c.sample_groups = count_key(t, "\"samples\"");
        c.weights = count_key(t, "\"weights\"");
        c.units = count_key(t, "\"unit\"");
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"{
  "$schema": "https://www.speedscope.app/file-format-schema.json",
  "shared": {"frames": [{"name":"a","file":"a.c"},{"name":"b"},{"name":"c"}]},
  "profiles": [
    {"type":"sampled","name":"p","unit":"microseconds","startValue":0,"endValue":9,
     "samples":[[0,1],[1,2],[2]],"weights":[3,3,3]},
    {"type":"evented","name":"q","unit":"none",
     "events":[{"at":0,"frame":0,"type":"O"},{"at":1,"frame":0,"type":"C"}]},
    {"type":"list","name":"r","unit":"bytes","lines":[[0,1]]}
  ],
  "exporter": "x"
}"#;
        assert!(detect(b));
        let c = Speedscope::parse(b).unwrap();
        assert_eq!(c.profiles, 3);
        assert_eq!(c.sampled, 1);
        assert_eq!(c.evented, 1);
        assert_eq!(c.listed, 1);
        assert_eq!(c.frames, 3);
        assert_eq!(c.sample_groups, 1);
        assert_eq!(c.weights, 1);
        assert_eq!(c.units, 3);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"frames\": []}"));
        assert!(!detect(b"{\"traceEvents\": []}"));
        assert!(Speedscope::parse(b"{}").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

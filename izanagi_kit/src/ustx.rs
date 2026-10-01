//! OpenUtau project (`*.ustx`) YAML census.
//!
//! Tracks `ustx_version`, tempo/meter maps, `tracks`/`parts`/`notes`
//! list items plus per-note `tone:`/`lyric:`/`vibrato:` entries.
//!
//! ```
//! let s = b"name: Demo\nustx_version: 6\nbpm: 120\nbeat_per_bar: 4\nbeat_unit: 4\nresolution: 480\ntempos:\n  - position: 0\n    bpm: 120\ntracks:\n  - track_name: Voice\n    singer: defoko\n    phonemizer: ja\n    parts:\n      - name: P1\n        position: 0\n        notes:\n          - position: 0\n            duration: 240\n            tone: 60\n            lyric: a\n            vibrato: {}\n";
//! assert!(izanagi_kit::ustx::detect(s));
//! let x = izanagi_kit::ustx::Ustx::parse(s).unwrap();
//! assert_eq!(x.bpm, 120);
//! assert_eq!(x.tracks, 1);
//! assert_eq!(x.parts, 1);
//! assert_eq!(x.notes, 1);
//! assert_eq!(x.tones, 1);
//! ```

/// Parsed census of an OpenUtau `*.ustx` project.
#[derive(Debug, Clone)]
pub struct Ustx {
    /// `ustx_version:` value, or 0.
    pub version: usize,
    /// Top-level `bpm:` value, or 0.
    pub bpm: usize,
    /// `beat_per_bar:` value, or 0.
    pub beat_per_bar: usize,
    /// `beat_unit:` value, or 0.
    pub beat_unit: usize,
    /// `resolution:` value, or 0.
    pub resolution: usize,
    /// Items in `expressions:`.
    pub expressions: usize,
    /// Items in `curves:`.
    pub curves: usize,
    /// Items in `tempos:`.
    pub tempos: usize,
    /// Items in `time_signatures:`.
    pub time_signatures: usize,
    /// Items in `tracks:`.
    pub tracks: usize,
    /// Items in `parts:` blocks.
    pub parts: usize,
    /// Items in `notes:` blocks.
    pub notes: usize,
    /// `singer:` assignments.
    pub singers: usize,
    /// `phonemizer:` assignments.
    pub phonemizers: usize,
    /// `tone:` assignments.
    pub tones: usize,
    /// `lyric:` assignments.
    pub lyrics: usize,
    /// `vibrato:` keys.
    pub vibrato: usize,
    /// `pitch:` keys.
    pub pitch: usize,
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

fn child_lines<'a>(t: &'a str, key: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut grab = false;
    let mut ki = 0usize;
    for l in t.lines() {
        if grab {
            if l.trim().is_empty() {
                continue;
            }
            if indent(l) <= ki {
                break;
            }
            out.push(l);
        } else if l.trim() == key {
            ki = indent(l);
            grab = true;
        }
    }
    out
}

fn dash_items(t: &str, key: &str) -> usize {
    let c = child_lines(t, key);
    let Some(m) = c.iter().map(|l| indent(l)).min() else {
        return 0;
    };
    c.iter()
        .filter(|l| indent(l) == m && l.trim_start().starts_with("- "))
        .count()
}

fn num_after(t: &str, key: &str) -> usize {
    for l in t.lines() {
        let l = l.trim();
        if let Some(v) = l.strip_prefix(key) {
            let v = v.trim().trim_matches('"');
            if let Ok(n) = v.parse::<usize>() {
                return n;
            }
        }
    }
    0
}

/// Reports whether `b` looks like an OpenUtau `*.ustx` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("ustx_version:")
        || (t.contains("beat_per_bar:") && t.contains("tracks:") && t.contains("ustx"))
}

impl Ustx {
    /// Parses `b` as an OpenUtau project, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let parts = t
            .lines()
            .filter(|l| l.trim() == "parts:")
            .map(|_| ())
            .count();
        let mut notes = 0;
        let mut notes_key = false;
        let mut ki = 0usize;
        for l in t.lines() {
            if notes_key {
                if l.trim().is_empty() {
                    continue;
                }
                if indent(l) <= ki {
                    notes_key = false;
                } else if l.trim_start().starts_with("- ") && indent(l) == ki + 2 {
                    notes += 1;
                }
            }
            if l.trim() == "notes:" {
                notes_key = true;
                ki = indent(l);
            }
        }
        Some(Ustx {
            version: num_after(t, "ustx_version:"),
            bpm: num_after(t, "bpm:"),
            beat_per_bar: num_after(t, "beat_per_bar:"),
            beat_unit: num_after(t, "beat_unit:"),
            resolution: num_after(t, "resolution:"),
            expressions: dash_items(t, "expressions:"),
            curves: dash_items(t, "curves:"),
            tempos: dash_items(t, "tempos:"),
            time_signatures: dash_items(t, "time_signatures:"),
            tracks: dash_items(t, "tracks:"),
            parts,
            notes,
            singers: t.matches("singer:").count(),
            phonemizers: t.matches("phonemizer:").count(),
            tones: t.matches("tone:").count(),
            lyrics: t.matches("lyric:").count(),
            vibrato: t.matches("vibrato:").count(),
            pitch: t.matches("pitch:").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"name: Demo\nustx_version: 6\nbpm: 120\nbeat_per_bar: 4\nbeat_unit: 4\nresolution: 480\ntempos:\n  - position: 0\n    bpm: 120\ntracks:\n  - track_name: Voice\n    singer: defoko\n    phonemizer: ja\n    parts:\n      - name: P1\n        position: 0\n        notes:\n          - position: 0\n            duration: 240\n            tone: 60\n            lyric: a\n            vibrato: {}\n";

    #[test]
    fn parses_ustx() {
        assert!(detect(S));
        let x = Ustx::parse(S).unwrap();
        assert_eq!(x.bpm, 120);
        assert_eq!(x.tracks, 1);
        assert_eq!(x.parts, 1);
        assert_eq!(x.notes, 1);
        assert_eq!(x.tones, 1);
    }

    #[test]
    fn rejects_non_ustx() {
        assert!(!detect(b"a: 1"));
        assert!(Ustx::parse(b"{}").is_none());
    }
}

//! Synthesizer V project (`*.svp`) JSON census.
//!
//! Counts `time`/`meter`/`tempo` plus `tracks`/`groups`/`notes` and the
//! `parameters` curves (`pitchDelta`, `vibratoEnv`, `loudness`, `tension`,
//! `breathiness`, `voicing`, `gender`, `timbre`).
//!
//! ```
//! let s = br#"{"version":25,"time":{"meter":[{"index":0,"numerator":4,"denominator":4}],"tempo":[{"position":0,"bpm":120}]},"tracks":[{"name":"t","mainRef":{"groupID":"g","blick":0},"groups":[{"name":"","notes":[{"onset":0,"duration":180,"lyrics":"la","pitch":60,"phonemes":"","attributes":{}}],"parameters":{"pitchDelta":{"mode":"cubic","points":[]},"vibratoEnv":{"points":[]},"loudness":{},"tension":{},"breathiness":{},"voicing":{},"gender":{}}}]}]}"#;
//! assert!(izanagi_kit::svp::detect(s));
//! let v = izanagi_kit::svp::Svp::parse(s).unwrap();
//! assert_eq!(v.notes, 1);
//! assert_eq!(v.groups, 1);
//! assert_eq!(v.tracks, 1);
//! assert_eq!(v.onsets, 1);
//! assert_eq!(v.parameters, 1);
//! ```

/// Parsed census of a `*.svp` Synthesizer V project.
#[derive(Debug, Clone)]
pub struct Svp {
    /// `"version"` value, or 0.
    pub version: usize,
    /// `"meter"` keys.
    pub meter: usize,
    /// `"numerator"` keys.
    pub numerators: usize,
    /// `"denominator"` keys.
    pub denominators: usize,
    /// `"tempo"` keys.
    pub tempos: usize,
    /// `"bpm"` keys.
    pub bpm: usize,
    /// `"position"` keys.
    pub positions: usize,
    /// `"tracks"` keys.
    pub tracks: usize,
    /// `"mainRef"` keys.
    pub main_refs: usize,
    /// `"groups"` keys.
    pub groups: usize,
    /// `"notes"` keys.
    pub notes: usize,
    /// `"onset"` keys.
    pub onsets: usize,
    /// `"duration"` keys.
    pub durations: usize,
    /// `"lyrics"` keys.
    pub lyrics: usize,
    /// `"pitch"` keys.
    pub pitches: usize,
    /// `"phonemes"` keys.
    pub phonemes: usize,
    /// `"attributes"` keys.
    pub attributes: usize,
    /// `"parameters"` keys.
    pub parameters: usize,
    /// `"pitchDelta"` keys.
    pub pitch_delta: usize,
    /// `"vibratoEnv"` keys.
    pub vibrato_env: usize,
    /// `"loudness"` keys.
    pub loudness: usize,
    /// `"tension"` keys.
    pub tension: usize,
    /// `"breathiness"` keys.
    pub breathiness: usize,
    /// `"voicing"` keys.
    pub voicing: usize,
    /// `"gender"` keys.
    pub gender: usize,
    /// `"timbre"` keys.
    pub timbre: usize,
    /// `"points"` keys.
    pub points: usize,
    /// `"name"` keys.
    pub names: usize,
}

/// Reports whether `b` looks like a `*.svp` document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"mainRef\"") && t.contains("\"onset\""))
        || (t.contains("\"meter\"") && t.contains("\"onset\""))
}

fn jnum(t: &str, key: &str) -> usize {
    let pat = format!("\"{key}\":");
    if let Some(i) = t.find(&pat) {
        return t[i + pat.len()..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
    }
    0
}

fn count(t: &str, key: &str) -> usize {
    t.matches(key).count()
}

impl Svp {
    /// Parses `b` as a SVP document, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Svp {
            version: jnum(t, "version"),
            meter: count(t, "\"meter\""),
            numerators: count(t, "\"numerator\""),
            denominators: count(t, "\"denominator\""),
            tempos: count(t, "\"tempo\""),
            bpm: count(t, "\"bpm\""),
            positions: count(t, "\"position\""),
            tracks: count(t, "\"tracks\""),
            main_refs: count(t, "\"mainRef\""),
            groups: count(t, "\"groups\""),
            notes: count(t, "\"notes\""),
            onsets: count(t, "\"onset\""),
            durations: count(t, "\"duration\""),
            lyrics: count(t, "\"lyrics\""),
            pitches: count(t, "\"pitch\""),
            phonemes: count(t, "\"phonemes\""),
            attributes: count(t, "\"attributes\""),
            parameters: count(t, "\"parameters\""),
            pitch_delta: count(t, "\"pitchDelta\""),
            vibrato_env: count(t, "\"vibratoEnv\""),
            loudness: count(t, "\"loudness\""),
            tension: count(t, "\"tension\""),
            breathiness: count(t, "\"breathiness\""),
            voicing: count(t, "\"voicing\""),
            gender: count(t, "\"gender\""),
            timbre: count(t, "\"timbre\""),
            points: count(t, "\"points\""),
            names: count(t, "\"name\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = br#"{"version":25,"time":{"meter":[{"index":0,"numerator":4,"denominator":4}],"tempo":[{"position":0,"bpm":120}]},"tracks":[{"name":"t","mainRef":{"groupID":"g","blick":0},"groups":[{"name":"","notes":[{"onset":0,"duration":180,"lyrics":"la","pitch":60,"phonemes":"","attributes":{}}],"parameters":{"pitchDelta":{"mode":"cubic","points":[]},"vibratoEnv":{"points":[]},"loudness":{},"tension":{},"breathiness":{},"voicing":{},"gender":{}}}]}]}"#;

    #[test]
    fn parses_svp() {
        assert!(detect(S));
        let v = Svp::parse(S).unwrap();
        assert_eq!(v.notes, 1);
        assert_eq!(v.groups, 1);
        assert_eq!(v.tracks, 1);
        assert_eq!(v.onsets, 1);
        assert_eq!(v.parameters, 1);
    }

    #[test]
    fn rejects_non_svp() {
        assert!(!detect(b"{}"));
        assert!(Svp::parse(b"[]").is_none());
    }
}

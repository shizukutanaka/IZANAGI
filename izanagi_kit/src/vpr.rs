//! VOCALOID5 project (`*.vpr`) JSON census.
//!
//! Counts `tracks`/`parts`/`notes` plus per-note `pos`/`duration`/`number`
//! /`lyric`/`phoneme` keys and master-track tempo/timeSig events.
//!
//! ```
//! let s = br#"{"title":"t","vprVersion":5,"masterTrack":{"tempo":{"events":[{"pos":0,"value":120}]},"timeSig":{"numerator":4,"denominator":4}},"tracks":[{"name":"T","parts":[{"name":"P","duration":480,"notes":[{"pos":0,"duration":240,"number":60,"lyric":"a","phoneme":"a"}]}]}]}"#;
//! assert!(izanagi_kit::vpr::detect(s));
//! let v = izanagi_kit::vpr::Vpr::parse(s).unwrap();
//! assert_eq!(v.notes, 1);
//! assert_eq!(v.parts, 1);
//! assert_eq!(v.tracks, 1);
//! assert_eq!(v.lyrics, 1);
//! assert_eq!(v.phonemes, 1);
//! ```

/// Parsed census of a `*.vpr` VOCALOID5 project.
#[derive(Debug, Clone)]
pub struct Vpr {
    /// `vprVersion` value, or 0.
    pub vpr_version: usize,
    /// `"tracks"` keys.
    pub tracks: usize,
    /// `"parts"` keys.
    pub parts: usize,
    /// `"notes"` keys.
    pub notes: usize,
    /// `"pos"` keys.
    pub pos: usize,
    /// `"duration"` keys.
    pub durations: usize,
    /// `"number"` keys.
    pub numbers: usize,
    /// `"lyric"` keys.
    pub lyrics: usize,
    /// `"phoneme"` keys.
    pub phonemes: usize,
    /// `"name"` keys.
    pub names: usize,
    /// `"events"` keys.
    pub events: usize,
    /// `"value"` keys.
    pub values: usize,
    /// `"tempo"` keys.
    pub tempos: usize,
    /// `"timeSig"` keys.
    pub time_sigs: usize,
    /// `"numerator"` keys.
    pub numerators: usize,
    /// `"denominator"` keys.
    pub denominators: usize,
    /// `"pitchBend"`/`"pitchBendSens"` keys.
    pub pitch_bends: usize,
    /// `"vibrato"` keys.
    pub vibrato: usize,
    /// `"dynamics"` keys.
    pub dynamics: usize,
    /// `"singingSkill"` keys.
    pub singing_skill: usize,
    /// `"voice"` keys.
    pub voices: usize,
    /// `"title"` keys.
    pub titles: usize,
    /// `"loop"` keys.
    pub loops: usize,
}

/// Reports whether `b` looks like a `*.vpr` document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("\"vprVersion\"")
        || (t.contains("\"masterTrack\"") && t.contains("\"parts\"") && t.contains("\"pos\""))
}

fn jnum(t: &str, key: &str) -> usize {
    let pat = format!("\"{key}\":");
    let mut n = 0;
    if let Some(i) = t.find(&pat) {
        let rest = &t[i + pat.len()..];
        let d: usize = rest
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        n = d;
    }
    n
}

fn count(t: &str, key: &str) -> usize {
    t.matches(key).count()
}

impl Vpr {
    /// Parses `b` as a VPR document, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Vpr {
            vpr_version: jnum(t, "vprVersion"),
            tracks: count(t, "\"tracks\""),
            parts: count(t, "\"parts\""),
            notes: count(t, "\"notes\""),
            pos: count(t, "\"pos\""),
            durations: count(t, "\"duration\""),
            numbers: count(t, "\"number\""),
            lyrics: count(t, "\"lyric\""),
            phonemes: count(t, "\"phoneme\""),
            names: count(t, "\"name\""),
            events: count(t, "\"events\""),
            values: count(t, "\"value\""),
            tempos: count(t, "\"tempo\""),
            time_sigs: count(t, "\"timeSig\""),
            numerators: count(t, "\"numerator\""),
            denominators: count(t, "\"denominator\""),
            pitch_bends: count(t, "\"pitchBend\"") + count(t, "\"pitchBendSens\""),
            vibrato: count(t, "\"vibrato\""),
            dynamics: count(t, "\"dynamics\""),
            singing_skill: count(t, "\"singingSkill\""),
            voices: count(t, "\"voice\""),
            titles: count(t, "\"title\""),
            loops: count(t, "\"loop\""),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = br#"{"title":"t","vprVersion":5,"masterTrack":{"tempo":{"events":[{"pos":0,"value":120}]},"timeSig":{"numerator":4,"denominator":4}},"tracks":[{"name":"T","parts":[{"name":"P","duration":480,"notes":[{"pos":0,"duration":240,"number":60,"lyric":"a","phoneme":"a"}]}]}]}"#;

    #[test]
    fn parses_vpr() {
        assert!(detect(S));
        let v = Vpr::parse(S).unwrap();
        assert_eq!(v.notes, 1);
        assert_eq!(v.parts, 1);
        assert_eq!(v.tracks, 1);
        assert_eq!(v.lyrics, 1);
        assert_eq!(v.phonemes, 1);
    }

    #[test]
    fn rejects_non_vpr() {
        assert!(!detect(b"{}"));
        assert!(Vpr::parse(b"[]").is_none());
    }
}

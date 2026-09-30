//! CeVIO project (`*.ccs`) XML census.
//!
//! Counts `Scene`/`Unit`/`Song`/`Talk`/`Part` structure plus `Note`
//! elements with `Clock`/`Duration`/`Lyric`/`Phoneme`/`PitchOctave`
//! /`PitchStep` attributes.
//!
//! ```
//! let s = br#"<CevioData version="5"><Scene><Unit><CastId>c1</CastId><Song><Score><Part><Dynamics/><Note Clock="0" Duration="480" Lyric="a" Phoneme="a" DoReMi="false" PitchOctave="4" PitchStep="0"/></Part></Score></Song></Unit></CevioData>"#;
//! assert!(izanagi_kit::ccs::detect(s));
//! let c = izanagi_kit::ccs::Ccs::parse(s).unwrap();
//! assert_eq!(c.notes, 1);
//! assert_eq!(c.songs, 1);
//! assert_eq!(c.units, 1);
//! assert_eq!(c.lyrics, 1);
//! assert_eq!(c.pitch_octaves, 1);
//! ```

/// Parsed census of a `*.ccs` CeVIO project.
#[derive(Debug, Clone)]
pub struct Ccs {
    /// `<Scene>` elements.
    pub scenes: usize,
    /// `<Unit>` elements.
    pub units: usize,
    /// `<Song>` elements.
    pub songs: usize,
    /// `<Talk>` elements.
    pub talks: usize,
    /// `<Score>` elements.
    pub scores: usize,
    /// `<Part>` elements.
    pub parts: usize,
    /// `<Note` elements.
    pub notes: usize,
    /// `Clock=` attributes.
    pub clocks: usize,
    /// `Duration=` attributes.
    pub durations: usize,
    /// `Lyric=` attributes.
    pub lyrics: usize,
    /// `Phoneme=` attributes.
    pub phonemes: usize,
    /// `PitchOctave=` attributes.
    pub pitch_octaves: usize,
    /// `PitchStep=` attributes.
    pub pitch_steps: usize,
    /// `<Dynamics` elements.
    pub dynamics: usize,
    /// `<Text` elements.
    pub texts: usize,
    /// `<CastId>` elements.
    pub casts: usize,
    /// `<Tempo`/`Tempo=` markers.
    pub tempos: usize,
    /// `<Time`/`Time=` markers.
    pub times: usize,
}

/// Reports whether `b` looks like a `*.ccs` document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("<CevioData") || (t.contains("<Note") && t.contains("Phoneme="))
}

fn count(t: &str, key: &str) -> usize {
    t.matches(key).count()
}

impl Ccs {
    /// Parses `b` as a CCS document, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Ccs {
            scenes: count(t, "<Scene>"),
            units: count(t, "<Unit>"),
            songs: count(t, "<Song>"),
            talks: count(t, "<Talk>"),
            scores: count(t, "<Score>"),
            parts: count(t, "<Part>"),
            notes: count(t, "<Note "),
            clocks: count(t, "Clock="),
            durations: count(t, "Duration="),
            lyrics: count(t, "Lyric="),
            phonemes: count(t, "Phoneme="),
            pitch_octaves: count(t, "PitchOctave="),
            pitch_steps: count(t, "PitchStep="),
            dynamics: count(t, "<Dynamics"),
            texts: count(t, "<Text"),
            casts: count(t, "<CastId>"),
            tempos: count(t, "<Tempo") + count(t, "Tempo="),
            times: count(t, "<Time") + count(t, "Time="),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = br#"<CevioData version="5"><Scene><Unit><CastId>c1</CastId><Song><Score><Part><Dynamics/><Note Clock="0" Duration="480" Lyric="a" Phoneme="a" DoReMi="false" PitchOctave="4" PitchStep="0"/></Part></Score></Song></Unit></CevioData>"#;

    #[test]
    fn parses_ccs() {
        assert!(detect(S));
        let c = Ccs::parse(S).unwrap();
        assert_eq!(c.notes, 1);
        assert_eq!(c.songs, 1);
        assert_eq!(c.units, 1);
        assert_eq!(c.lyrics, 1);
        assert_eq!(c.pitch_octaves, 1);
    }

    #[test]
    fn rejects_non_ccs() {
        assert!(!detect(b"<xml/>"));
        assert!(Ccs::parse(b"{}").is_none());
    }
}

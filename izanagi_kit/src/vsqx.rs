//! VOCALOID3/4 sequence (`*.vsqx`) XML census.
//!
//! Counts `vsqx` root children: master track (tempo/timeSig/preMeasure),
//! `vsTrack`/`musicalPart`/`note` elements and lyric/phoneme payloads.
//!
//! ```
//! let s = br#"<vsqx><vender>Yamaha</vender><masterTrack><seqName>M</seqName><resolution>480</resolution><preMeasure>4</preMeasure><timeSig><nume>4</nume><denomi>4</denomi></timeSig><tempo><posTick>0</posTick><bpm>120</bpm></tempo></masterTrack><vsTrack><musicalPart><posTick>0</posTick><durTick>960</durTick><note><posTick>0</posTick><durTick>480</durTick><noteNum>60</noteNum><lyric>a</lyric><phnms>a</phnms></note></musicalPart></vsTrack></vsqx>"#;
//! assert!(izanagi_kit::vsqx::detect(s));
//! let v = izanagi_kit::vsqx::Vsqx::parse(s).unwrap();
//! assert_eq!(v.notes, 1);
//! assert_eq!(v.parts, 1);
//! assert_eq!(v.tracks, 1);
//! assert_eq!(v.note_nums, 1);
//! assert_eq!(v.lyrics, 1);
//! ```

/// Parsed census of a `*.vsqx` VOCALOID sequence.
#[derive(Debug, Clone)]
pub struct Vsqx {
    /// `<note>` elements.
    pub notes: usize,
    /// `<noteNum>` elements.
    pub note_nums: usize,
    /// `<posTick>` elements.
    pub pos_ticks: usize,
    /// `<durTick>` elements.
    pub dur_ticks: usize,
    /// `<lyric>` elements.
    pub lyrics: usize,
    /// `<phnms>` elements.
    pub phnms: usize,
    /// `<musicalPart>` elements.
    pub parts: usize,
    /// `<vsTrack>` elements.
    pub tracks: usize,
    /// `<vVoice>` elements.
    pub voices: usize,
    /// `<tempo>` elements.
    pub tempos: usize,
    /// `<timeSig>` elements.
    pub time_sigs: usize,
    /// `<preMeasure>` elements.
    pub pre_measures: usize,
    /// `<seqName>` elements.
    pub seq_names: usize,
    /// `<singer>` elements.
    pub singers: usize,
    /// `<resolution>` elements.
    pub resolutions: usize,
    /// `<monoTrack>` elements.
    pub mono_tracks: usize,
    /// `<stTrack>` elements.
    pub stereo_tracks: usize,
    /// `<aux>` elements.
    pub aux: usize,
    /// `<wavUnit>` elements.
    pub wav_units: usize,
}

/// Reports whether `b` looks like a `*.vsqx` document.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("<vsqx") || (t.contains("<vVoiceTable") && t.contains("<masterTrack"))
}

fn count(t: &str, tag: &str) -> usize {
    t.matches(tag).count()
}

impl Vsqx {
    /// Parses `b` as a VSQX document, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        Some(Vsqx {
            notes: count(t, "<note>") + count(t, "<note "),
            note_nums: count(t, "<noteNum>"),
            pos_ticks: count(t, "<posTick>"),
            dur_ticks: count(t, "<durTick>"),
            lyrics: count(t, "<lyric>"),
            phnms: count(t, "<phnms>"),
            parts: count(t, "<musicalPart>"),
            tracks: count(t, "<vsTrack>"),
            voices: count(t, "<vVoice>"),
            tempos: count(t, "<tempo>"),
            time_sigs: count(t, "<timeSig>"),
            pre_measures: count(t, "<preMeasure>"),
            seq_names: count(t, "<seqName>"),
            singers: count(t, "<singer>"),
            resolutions: count(t, "<resolution>"),
            mono_tracks: count(t, "<monoTrack>"),
            stereo_tracks: count(t, "<stTrack>"),
            aux: count(t, "<aux>"),
            wav_units: count(t, "<wavUnit>"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = br#"<vsqx><vender>Yamaha</vender><masterTrack><seqName>M</seqName><resolution>480</resolution><preMeasure>4</preMeasure><timeSig><nume>4</nume><denomi>4</denomi></timeSig><tempo><posTick>0</posTick><bpm>120</bpm></tempo></masterTrack><vsTrack><musicalPart><posTick>0</posTick><durTick>960</durTick><note><posTick>0</posTick><durTick>480</durTick><noteNum>60</noteNum><lyric>a</lyric><phnms>a</phnms></note></musicalPart></vsTrack></vsqx>"#;

    #[test]
    fn parses_vsqx() {
        assert!(detect(S));
        let v = Vsqx::parse(S).unwrap();
        assert_eq!(v.notes, 1);
        assert_eq!(v.parts, 1);
        assert_eq!(v.tracks, 1);
        assert_eq!(v.note_nums, 1);
        assert_eq!(v.lyrics, 1);
    }

    #[test]
    fn rejects_non_vsqx() {
        assert!(!detect(b"<xml/>"));
        assert!(Vsqx::parse(b"{}").is_none());
    }
}

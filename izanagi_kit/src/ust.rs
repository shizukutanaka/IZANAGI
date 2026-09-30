//! UTAU sequence (`*.ust`) section census.
//!
//! INI-like text: a `[#SETTING]` header, one `[#NNNN]` section per note,
//! optional `[#PREV]`/`[#NEXT]` sections and a trailing `[#TRACKEND]`.
//!
//! ```
//! let s = b"[#SETTING]\nTempo=120\nProjectName=test\n[#0000]\nLength=480\nLyric=R\nNoteNum=60\n[#0001]\nLength=240\nLyric=a\nNoteNum=62\nIntensity=100\n[#TRACKEND]\n";
//! assert!(izanagi_kit::ust::detect(s));
//! let u = izanagi_kit::ust::Ust::parse(s).unwrap();
//! assert_eq!(u.sections, 4);
//! assert_eq!(u.notes, 2);
//! assert_eq!(u.rest_notes, 1);
//! assert_eq!(u.lyrics, 2);
//! assert_eq!(u.intensity, 1);
//! ```

/// Parsed census of a UTAU `*.ust` sequence.
#[derive(Debug, Clone)]
pub struct Ust {
    /// Total `[#...]` section headers.
    pub sections: usize,
    /// Note sections named `[#` + digits + `]`.
    pub notes: usize,
    /// Notes whose `Lyric` is `R` (rest).
    pub rest_notes: usize,
    /// `Key=Value` line count.
    pub keys: usize,
    /// `Tempo=` assignments.
    pub tempo: usize,
    /// `Lyric=` assignments.
    pub lyrics: usize,
    /// `NoteNum=` assignments.
    pub note_nums: usize,
    /// `Length=` assignments.
    pub lengths: usize,
    /// `PreUtterance=` assignments.
    pub pre_utterance: usize,
    /// `Envelope=` assignments.
    pub envelopes: usize,
    /// `Piches=`/`PitchBend=`/`PBS=`/`PBW=` assignments.
    pub pitch_bends: usize,
    /// `Flags=` assignments.
    pub flags: usize,
    /// `Intensity=` assignments.
    pub intensity: usize,
    /// `Modulation=` assignments.
    pub modulation: usize,
    /// `Velocity=` assignments.
    pub velocity: usize,
    /// `StartPoint=` assignments.
    pub start_points: usize,
    /// `Mode2=True` markers.
    pub mode2: usize,
}

/// Reports whether `b` looks like a UTAU `*.ust` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    if !t.contains("[#") || !t.contains('=') {
        return false;
    }
    t.lines().any(|l| {
        let l = l.trim();
        l == "[#SETTING]"
            || l == "[#TRACKEND]"
            || (l.starts_with("[#")
                && l.ends_with(']')
                && l[2..l.len() - 1].bytes().all(|c| c.is_ascii_digit()))
    })
}

impl Ust {
    /// Parses `b` as a UTAU sequence, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut u = Ust {
            sections: 0,
            notes: 0,
            rest_notes: 0,
            keys: 0,
            tempo: 0,
            lyrics: 0,
            note_nums: 0,
            lengths: 0,
            pre_utterance: 0,
            envelopes: 0,
            pitch_bends: 0,
            flags: 0,
            intensity: 0,
            modulation: 0,
            velocity: 0,
            start_points: 0,
            mode2: 0,
        };
        let mut in_note = false;
        for l in t.lines() {
            let l = l.trim_end();
            if l.starts_with("[#") && l.ends_with(']') {
                u.sections += 1;
                in_note = l[2..l.len() - 1].bytes().all(|c| c.is_ascii_digit());
                if in_note {
                    u.notes += 1;
                }
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                u.keys += 1;
                match k {
                    "Tempo" => u.tempo += 1,
                    "Lyric" => {
                        u.lyrics += 1;
                        if in_note && v == "R" {
                            u.rest_notes += 1;
                        }
                    }
                    "NoteNum" => u.note_nums += 1,
                    "Length" => u.lengths += 1,
                    "PreUtterance" => u.pre_utterance += 1,
                    "Envelope" => u.envelopes += 1,
                    "Piches" | "PitchBend" | "PBS" | "PBW" | "PBY" | "PBM" | "PBType" => {
                        u.pitch_bends += 1
                    }
                    "Flags" => u.flags += 1,
                    "Intensity" => u.intensity += 1,
                    "Modulation" => u.modulation += 1,
                    "Velocity" => u.velocity += 1,
                    "StartPoint" => u.start_points += 1,
                    "Mode2" if v == "True" => u.mode2 += 1,
                    _ => {}
                }
            }
        }
        Some(u)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"[#SETTING]\nTempo=120\nProjectName=test\n[#0000]\nLength=480\nLyric=R\nNoteNum=60\n[#0001]\nLength=240\nLyric=a\nNoteNum=62\nIntensity=100\n[#TRACKEND]\n";

    #[test]
    fn parses_ust() {
        assert!(detect(S));
        let u = Ust::parse(S).unwrap();
        assert_eq!(u.sections, 4);
        assert_eq!(u.notes, 2);
        assert_eq!(u.rest_notes, 1);
        assert_eq!(u.lyrics, 2);
        assert_eq!(u.intensity, 1);
    }

    #[test]
    fn rejects_non_ust() {
        assert!(!detect(b"just text"));
        assert!(Ust::parse(b"{}").is_none());
    }
}

//! BrainVision marker (`*.vmrk`) census.
//!
//! `[Brain Vision Data Format Marker]` header, `Data File=` reference
//! and `[Marker Infos]` `MkN=Type,Description,Position,Points,Channel,
//! Date` entries.
//!
//! ```
//! let s = b"[Brain Vision Data Format Marker]\nData File=d.eeg\n[Marker Infos]\nMk1=New Segment,,1,1,0,20200101120000\nMk2=Stimulus,S  1,100,1,0\nMk3=Response,R  1,200,1,0\n";
//! assert!(izanagi_kit::vmrk::detect(s));
//! let m = izanagi_kit::vmrk::Vmrk::parse(s).unwrap();
//! assert_eq!(m.markers, 3);
//! assert_eq!(m.new_segments, 1);
//! assert_eq!(m.stimuli, 1);
//! assert_eq!(m.responses, 1);
//! assert_eq!(m.positions, 3);
//! ```

/// Parsed census of a BrainVision `*.vmrk` marker file.
#[derive(Debug, Clone)]
pub struct Vmrk {
    /// `[...]` section headers.
    pub sections: usize,
    /// `MkN=` marker entries.
    pub markers: usize,
    /// `New Segment` markers.
    pub new_segments: usize,
    /// `Stimulus` markers.
    pub stimuli: usize,
    /// `Response` markers.
    pub responses: usize,
    /// `Comment` markers.
    pub comment_markers: usize,
    /// Other marker types.
    pub other_types: usize,
    /// Comma-separated position fields.
    pub positions: usize,
    /// Point-count fields.
    pub points: usize,
    /// Non-zero channel fields.
    pub channels: usize,
    /// `YYYYMMDDHHMMSS` date fields.
    pub dates: usize,
    /// `Data File=` assignments.
    pub data_files: usize,
    /// `[Marker Infos]` sections.
    pub marker_infos: usize,
    /// `[Comment]` section lines.
    pub comments: usize,
}

/// Reports whether `b` looks like a BrainVision marker file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("[Brain Vision") && t.contains("Mk") || t.contains("[Marker Infos]")
}

impl Vmrk {
    /// Parses `b` as a marker file, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut m = Vmrk {
            sections: 0,
            markers: 0,
            new_segments: 0,
            stimuli: 0,
            responses: 0,
            comment_markers: 0,
            other_types: 0,
            positions: 0,
            points: 0,
            channels: 0,
            dates: 0,
            data_files: 0,
            marker_infos: 0,
            comments: 0,
        };
        let mut in_comment = false;
        for l in t.lines() {
            let l = l.trim_end();
            if l.starts_with('[') && l.ends_with(']') {
                m.sections += 1;
                in_comment = l == "[Comment]";
                if l == "[Marker Infos]" {
                    m.marker_infos += 1;
                }
                continue;
            }
            if in_comment {
                if !l.trim().is_empty() {
                    m.comments += 1;
                }
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                if k == "Data File" {
                    m.data_files += 1;
                    continue;
                }
                if k.starts_with("Mk") && k[2..].bytes().all(|c| c.is_ascii_digit()) {
                    m.markers += 1;
                    let f: Vec<&str> = v.split(',').collect();
                    let ty = f.first().map_or("", |s| s.trim());
                    match ty {
                        "New Segment" => m.new_segments += 1,
                        "Stimulus" => m.stimuli += 1,
                        "Response" => m.responses += 1,
                        "Comment" => m.comment_markers += 1,
                        _ => m.other_types += 1,
                    }
                    if f.get(2).is_some_and(|p| {
                        p.trim().bytes().all(|c| c.is_ascii_digit()) && !p.trim().is_empty()
                    }) {
                        m.positions += 1;
                    }
                    if f.get(3).is_some_and(|p| !p.trim().is_empty()) {
                        m.points += 1;
                    }
                    if f.get(4)
                        .is_some_and(|c| c.trim().parse::<usize>().unwrap_or(0) > 0)
                    {
                        m.channels += 1;
                    }
                    if f.get(5).is_some_and(|d| {
                        d.trim().len() >= 14 && d.trim().bytes().all(|c| c.is_ascii_digit())
                    }) {
                        m.dates += 1;
                    }
                }
            }
        }
        Some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"[Brain Vision Data Format Marker]\nData File=d.eeg\n[Marker Infos]\nMk1=New Segment,,1,1,0,20200101120000\nMk2=Stimulus,S  1,100,1,0\nMk3=Response,R  1,200,1,0\n";

    #[test]
    fn parses_vmrk() {
        assert!(detect(S));
        let m = Vmrk::parse(S).unwrap();
        assert_eq!(m.markers, 3);
        assert_eq!(m.new_segments, 1);
        assert_eq!(m.stimuli, 1);
        assert_eq!(m.responses, 1);
        assert_eq!(m.positions, 3);
    }

    #[test]
    fn rejects_non_vmrk() {
        assert!(!detect(b"[x]"));
        assert!(Vmrk::parse(b"").is_none());
    }
}

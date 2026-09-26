//! CUE sheet (`.cue`) — the text sidecar describing a disc image's
//! track layout.
//!
//! Recognized commands: `FILE "name" TYPE`, `TRACK nn MODE`,
//! `INDEX ii mm:ss:ff`, `PREGAP`/`POSTGAP mm:ss:ff`,
//! `TITLE`/`PERFORMER`/`SONGWRITER` strings, `FILE`/`FLAGS` and
//! `REM` (kept verbatim). Times convert to 75-frames-per-second
//! disc frames via [`mmssff`].
//!
//! ```
//! use izanagi_kit::cue::{parse, mmssff};
//!
//! let cue = parse("FILE \"disk.bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    INDEX 01 00:05:00\n").unwrap();
//! assert_eq!(cue.files.len(), 1);
//! assert_eq!(cue.tracks.len(), 2);
//! assert_eq!(cue.tracks[1].lba(), Some(5 * 75));
//! assert_eq!(mmssff(1, 2, 3), Some(75 * 62 + 3));
//! ```

use std::string::String;
use std::vec::Vec;

/// Disc sector rate: 75 frames per second.
pub const FPS: u32 = 75;

/// Convert `mm:ss:ff` to an absolute frame number.
pub fn mmssff(m: u32, s: u32, f: u32) -> Option<u32> {
    if s >= 60 || f >= FPS {
        return None;
    }
    Some((m * 60 + s) * FPS + f)
}

/// One `TRACK` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// Track number (usually 1-based, 0 and 0xAA have meaning on
    /// some writers).
    pub number: u8,
    /// Mode string (`AUDIO`, `MODE1/2352`, `CDI/2352`, …).
    pub mode: String,
    /// Index of this track's `FILE` in [`Cue::files`].
    pub file: usize,
    /// `TITLE` inside the track block.
    pub title: String,
    /// `PERFORMER` inside the track block.
    pub performer: String,
    /// Index entries `(index number, disc frame)`.
    pub indices: Vec<(u8, u32)>,
    /// `PREGAP`/`POSTGAP` lengths in frames when present.
    pub pregap: Option<u32>,
    /// Post-gap.
    pub postgap: Option<u32>,
    /// Track `FLAGS` (DCP/4CH/PRE/SCPS) verbatim.
    pub flags: String,
}

impl Track {
    /// Start frame: the `INDEX 01` position when present.
    pub fn lba(&self) -> Option<u32> {
        self.indices.iter().find(|(n, _)| *n == 1).map(|(_, f)| *f)
    }
}

/// A parsed CUE sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    /// `FILE` commands as `(name, kind)`.
    pub files: Vec<(String, String)>,
    /// Track blocks in order.
    pub tracks: Vec<Track>,
    /// Disc-level `TITLE`/`PERFORMER`/`SONGWRITER`/`CATALOG`.
    pub title: String,
    /// Performer.
    pub performer: String,
    /// `CATALOG` barcode.
    pub catalog: String,
    /// `REM` comment lines.
    pub rem: Vec<String>,
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s)
}

fn time_token(tok: &str) -> Option<u32> {
    let mut parts = tok.splitn(4, ':');
    let m: u32 = parts.next()?.parse().ok()?;
    let s: u32 = parts.next()?.parse().ok()?;
    let f: u32 = parts.next()?.parse().ok()?;
    mmssff(m, s, f)
}

/// Parse a CUE sheet, or `None` on a structural error (bad time,
/// TRACK before FILE, …).
pub fn parse(text: &str) -> Option<Cue> {
    let mut cue = Cue {
        files: Vec::new(),
        tracks: Vec::new(),
        title: String::new(),
        performer: String::new(),
        catalog: String::new(),
        rem: Vec::new(),
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.splitn(2, [' ', '\t']);
        let cmd = it.next()?;
        let rest = it.next().unwrap_or("").trim();
        let upper = cmd.to_ascii_uppercase();
        let cur = cue.tracks.last_mut();
        match upper.as_str() {
            "FILE" => {
                let mut p = rest.splitn(2, [' ', '\t']);
                let name = unquote(p.next()?);
                let kind = p.next().unwrap_or("").trim();
                cue.files
                    .push((name.to_string(), kind.to_ascii_uppercase()));
            }
            "TRACK" => {
                let mut p = rest.splitn(2, [' ', '\t']);
                let n: u8 = p.next()?.parse().ok()?;
                let mode = p.next().unwrap_or("").trim();
                if cue.files.is_empty() {
                    return None;
                }
                cue.tracks.push(Track {
                    number: n,
                    mode: mode.to_ascii_uppercase(),
                    file: cue.files.len() - 1,
                    title: String::new(),
                    performer: String::new(),
                    indices: Vec::new(),
                    pregap: None,
                    postgap: None,
                    flags: String::new(),
                });
            }
            "INDEX" => {
                let t = cur?;
                let mut p = rest.splitn(2, [' ', '\t']);
                let n: u8 = p.next()?.parse().ok()?;
                let f = time_token(p.next().unwrap_or("").trim())?;
                t.indices.push((n, f));
            }
            "PREGAP" => cur?.pregap = Some(time_token(rest)?),
            "POSTGAP" => cur?.postgap = Some(time_token(rest)?),
            "TITLE" => {
                let v = unquote(rest);
                match cur {
                    Some(t) => t.title = v.to_string(),
                    None => cue.title = v.to_string(),
                }
            }
            "PERFORMER" => {
                let v = unquote(rest);
                match cur {
                    Some(t) => t.performer = v.to_string(),
                    None => cue.performer = v.to_string(),
                }
            }
            "SONGWRITER" => {}
            "CATALOG" => cue.catalog = rest.trim().to_string(),
            "FLAGS" => {
                if let Some(t) = cur {
                    t.flags = rest.to_ascii_uppercase();
                }
            }
            "REM" => cue.rem.push(rest.to_string()),
            _ => {}
        }
    }
    if cue.tracks.is_empty() {
        return None;
    }
    Some(cue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_layout() {
        let c = parse(
            "TITLE \"DISK\"\n\
             PERFORMER \"DEV\"\n\
             CATALOG 0123456789012\n\
             REM genre test\n\
             FILE \"a.wav\" WAVE\n\
             \x20 TRACK 01 AUDIO\n\
             \x20\x20 INDEX 00 00:00:00\n\
             \x20\x20 INDEX 01 00:02:00\n\
             \x20\x20 PREGAP 00:01:00\n",
        )
        .unwrap();
        assert_eq!(c.title, "DISK");
        assert_eq!(c.performer, "DEV");
        assert_eq!(c.catalog, "0123456789012");
        assert_eq!(c.rem, ["genre test"]);
        assert_eq!(c.files, [("a.wav".to_string(), "WAVE".to_string())]);
        let t = &c.tracks[0];
        assert_eq!(t.number, 1);
        assert_eq!(t.mode, "AUDIO");
        assert_eq!(t.file, 0);
        assert_eq!(t.indices, [(0, 0), (1, 150)]);
        assert_eq!(t.lba(), Some(150));
        assert_eq!(t.pregap, Some(75));
        assert_eq!(t.postgap, None);
    }

    #[test]
    fn multi_file_and_track_title() {
        let c = parse(
            "FILE \"t1.bin\" BINARY\n TRACK 1 MODE1/2352\n  INDEX 1 00:00:00\n FILE \"t2.bin\" BINARY\n TRACK 2 MODE1/2352\n  TITLE \"Second\"\n  FLAGS DCP\n  INDEX 1 00:10:00\n",
        )
        .unwrap();
        assert_eq!(c.tracks.len(), 2);
        assert_eq!(c.tracks[1].file, 1);
        assert_eq!(c.tracks[1].title, "Second");
        assert_eq!(c.tracks[1].flags, "DCP");
        assert_eq!(c.tracks[1].lba(), Some(10 * 75));
    }

    #[test]
    fn mmssff_bounds() {
        assert_eq!(mmssff(0, 0, 0), Some(0));
        assert_eq!(mmssff(79, 59, 74), Some((79 * 60 + 59) * 75 + 74));
        assert_eq!(mmssff(0, 60, 0), None);
        assert_eq!(mmssff(0, 0, 75), None);
        assert_eq!(time_token("01:02:03"), Some(75 * 62 + 3));
        assert_eq!(time_token("bogus"), None);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("TRACK 1 AUDIO\n INDEX 1 00:00:00"), None); // no FILE
        assert_eq!(
            parse("FILE \"x\" BINARY\nTRACK 1 AUDIO\nINDEX 1 99:99:99"),
            None
        );
        assert_eq!(
            parse("FILE \"x\" BINARY\nTRACK 1 AUDIO\nINDEX x 00:00:00"),
            None
        );
    }
}

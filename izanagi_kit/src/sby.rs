//! SymbiYosys `.sby` configuration census.
//!
//! INI-style sections `[tasks]` `[options]` `[engines]` `[script]`
//! `[file …]`/`[files]` with `key: value` (script/files) or
//! `key value` (options/tasks) lines; `#` comments. Modes:
//! `bmc`, `prove`, `cover`, `live`.
//!
//! ```
//! let d = b"[options]\nmode bmc\ndepth 10\n\n[engines]\nsmtbmc yices\n\n\
//! [script]\nread -formal top.v\nprep -top top\n\n[files]\ntop.v\n";
//! let s = izanagi_kit::sby::parse(d).unwrap();
//! assert_eq!(s.mode, Some(3)); // "bmc"
//! assert_eq!(s.engine_lines, 1);
//! ```

/// Census fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sby {
    /// Section name seen bitfield is flattened into counts below.
    pub tasks_seen: bool,
    /// `[options]` present.
    pub options_seen: bool,
    /// `[engines]` present.
    pub engines_seen: bool,
    /// `[script]` present.
    pub script_seen: bool,
    /// `[file`/`[files]` present.
    pub files_seen: bool,
    /// `mode` value length (`bmc`=3/`prove`=5/`cover`=5/`live`=4) — `Some(0)` empty.
    pub mode: Option<usize>,
    /// `depth` value when numeric.
    pub depth: Option<u32>,
    /// Non-empty lines inside `[engines]`.
    pub engine_lines: u32,
    /// Non-empty lines inside `[script]`.
    pub script_lines: u32,
    /// Non-empty lines inside `[file`/`[files]`.
    pub file_lines: u32,
    /// `key value`/`key: value` options in `[options]`/`[tasks]`.
    pub option_lines: u32,
}

#[derive(PartialEq)]
enum Sec {
    Tasks,
    Options,
    Engines,
    Script,
    Files,
    Other,
}

fn section_of(t: &str) -> Sec {
    let inner = t
        .strip_prefix('[')
        .and_then(|x| x.find(']').map(|e| &x[..e]))
        .unwrap_or("");
    match inner {
        "tasks" => Sec::Tasks,
        "options" => Sec::Options,
        "engines" => Sec::Engines,
        "script" => Sec::Script,
        _ if inner.starts_with("file") => Sec::Files,
        _ => Sec::Other,
    }
}

/// `true` on ≥2 known SBY sections (`[engines]` + one more).
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut seen = 0u8;
    for line in s.lines() {
        let t = line.trim();
        match section_of(t) {
            Sec::Options => seen |= 1,
            Sec::Engines => seen |= 2,
            Sec::Script => seen |= 4,
            Sec::Tasks => seen |= 8,
            Sec::Files => seen |= 16,
            Sec::Other => {}
        }
    }
    seen & 2 != 0 && seen.count_ones() >= 2
}

/// Census; `None` without SBY sections.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Sby> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut r = Sby::default();
    let mut sec = Sec::Other;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') {
            sec = section_of(t);
            match sec {
                Sec::Tasks => r.tasks_seen = true,
                Sec::Options => r.options_seen = true,
                Sec::Engines => r.engines_seen = true,
                Sec::Script => r.script_seen = true,
                Sec::Files => r.files_seen = true,
                Sec::Other => {}
            }
            continue;
        }
        match sec {
            Sec::Engines => r.engine_lines += 1,
            Sec::Script => r.script_lines += 1,
            Sec::Files => r.file_lines += 1,
            Sec::Tasks | Sec::Options => {
                r.option_lines += 1;
                let (k, v) = t.split_once([' ', ':']).unwrap_or((t, ""));
                let v = v.trim_start_matches([' ', ':']);
                if k == "mode" {
                    r.mode = Some(v.len());
                } else if k == "depth" {
                    r.depth = v.parse().ok();
                }
            }
            Sec::Other => {}
        }
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"[tasks]\nprove\n\n[options]\nmode bmc\ndepth 10\n\n\
[engines]\nsmtbmc yices\nabc pdr\n\n[script]\nread -formal top.v\nprep -top top\n\n\
[files]\ntop.v\nlib.v\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"[engines]\nsmtbmc\n")); // only one known section
        assert!(!detect(b"[foo]\nbar baz\n"));
    }

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert!(s.tasks_seen && s.options_seen && s.engines_seen);
        assert!(s.script_seen && s.files_seen);
        assert_eq!(s.mode, Some(3));
        assert_eq!(s.depth, Some(10));
        assert_eq!(s.engine_lines, 2);
        assert_eq!(s.script_lines, 2);
        assert_eq!(s.file_lines, 2);
        assert_eq!(s.option_lines, 3); // tasks "prove" + 2 options
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"[x]\nmode bmc\n").is_none());
    }
}

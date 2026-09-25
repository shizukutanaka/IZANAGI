//! SubRip (`.srt`) subtitles — blocks of
//! `index\nHH:MM:SS,mmm --> HH:MM:SS,mmm\ntext…` separated by blank
//! lines. `,` or `.` is accepted as the millisecond separator; times
//! are stored as `u64` milliseconds. Parsing is total (`None` on a
//! malformed block); indices are stored as-written (rewriters often
//! renumber, so strict sequencing is not enforced).
//!
//! [`emit`] writes the canonical `\r\n`-joined form; `parse(emit(x))`
//! reproduces `x` byte-for-byte in content.
//!
//! ```
//! use izanagi_kit::srt::{parse, Sub};
//!
//! let s = parse("1\n00:00:01,000 --> 00:00:02,500\nHello\n\n").unwrap();
//! assert_eq!(s[0].from, 1000);
//! assert_eq!(s[0].to, 2500);
//! ```

use std::string::String;
use std::vec::Vec;

/// One subtitle cue.
#[derive(Clone, Debug, PartialEq)]
pub struct Sub {
    /// Block index as written in the file.
    pub idx: u32,
    /// Start time, milliseconds.
    pub from: u64,
    /// End time, milliseconds.
    pub to: u64,
    /// Cue text (may contain newlines).
    pub text: String,
}

/// `HH:MM:SS,mmm` (or `.`) → milliseconds. `HH` may exceed 23.
pub fn parse_time(s: &str) -> Option<u64> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() < 9 {
        return None;
    }
    let two = |i: usize| -> Option<u64> {
        let a = b.get(i)?.checked_sub(b'0')?;
        let c = b.get(i + 1)?.checked_sub(b'0')?;
        if a > 9 || c > 9 {
            return None;
        }
        Some(a as u64 * 10 + c as u64)
    };
    let h: u64 = s.get(..s.len() - 10)?.parse().ok()?; // hours, any width
    if b[s.len() - 10] != b':' || b[s.len() - 7] != b':' {
        return None;
    }
    let m = two(s.len() - 9)?;
    let sec = two(s.len() - 6)?;
    if m > 59 || sec > 59 {
        return None;
    }
    let sep = b[s.len() - 4];
    if sep != b',' && sep != b'.' {
        return None;
    }
    let mut ms: u64 = 0;
    let mut digits = 0;
    for &c in &b[s.len() - 3..] {
        if !c.is_ascii_digit() {
            return None;
        }
        ms = ms * 10 + (c - b'0') as u64;
        digits += 1;
    }
    if digits != 3 {
        return None;
    }
    Some(((h * 60 + m) * 60 + sec) * 1000 + ms)
}

/// Milliseconds → canonical `HH:MM:SS,mmm` (two-digit hours min).
pub fn emit_time(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = ms / 60_000 % 60;
    let s = ms / 1000 % 60;
    let msec = ms % 1000;
    std::format!("{h:02}:{m:02}:{s:02},{msec:03}")
}

/// Parse the whole file; `None` if any block is malformed.
/// `from > to` is allowed through (broken files exist — degrade, not
/// fail); callers can filter.
pub fn parse(src: &str) -> Option<Vec<Sub>> {
    let mut out = Vec::new();
    // Split on blank lines; \r\n and \n both accepted.
    let norm = src.replace("\r\n", "\n");
    for block in norm.split("\n\n") {
        let b = block.trim_matches('\n');
        if b.is_empty() {
            continue;
        }
        let mut lines = b.lines();
        let idx: u32 = lines.next()?.trim().parse().ok()?;
        let tl = lines.next()?;
        let (a, t) = tl.split_once("-->")?;
        let from = parse_time(a)?;
        let to = parse_time(t.trim_start_matches(' '))?;
        let mut text = String::new();
        for (i, l) in lines.enumerate() {
            if i > 0 {
                text.push('\n');
            }
            text.push_str(l);
        }
        if text.is_empty() {
            return None;
        }
        out.push(Sub {
            idx,
            from,
            to,
            text,
        });
    }
    Some(out)
}

/// Canonical `\r\n` emission.
pub fn emit(subs: &[Sub]) -> String {
    let mut out = String::new();
    for s in subs {
        out.push_str(&std::format!(
            "{}\r\n{} --> {}\r\n{}\r\n\r\n",
            s.idx,
            emit_time(s.from),
            emit_time(s.to),
            s.text
        ));
    }
    out
}

/// Shift all cues by `ms` (clamped at 0).
pub fn shifted(subs: &[Sub], ms: i64) -> Vec<Sub> {
    subs.iter()
        .map(|s| Sub {
            idx: s.idx,
            from: (s.from as i64).saturating_add(ms).max(0) as u64,
            to: (s.to as i64).saturating_add(ms).max(0) as u64,
            text: s.text.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const S1: &str =
        "1\n00:00:01,000 --> 00:00:02,500\nHello\n\n2\n00:01:00.000 --> 00:01:01.000\nTwo\nlines\n\n";

    #[test]
    fn basic_parse() {
        let s = parse(S1).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].from, s[0].to), (1000, 2500));
        assert_eq!(s[1].text, "Two\nlines");
        assert_eq!(s[1].idx, 2);
    }

    #[test]
    fn emit_roundtrip() {
        let s = parse(S1).unwrap();
        let e = emit(&s);
        assert_eq!(parse(&e).unwrap(), s);
    }

    #[test]
    fn time_helpers() {
        assert_eq!(parse_time("00:00:00,000"), Some(0));
        assert_eq!(
            parse_time("99:59:59.999"),
            Some(99 * 3_600_000 + 59 * 60_000 + 59_999)
        );
        assert_eq!(emit_time(3_661_005), "01:01:01,005");
        assert_eq!(parse_time("00:60:00,000"), None);
        assert_eq!(parse_time("00:00:00,00"), None);
        assert_eq!(parse_time("xx"), None);
    }

    #[test]
    fn shift_clamps() {
        let s = parse(S1).unwrap();
        let sh = shifted(&s, -2000);
        assert_eq!(sh[0].from, 0);
        assert_eq!(sh[0].to, 500);
        assert_eq!(shifted(&s, 1000)[0].from, 2000);
    }

    #[test]
    fn malformed_rejected() {
        for bad in [
            "1\n",
            "1\n00:00:01,000\n",
            "x\n00:00:01,000 --> 00:00:02,000\nt\n",
            "1\n00:00:01,000 --> 00:00:02,000\n\n",
            "1\n00:00:01 --> 00:00:02,000\nt\n",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn empty_ok() {
        assert_eq!(parse(""), Some(vec![]));
        assert_eq!(parse("\n\n\n"), Some(vec![]));
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(S1), parse(S1));
        assert_eq!(emit(&parse(S1).unwrap()), emit(&parse(S1).unwrap()));
    }
}

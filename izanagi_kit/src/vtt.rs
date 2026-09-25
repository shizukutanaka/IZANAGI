//! WebVTT subtitles (W3C) — [`srt`](crate::srt)'s web sibling:
//! `WEBVTT` magic header, blank-line-separated blocks of `NOTE`,
//! `STYLE`, `REGION`, or cues `id\nt1 --> t2 settings\ntext`.
//! Timestamps are `HH:MM:SS.mmm` or `MM:SS.mmm` (dot mandatory);
//! cue settings (`align:`, `position:`, `line:`, `vertical:`) are kept
//! verbatim. Parsing is total; `emit` produces canonical output.
//!
//! ```
//! use izanagi_kit::vtt::{parse, emit};
//! let v = parse("WEBVTT\n\nc1\n00:00:01.000 --> 00:00:02.500\nHi\n").unwrap();
//! assert_eq!((v.cues[0].from, v.cues[0].to), (1000, 2500));
//! assert_eq!(emit(&v), "WEBVTT\n\nc1\n00:00:01.000 --> 00:00:02.500\nHi\n\n");
//! ```

use std::string::String;
use std::vec::Vec;

/// One WebVTT cue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    /// Optional cue identifier (empty when absent).
    pub id: String,
    /// Start time in milliseconds.
    pub from: u64,
    /// End time in milliseconds.
    pub to: u64,
    /// Cue settings verbatim (`"align:start position:0%"` etc.).
    pub settings: String,
    /// Payload lines (verbatim, may contain `<` tags).
    pub text: String,
}

/// A parsed `.vtt` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vtt {
    /// Non-cue blocks (`NOTE`, `STYLE`, `REGION`) verbatim.
    pub blocks: Vec<String>,
    /// Cues in file order.
    pub cues: Vec<Cue>,
}

/// `HH:MM:SS.mmm` or `MM:SS.mmm` → ms; `None` on any deviation.
pub fn parse_time(s: &str) -> Option<u64> {
    let (hms, ms) = s.split_once('.')?;
    if ms.len() != 3 || !ms.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let ms: u64 = ms.parse().ok()?;
    let mut it = hms.splitn(3, ':');
    let (h, m, sec) = match (it.next(), it.next(), it.next()) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        (Some(a), Some(b), None) => ("0", a, b),
        _ => return None,
    };
    if h.is_empty() || !h.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if m.len() != 2 || sec.len() != 2 {
        return None;
    }
    let m: u64 = m.parse().ok()?;
    let sec: u64 = sec.parse().ok()?;
    let h: u64 = h.parse().ok()?;
    if m > 59 || sec > 59 {
        return None;
    }
    Some(h * 3_600_000 + m * 60_000 + sec * 1000 + ms)
}

/// Canonical `HH:MM:SS.mmm` (hours are at least two digits).
pub fn emit_time(t: u64) -> String {
    std::format!(
        "{:02}:{:02}:{:02}.{:03}",
        t / 3_600_000,
        (t / 60_000) % 60,
        (t / 1000) % 60,
        t % 1000
    )
}

/// Parse a `.vtt` file; `None` on bad magic or a malformed cue.
/// Unknown/unsupported blocks become verbatim [`Vtt::blocks`].
pub fn parse(src: &str) -> Option<Vtt> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut head = src.splitn(2, '\n');
    let l0 = head.next()?.trim_end();
    // signature: `WEBVTT` optionally followed by space/tab text
    if l0 != "WEBVTT" && !l0.starts_with("WEBVTT ") && !l0.starts_with("WEBVTT\t") {
        return None;
    }
    let body = head.next().unwrap_or("").replace("\r\n", "\n");
    let mut v = Vtt {
        blocks: Vec::new(),
        cues: Vec::new(),
    };
    for block in body.split("\n\n") {
        let block = block.trim_matches('\n');
        if block.is_empty() {
            continue;
        }
        if block.starts_with("NOTE") || block.starts_with("STYLE") || block.starts_with("REGION") {
            v.blocks.push(block.to_string());
            continue;
        }
        // cue: [id]\ntiming\ntext
        let mut lines = block.lines();
        let first = lines.next()?;
        let (id, timing) = if first.contains("-->") {
            (String::new(), first)
        } else {
            let t = lines.next()?;
            if !t.contains("-->") {
                v.blocks.push(block.to_string());
                continue;
            }
            (first.to_string(), t)
        };
        let (a, t) = timing.split_once("-->")?;
        let from = parse_time(a.trim())?;
        let rest = t.trim_start();
        let mut it = rest.splitn(2, [' ', '\t']);
        let to = parse_time(it.next()?)?;
        let settings = it.next().unwrap_or("").to_string();
        let mut text = String::new();
        for (i, l) in lines.enumerate() {
            if i > 0 {
                text.push('\n');
            }
            text.push_str(l);
        }
        v.cues.push(Cue {
            id,
            from,
            to,
            settings,
            text,
        });
    }
    Some(v)
}

/// Canonical emission: `WEBVTT\n\n` + blocks + cues (CRLF-free; VTT
/// accepts LF).
pub fn emit(v: &Vtt) -> String {
    let mut s = String::from("WEBVTT\n\n");
    for b in &v.blocks {
        s.push_str(b);
        s.push_str("\n\n");
    }
    for c in &v.cues {
        if !c.id.is_empty() {
            s.push_str(&c.id);
            s.push('\n');
        }
        s.push_str(&emit_time(c.from));
        s.push_str(" --> ");
        s.push_str(&emit_time(c.to));
        if !c.settings.is_empty() {
            s.push(' ');
            s.push_str(&c.settings);
        }
        s.push('\n');
        s.push_str(&c.text);
        s.push_str("\n\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "WEBVTT - header\n\nNOTE this is a comment\n\nSTYLE\n::cue { color: lime }\n\nintro\n00:00:00.000 --> 00:00:01.500 align:start\nHello <b>world</b>\n\n00:01.000 --> 00:02.000 position:10%\nShort form\n\n";

    #[test]
    fn basic_parse() {
        let v = parse(DOC).unwrap();
        assert_eq!(v.blocks.len(), 2);
        assert_eq!(v.cues.len(), 2);
        assert_eq!(v.cues[0].id, "intro");
        assert_eq!(v.cues[0].settings, "align:start");
        assert_eq!(v.cues[0].text, "Hello <b>world</b>");
        assert_eq!(v.cues[1].from, 1000);
        assert_eq!(v.cues[1].to, 2000);
    }

    #[test]
    fn times() {
        assert_eq!(parse_time("00:00:01.000"), Some(1000));
        assert_eq!(parse_time("01:02.500"), Some(62_500));
        assert_eq!(parse_time("1:00:00.000"), Some(3_600_000));
        assert_eq!(parse_time("00:60:00.000"), None);
        assert_eq!(parse_time("00:00:00,000"), None); // comma is srt
        assert_eq!(parse_time("00:00:00.00"), None);
        assert_eq!(emit_time(3_661_005), "01:01:01.005");
    }

    #[test]
    fn emit_roundtrip() {
        let v = parse(DOC).unwrap();
        assert_eq!(parse(&emit(&v)).unwrap(), v);
    }

    #[test]
    fn bom_and_crlf() {
        let v = parse("\u{feff}WEBVTT\r\n\r\n00:00:00.000 --> 00:00:01.000\r\nx\r\n").unwrap();
        assert_eq!(v.cues.len(), 1);
        assert_eq!(v.cues[0].text, "x");
    }

    #[test]
    fn malformed_rejected() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("WEBVT"), None);
        assert_eq!(parse("WEBVTT\n\n00:00:00.000 --> xx\ny"), None);
        assert_eq!(parse("WEBVTT\n\n--> 00:00:01.000\ny"), None);
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(DOC), parse(DOC));
        assert_eq!(emit(&parse(DOC).unwrap()), emit(&parse(DOC).unwrap()));
    }
}

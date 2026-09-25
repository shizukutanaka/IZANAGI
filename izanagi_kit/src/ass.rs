//! SSA/ASS subtitles — [`srt`](crate::srt)/[`vtt`](crate::vtt)'s
//! advanced sibling: `[Script Info]` key-values, `[V4+ Styles]` and
//! `[Events]` sections whose rows are mapped by each section's
//! `Format:` column list. Times are `H:MM:SS.cc` centiseconds.
//! Unrecognised sections are kept verbatim in [`Ass::other`]; the
//! final `Text` column swallows the rest of the line (commas allowed).
//!
//! ```
//! use izanagi_kit::ass::{parse, ass_time, EventKind};
//! let a = parse("[Events]\nFormat: Layer, Start, End, Text\nDialogue: 0,0:00:01.00,0:00:02.50,Hello\n").unwrap();
//! assert_eq!(a.events[0].kind, EventKind::Dialogue);
//! assert_eq!(a.events[0].fields["Start"], "0:00:01.00");
//! assert_eq!(ass_time("0:00:01.00"), Some(1000));
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// Event row kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// Rendered dialogue.
    Dialogue,
    /// Non-rendered comment.
    Comment,
}

/// One `[Events]` row resolved through the section's `Format:`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// Dialogue or comment.
    pub kind: EventKind,
    /// Column → value (the trailing `Text` column keeps commas).
    pub fields: BTreeMap<String, String>,
}

impl Event {
    /// `Start` field as milliseconds.
    pub fn start(&self) -> Option<u64> {
        ass_time(self.fields.get("Start")?)
    }
    /// `End` field as milliseconds.
    pub fn end(&self) -> Option<u64> {
        ass_time(self.fields.get("End")?)
    }
    /// `Text` field verbatim (`\N` newlines kept).
    pub fn text(&self) -> Option<&str> {
        self.fields.get("Text").map(|s| s.as_str())
    }
}

/// A parsed `.ass`/`.ssa` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Ass {
    /// `[Script Info]` `key: value` pairs, in order.
    pub info: Vec<(String, String)>,
    /// `[V4+ Styles]` `Format:` columns.
    pub style_format: Vec<String>,
    /// `[V4+ Styles]` `Style:` rows resolved through `style_format`.
    pub styles: Vec<BTreeMap<String, String>>,
    /// `[Events]` `Format:` columns.
    pub event_format: Vec<String>,
    /// `[Events]` `Dialogue:`/`Comment:` rows.
    pub events: Vec<Event>,
    /// Unrecognised sections verbatim (`(name, lines)`).
    pub other: Vec<(String, Vec<String>)>,
}

/// `H:MM:SS.cc` → ms; centiseconds are exactly two digits.
pub fn ass_time(s: &str) -> Option<u64> {
    let (hms, cs) = s.split_once('.')?;
    if cs.len() != 2 || !cs.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let cs: u64 = cs.parse().ok()?;
    let mut it = hms.splitn(3, ':');
    let (h, m, sec): (&str, &str, &str) = match (it.next(), it.next(), it.next()) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => return None,
    };
    if m.len() != 2 || sec.len() != 2 {
        return None;
    }
    let h: u64 = h.parse().ok()?;
    let m: u64 = m.parse().ok()?;
    let sec: u64 = sec.parse().ok()?;
    if m > 59 || sec > 59 {
        return None;
    }
    Some(h * 3_600_000 + m * 60_000 + sec * 1000 + cs * 10)
}

/// Canonical `H:MM:SS.cc` (hours unpadded: `0:01:01.00`).
pub fn emit_time(t: u64) -> String {
    std::format!(
        "{}:{:02}:{:02}.{:02}",
        t / 3_600_000,
        (t / 60_000) % 60,
        (t / 1000) % 60,
        (t % 1000) / 10
    )
}

fn split_fields(fmt: &[String], row: &str) -> Option<BTreeMap<String, String>> {
    let mut m = BTreeMap::new();
    if fmt.is_empty() {
        return None;
    }
    // all but the last column split at ','
    let mut rest = row;
    for (i, col) in fmt.iter().enumerate() {
        if i + 1 == fmt.len() {
            m.insert(col.clone(), rest.to_string());
        } else {
            let (v, r) = rest.split_once(',')?;
            m.insert(col.clone(), v.trim().to_string());
            rest = r;
        }
    }
    Some(m)
}

fn cols(s: &str) -> Vec<String> {
    s.split(',').map(|c| c.trim().to_string()).collect()
}

/// Parse an ASS file; `None` when `[Events]` or its `Format:` is
/// missing, or a row has fewer commas than columns.
pub fn parse(src: &str) -> Option<Ass> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let src = src.replace("\r\n", "\n");
    let mut a = Ass {
        info: Vec::new(),
        style_format: Vec::new(),
        styles: Vec::new(),
        event_format: Vec::new(),
        events: Vec::new(),
        other: Vec::new(),
    };
    let mut section = String::new();
    for raw in src.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() || line.trim().starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            if !matches!(
                section.as_str(),
                "Script Info" | "V4+ Styles" | "V4 Styles" | "Events"
            ) {
                a.other.push((section.clone(), Vec::new()));
            }
            continue;
        }
        match section.as_str() {
            "Script Info" => {
                if let Some((k, v)) = line.split_once(':') {
                    a.info.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
            "V4+ Styles" | "V4 Styles" => {
                if let Some(v) = line.split_once(':') {
                    match v.0.trim() {
                        "Format" => a.style_format = cols(v.1),
                        "Style" => {
                            if let Some(m) = split_fields(&a.style_format, v.1.trim()) {
                                a.styles.push(m);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "Events" => {
                if let Some((k, v)) = line.split_once(':') {
                    match k.trim() {
                        "Format" => a.event_format = cols(v),
                        "Dialogue" | "Comment" => {
                            let m = split_fields(&a.event_format, v.trim())?;
                            a.events.push(Event {
                                kind: if k.trim() == "Dialogue" {
                                    EventKind::Dialogue
                                } else {
                                    EventKind::Comment
                                },
                                fields: m,
                            });
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                if let Some(last) = a.other.last_mut() {
                    last.1.push(line.to_string());
                }
            }
        }
    }
    if a.event_format.is_empty() {
        return None;
    }
    Some(a)
}

/// Canonical emission (sections in fixed order, `Format:` re-emitted).
pub fn emit(a: &Ass) -> String {
    let mut s = String::new();
    if !a.info.is_empty() {
        s.push_str("[Script Info]\n");
        for (k, v) in &a.info {
            s.push_str(k);
            s.push_str(": ");
            s.push_str(v);
            s.push('\n');
        }
        s.push('\n');
    }
    if !a.style_format.is_empty() || !a.styles.is_empty() {
        s.push_str("[V4+ Styles]\n");
        if !a.style_format.is_empty() {
            s.push_str(&std::format!("Format: {}\n", a.style_format.join(", ")));
            for st in &a.styles {
                let row: Vec<&str> = a
                    .style_format
                    .iter()
                    .map(|c| st.get(c).map(|x| x.as_str()).unwrap_or(""))
                    .collect();
                s.push_str(&std::format!("Style: {}\n", row.join(",")));
            }
        }
        s.push('\n');
    }
    if !a.event_format.is_empty() {
        s.push_str("[Events]\n");
        s.push_str(&std::format!("Format: {}\n", a.event_format.join(", ")));
        for e in &a.events {
            let row: Vec<&str> = a
                .event_format
                .iter()
                .map(|c| e.fields.get(c).map(|x| x.as_str()).unwrap_or(""))
                .collect();
            s.push_str(&std::format!(
                "{}: {}\n",
                match e.kind {
                    EventKind::Dialogue => "Dialogue",
                    EventKind::Comment => "Comment",
                },
                row.join(",")
            ));
        }
        s.push('\n');
    }
    for (name, lines) in &a.other {
        s.push_str(&std::format!("[{name}]\n"));
        for l in lines {
            s.push_str(l);
            s.push('\n');
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "[Script Info]\nTitle: Demo\nScriptType: v4.00+\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour\nStyle: Default,Arial,20,&H00FFFFFF\n\n[Events]\nFormat: Layer, Start, End, Style, Text\nDialogue: 0,0:00:01.00,0:00:02.50,Default,Hello, world\nComment: 0,0:00:03.00,0:00:04.00,Default,note\n";

    #[test]
    fn basic_parse() {
        let a = parse(DOC).unwrap();
        assert_eq!(a.info[0], ("Title".into(), "Demo".into()));
        assert_eq!(a.styles.len(), 1);
        assert_eq!(a.styles[0]["Name"], "Default");
        assert_eq!(a.events.len(), 2);
        assert_eq!(a.events[0].kind, EventKind::Dialogue);
        assert_eq!(a.events[1].kind, EventKind::Comment);
        assert_eq!(a.events[0].start(), Some(1000));
        assert_eq!(a.events[0].end(), Some(2500));
        assert_eq!(a.events[0].text(), Some("Hello, world")); // comma in Text
    }

    #[test]
    fn times() {
        assert_eq!(ass_time("0:00:00.00"), Some(0));
        assert_eq!(ass_time("1:02:03.45"), Some(3_723_450));
        assert_eq!(ass_time("0:60:00.00"), None);
        assert_eq!(ass_time("00:00:00,00"), None);
        assert_eq!(ass_time("0:00:00.0"), None);
        assert_eq!(emit_time(3_723_450), "1:02:03.45");
    }

    #[test]
    fn emit_roundtrip() {
        let a = parse(DOC).unwrap();
        assert_eq!(parse(&emit(&a)).unwrap(), a);
    }

    #[test]
    fn other_sections_kept() {
        let a = parse("[Fonts]\nfont1.ttf\n\n[Events]\nFormat: Start, Text\n").unwrap();
        assert_eq!(a.other[0].0, "Fonts");
        assert_eq!(a.other[0].1, vec!["font1.ttf"]);
        assert!(a.events.is_empty());
    }

    #[test]
    fn malformed_rejected() {
        assert_eq!(parse(""), None); // no events section
        assert_eq!(parse("[Events]\n"), None); // no Format
        assert_eq!(
            parse("[Events]\nFormat: Start, Text\nDialogue: 0:00:01.00\n"),
            None
        ); // row short
           // comma-bearing Text works
        assert!(parse("[Events]\nFormat: Start, Text\nDialogue: 0:00:01.00,x,y\n").is_some());
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(DOC), parse(DOC));
        assert_eq!(emit(&parse(DOC).unwrap()), emit(&parse(DOC).unwrap()));
    }
}

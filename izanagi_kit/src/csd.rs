//! Csound `.csd` — XML-ish wrapper `<CsoundSynthesizer>` containing
//! `<CsOptions>`, `<CsInstruments>`, `<CsScore>`, `<CsLicense>` sections.
//!
//! ```
//! let d = b"<CsoundSynthesizer>\n<CsOptions>\n-o dac\n</CsOptions>\n<CsInstruments>\ninstr 1\nendin\n</CsInstruments>\n<CsScore>\ni 1 0 1\n</CsScore>\n</CsoundSynthesizer>\n";
//! let c = izanagi_kit::csd::parse(d).unwrap();
//! assert_eq!(c.instruments, 1);
//! assert_eq!(c.score_events, 1);
//! assert_eq!(c.options, 1);
//! assert!(izanagi_kit::csd::detect(d));
//! ```

/// A parsed Csound `.csd` file.
#[derive(Debug, Clone)]
pub struct Csd {
    /// `instr N…endin` blocks inside `<CsInstruments>`.
    pub instruments: usize,
    /// Non-blank lines inside `<CsScore>` (i/e/f/p statements).
    pub score_events: usize,
    /// Non-blank lines inside `<CsOptions>`.
    pub options: usize,
    /// Present sections found.
    pub has_license: bool,
    /// `<CsVersion>` seen.
    pub has_version: bool,
    /// `opcode…endop` user-defined opcodes.
    pub opcodes: usize,
    /// `xmlstrict`/`html`-looking stray tags that are not recognized.
    pub stray_tags: usize,
    /// Orchestra comment lines (`;`).
    pub comment_lines: usize,
}

const SECTIONS: &[&str] = &[
    "CsInstruments",
    "CsScore",
    "CsOptions",
    "CsLicense",
    "CsVersion",
    "CsHtml5",
];

fn section_body<'a>(t: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let i = t.find(&open)?;
    let body = &t[i + open.len()..];
    let j = body.find(&close)?;
    Some(&body[..j])
}

fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Detects a `.csd` file: `<CsoundSynthesizer>` root + a known section.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_comments(t);
    t.contains("<CsoundSynthesizer>") && SECTIONS.iter().any(|s| t.contains(&format!("<{s}>")))
}

/// Parses a `.csd`; `None` on non-UTF-8 or missing root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Csd> {
    if !detect(b) {
        return None;
    }
    let t = strip_comments(std::str::from_utf8(b).ok()?);
    let mut s = Csd {
        instruments: 0,
        score_events: 0,
        options: 0,
        has_license: section_body(&t, "CsLicense").is_some(),
        has_version: section_body(&t, "CsVersion").is_some(),
        opcodes: 0,
        stray_tags: 0,
        comment_lines: 0,
    };
    if let Some(inst) = section_body(&t, "CsInstruments") {
        for line in inst.lines() {
            let l = line.trim();
            if l.starts_with(';') || l.starts_with("//") {
                s.comment_lines += 1;
            }
            if l.starts_with("instr ") {
                s.instruments += 1;
            }
            if l.starts_with("opcode ") {
                s.opcodes += 1;
            }
        }
    }
    if let Some(score) = section_body(&t, "CsScore") {
        s.score_events = score
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with(';'))
            .count();
    }
    if let Some(opts) = section_body(&t, "CsOptions") {
        s.options = opts
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .count();
    }
    // tags that aren't ours
    for (i, _) in t.match_indices('<') {
        let tail = &t[i..];
        if tail.starts_with("<!--") {
            continue;
        }
        let name: String = tail[1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        let known = ["CsoundSynthesizer", "CsInstrumentS"];
        if !name.is_empty()
            && !SECTIONS.contains(&name.as_str())
            && !known.contains(&name.as_str())
            && name != "/CsoundSynthesizer"
        {
            s.stray_tags += 1;
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<CsoundSynthesizer>\n<CsOptions>\n-o dac\n-d\n</CsOptions>\n<CsInstruments>\n; sine\ninstr 1\nendin\nopcode op 0\nendop\ninstr 2\nendin\n</CsInstruments>\n<CsScore>\ni 1 0 1\nf 0 2\n</CsScore>\n</CsoundSynthesizer>\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.instruments, 2);
        assert_eq!(s.opcodes, 1);
        assert_eq!(s.score_events, 2);
        assert_eq!(s.options, 2);
        assert_eq!(s.comment_lines, 1);
        assert!(!s.has_license);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(
            b"<CsoundSynthesizer><CsScore></CsScore></CsoundSynthesizer>"
        ));
        assert!(!detect(b"<xml/>"));
        assert!(!detect(b"instr 1"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<CsoundSynthesizer/>").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}

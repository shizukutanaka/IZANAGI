//! iCalendar (RFC 5545) — `BEGIN:VCALENDAR` … `VEVENT`/`VTODO`
//! components. Same folding rules as [`vcf`](crate::vcf); properties
//! are `NAME[;params]:value`. [`parse_dt`] turns `DTSTART`/`DTEND`
//! (`YYYYMMDDTHHMMSS[Z]`) into `(days_from_civil, seconds)` — the
//! [`cron`](crate::cron)/[`civil`](crate::civil) calendar sibling.
//! Components nested inside another (e.g. `VALARM`) are preserved
//! verbatim as [`MARKER`] properties on their parent.
//!
//! ```
//! use izanagi_kit::ics::{parse, get_prop, parse_dt};
//! let cal = parse("BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nDTSTART:20240101T090000Z\nEND:VEVENT\nEND:VCALENDAR\n").unwrap();
//! let dt = parse_dt(get_prop(&cal.components[0], "DTSTART").unwrap()).unwrap();
//! assert_eq!(dt.1, 9 * 3600);
//! ```

use std::string::String;
use std::vec::Vec;

/// Property key under which nested-component raw lines (`BEGIN:…`,
/// inner props, `END:…`) are stored on their parent component.
pub const MARKER: &str = "X-IC-MARKER";

/// One component (`VEVENT`, `VTODO`, …).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Component {
    /// Upper-case kind (`"VEVENT"`).
    pub kind: String,
    /// `(NAME\[;params\], value)` in order; nested component lines
    /// appear verbatim under [`MARKER`].
    pub props: Vec<(String, String)>,
}

/// A parsed calendar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ics {
    /// Calendar-level properties (`VERSION`, `PRODID`, …).
    pub props: Vec<(String, String)>,
    /// Top-level components in order.
    pub components: Vec<Component>,
}

impl Ics {
    /// Components of the given kind.
    pub fn of_kind<'a>(&'a self, kind: &str) -> Vec<&'a Component> {
        self.components.iter().filter(|c| c.kind == kind).collect()
    }
}

/// First value of a property (ignores `;params`).
pub fn get_prop<'a>(c: &'a Component, name: &str) -> Option<&'a str> {
    c.props
        .iter()
        .find(|(k, _)| {
            let (n, _) = k.split_once(';').unwrap_or((k.as_str(), ""));
            n.eq_ignore_ascii_case(name)
        })
        .map(|(_, v)| v.as_str())
}

/// `YYYYMMDD` or `YYYYMMDDTHHMMSS[Z]` → `(days_from_civil, secs)`.
/// `secs` is 0 for date-only forms.
pub fn parse_dt(s: &str) -> Option<(i32, u32)> {
    let b = s.as_bytes();
    if b.len() != 8 && !(b.len() == 15 || (b.len() == 16 && b[15] == b'Z')) {
        return None;
    }
    if b.len() >= 15 && b[8] != b'T' {
        return None;
    }
    let num = |i: usize, n: usize| -> Option<u32> {
        let s = std::str::from_utf8(&b[i..i + n]).ok()?;
        if !s.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        s.parse().ok()
    };
    let (y, m, d) = (num(0, 4)? as i32, num(4, 2)?, num(6, 2)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let days = crate::civil::days_from_civil(y, m, d);
    if b.len() == 8 {
        return Some((days, 0));
    }
    let (hh, mm, ss) = (num(9, 2)?, num(11, 2)?, num(13, 2)?);
    if hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    Some((days, hh * 3600 + mm * 60 + ss))
}

/// `(days, secs)` → `YYYYMMDDTHHMMSSZ` (UTC form).
pub fn emit_dt(days: i32, secs: u32) -> String {
    let (y, m, d) = crate::civil::civil_from_days(days);
    std::format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        y,
        m,
        d,
        secs / 3600,
        (secs / 60) % 60,
        secs % 60
    )
}

fn unfold(src: &str) -> Vec<String> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut out: Vec<String> = Vec::new();
    for l in src.lines() {
        if l.starts_with(' ') || l.starts_with('\t') {
            match out.last_mut() {
                Some(prev) => prev.push_str(&l[1..]),
                None => out.push(l.to_string()),
            }
        } else {
            out.push(l.to_string());
        }
    }
    out
}

/// Parse a `VCALENDAR`; `None` on unbalanced `BEGIN`/`END`, missing
/// `VERSION`, stray content, or colon-less lines.
pub fn parse(src: &str) -> Option<Ics> {
    let mut cal = Ics::default();
    let mut in_cal = false;
    let mut cur: Option<Component> = None;
    let mut depth = 0usize; // nested components inside `cur`
    let mut has_ver = false;
    for l in unfold(src) {
        if l.is_empty() {
            continue;
        }
        let up = l.to_uppercase();
        if let Some(rest) = up.strip_prefix("BEGIN:") {
            let name = rest.trim();
            if name == "VCALENDAR" {
                if in_cal || cur.is_some() {
                    return None;
                }
                in_cal = true;
                continue;
            }
            if !in_cal {
                return None;
            }
            match &mut cur {
                Some(c) => {
                    // nested component → verbatim marker line
                    c.props.push((MARKER.to_string(), l.clone()));
                    depth += 1;
                }
                None => {
                    cur = Some(Component {
                        kind: name.to_string(),
                        props: Vec::new(),
                    });
                }
            }
            continue;
        }
        if let Some(rest) = up.strip_prefix("END:") {
            let name = rest.trim();
            if name == "VCALENDAR" {
                if cur.is_some() || !in_cal {
                    return None;
                }
                in_cal = false;
                if !has_ver {
                    return None;
                }
                continue;
            }
            match &mut cur {
                Some(c) if depth > 0 => {
                    c.props.push((MARKER.to_string(), l.clone()));
                    depth -= 1;
                }
                Some(c) => {
                    if c.kind != name {
                        return None;
                    }
                    cal.components.push(cur.take()?);
                }
                None => return None,
            }
            continue;
        }
        if !in_cal {
            return None;
        }
        let (k, val) = l.split_once(':')?;
        let (n, _) = k.split_once(';').unwrap_or((k, ""));
        if n.is_empty() {
            return None;
        }
        match &mut cur {
            Some(c) if depth > 0 => c.props.push((MARKER.to_string(), l.clone())),
            Some(c) => {
                if n.eq_ignore_ascii_case("VERSION") {
                    has_ver = true;
                }
                c.props.push((k.to_string(), val.to_string()));
            }
            None => {
                if n.eq_ignore_ascii_case("VERSION") {
                    has_ver = true;
                }
                cal.props.push((k.to_string(), val.to_string()));
            }
        }
    }
    if in_cal || cur.is_some() {
        return None;
    }
    if cal.props.is_empty() && cal.components.is_empty() {
        return None;
    }
    Some(cal)
}

/// Canonical emission.
pub fn emit(cal: &Ics) -> String {
    let mut s = String::from("BEGIN:VCALENDAR\n");
    let prop = |s: &mut String, k: &str, v: &str| {
        if k == MARKER {
            s.push_str(v);
        } else {
            s.push_str(k);
            s.push(':');
            s.push_str(v);
        }
        s.push('\n');
    };
    for (k, v) in &cal.props {
        prop(&mut s, k, v);
    }
    for c in &cal.components {
        s.push_str(&std::format!("BEGIN:{}\n", c.kind));
        for (k, v) in &c.props {
            prop(&mut s, k, v);
        }
        s.push_str(&std::format!("END:{}\n", c.kind));
    }
    s.push_str("END:VCALENDAR\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//x//y//EN\nBEGIN:VEVENT\nUID:u1\nDTSTART:20240101T090000Z\nDTEND;TZID=UTC:20240101T100000Z\nSUMMARY:Meet\nBEGIN:VALARM\nACTION:DISPLAY\nEND:VALARM\nEND:VEVENT\nBEGIN:VTODO\nUID:t1\nEND:VTODO\nEND:VCALENDAR\n";

    #[test]
    fn basic_parse() {
        let cal = parse(DOC).unwrap();
        assert_eq!(cal.components.len(), 2);
        let e = &cal.of_kind("VEVENT")[0];
        assert_eq!(get_prop(e, "UID"), Some("u1"));
        assert_eq!(get_prop(e, "SUMMARY"), Some("Meet"));
        // param form of DTEND resolves too
        assert_eq!(get_prop(e, "DTEND"), Some("20240101T100000Z"));
        // VALARM lines preserved verbatim
        let marks: Vec<&str> = e
            .props
            .iter()
            .filter(|(k, _)| k == MARKER)
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(marks, vec!["BEGIN:VALARM", "ACTION:DISPLAY", "END:VALARM"]);
        assert_eq!(cal.of_kind("VTODO").len(), 1);
        assert_eq!(cal.of_kind("NOPE").len(), 0);
    }

    #[test]
    fn dt_roundtrip() {
        let (d, s) = parse_dt("20240101T090000Z").unwrap();
        assert_eq!(s, 32400);
        assert_eq!(emit_dt(d, s), "20240101T090000Z");
        assert_eq!(parse_dt("20240101").unwrap().1, 0);
        assert_eq!(parse_dt("20241301T090000Z"), None);
        assert_eq!(parse_dt("20240101T240000Z"), None);
        assert_eq!(parse_dt("2024010"), None);
        assert_eq!(parse_dt("junk"), None);
        // boundary: leap-day + day length
        assert!(parse_dt("20240229T235959").is_some());
        assert_eq!(parse_dt("20240230"), Some((d_feb30(), 0))); // civil lenient on day
    }

    fn d_feb30() -> i32 {
        // civil days_from_civil doesn't validate d <= days_in_month;
        // 2024-02-30 rolls into March 1 — document the leniency.
        crate::civil::days_from_civil(2024, 2, 30)
    }

    #[test]
    fn strictness() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("BEGIN:VCALENDAR\n"), None);
        assert_eq!(parse("BEGIN:VCALENDAR\nPRODID:x\nEND:VCALENDAR\n"), None);
        assert_eq!(
            parse("BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nEND:VTODO\nEND:VCALENDAR\n"),
            None
        );
        assert_eq!(parse("X:1\n"), None);
        assert_eq!(
            parse("BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nEND:VCALENDAR\nEND:VCALENDAR\n"),
            None
        );
    }

    #[test]
    fn emit_roundtrip() {
        let cal = parse(DOC).unwrap();
        let cal2 = parse(&emit(&cal)).unwrap();
        assert_eq!(cal, cal2);
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(DOC), parse(DOC));
        assert_eq!(emit(&parse(DOC).unwrap()), emit(&parse(DOC).unwrap()));
    }
}

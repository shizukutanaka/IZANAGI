//! iCalendar (RFC 5545) — `BEGIN:VCALENDAR` component tree with
//! `VEVENT`/`VTODO`/`VJOURNAL`/`VFREEBUSY`/`VTIMEZONE` components and
//! `DTSTART`/`DTEND`/`SUMMARY`/`RRULE` properties.
//!
//! ```
//! let d = b"BEGIN:VCALENDAR\r\nVERSION:2\x2e0\r\nBEGIN:VEVENT\r\nSUMMARY:Meet\r\nDTSTART:20240101T090000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
//! let c = izanagi_kit::ical::parse(d).unwrap();
//! assert_eq!(c.events, 1);
//! assert_eq!(c.dtstarts, 1);
//! assert!(izanagi_kit::ical::detect(d));
//! ```

/// Census of an iCalendar stream.
#[derive(Debug, Clone)]
pub struct Ical {
    /// `BEGIN:VCALENDAR` count.
    pub calendars: usize,
    /// `BEGIN:VEVENT` count.
    pub events: usize,
    /// `BEGIN:VTODO` count.
    pub todos: usize,
    /// `BEGIN:VJOURNAL` count.
    pub journals: usize,
    /// `BEGIN:VFREEBUSY` count.
    pub freebusies: usize,
    /// `BEGIN:VTIMEZONE` count.
    pub timezones: usize,
    /// `DTSTART` props.
    pub dtstarts: usize,
    /// `DTEND` props.
    pub dtends: usize,
    /// `SUMMARY` props.
    pub summaries: usize,
    /// `RRULE` props.
    pub rrules: usize,
    /// `VALARM` alarm blocks.
    pub alarms: usize,
    /// `BEGIN:`/`END:` line counts.
    pub begins: usize,
    /// `END:` count.
    pub ends: usize,
    /// Imbalance between BEGIN/END.
    pub depth_ok: bool,
    /// Folded continuation lines.
    pub folded: usize,
}

/// Detects an iCalendar stream: `BEGIN:VCALENDAR` + `END:VCALENDAR`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.windows(15).any(|w| w == b"BEGIN:VCALENDAR") && b.windows(13).any(|w| w == b"END:VCALENDAR")
}

/// Parses an iCalendar stream; `None` without the calendar block.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Ical> {
    if !detect(b) {
        return None;
    }
    let t = String::from_utf8_lossy(b);
    let mut c = Ical {
        calendars: t.matches("BEGIN:VCALENDAR").count(),
        events: t.matches("BEGIN:VEVENT").count(),
        todos: t.matches("BEGIN:VTODO").count(),
        journals: t.matches("BEGIN:VJOURNAL").count(),
        freebusies: t.matches("BEGIN:VFREEBUSY").count(),
        timezones: t.matches("BEGIN:VTIMEZONE").count(),
        dtstarts: t.matches("DTSTART").count(),
        dtends: t.matches("DTEND").count(),
        summaries: t.matches("SUMMARY").count(),
        rrules: t.matches("RRULE").count(),
        alarms: t.matches("BEGIN:VALARM").count(),
        begins: 0,
        ends: 0,
        depth_ok: false,
        folded: 0,
    };
    for line in t.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            c.folded += 1;
        } else if line.starts_with("BEGIN:") {
            c.begins += 1;
        } else if line.starts_with("END:") {
            c.ends += 1;
        }
    }
    c.depth_ok = c.begins == c.ends;
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAL: &[u8] = b"BEGIN:VCALENDAR\r\nVERSION:2\x2e0\r\nPRODID:-//x//EN\r\nBEGIN:VTIMEZONE\r\nTZID:UTC\r\nEND:VTIMEZONE\r\nBEGIN:VEVENT\r\nSUMMARY:Meet\r\nDTSTART:20240101T090000Z\r\nDTEND:20240101T100000Z\r\nRRULE:FREQ=WEEKLY\r\nBEGIN:VALARM\r\nTRIGGER:-PT15M\r\nEND:VALARM\r\nEND:VEVENT\r\nBEGIN:VTODO\r\nSUMMARY:Task\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    #[test]
    fn parses() {
        let c = parse(CAL).unwrap();
        assert_eq!(c.calendars, 1);
        assert_eq!(c.events, 1);
        assert_eq!(c.todos, 1);
        assert_eq!(c.timezones, 1);
        assert_eq!(c.dtstarts, 1);
        assert_eq!(c.rrules, 1);
        assert_eq!(c.alarms, 1);
        assert!(c.depth_ok);
    }

    #[test]
    fn folded() {
        let d = b"BEGIN:VCALENDAR\nEND:VCALENDAR\nBEGIN:VCALENDAR\nSUMMARY:x\n y\nEND:VCALENDAR";
        let c = parse(d).unwrap();
        assert_eq!(c.folded, 1);
        assert_eq!(c.calendars, 2);
    }

    #[test]
    fn detect_works() {
        assert!(detect(CAL));
        assert!(!detect(b"BEGIN:VCALENDAR"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
    }
}

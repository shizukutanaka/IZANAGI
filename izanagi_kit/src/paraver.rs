//! Paraver trace (`.prv`) parser — BSC performance-analysis traces.
//!
//! Detects the `#Paraver (dd/mm/yy at hh:mm):<duration>:…` header and
//! counts body records by type field (`1:` state, `2:` event,
//! `3:` communication, `4:` global OP), plus `#Paraver` config blocks
//! and plain comment lines.
//!
//! ```
//! let b = br#"#Paraver (19/02/2010 at 16:22):5000000010:1(1):1:1(1:1)
//! #Paraver CFG
//! 2:0:1:1:1:0:1000000:9
//! 1:0:1:1:1:0:999999:1
//! 3:0:1:1:1:1:2:1:1:1:0:100:200:500
//! "#;
//! assert!(izanagi_kit::paraver::detect(b));
//! let c = izanagi_kit::paraver::Paraver::parse(b).unwrap();
//! assert_eq!(c.records, 3);
//! assert_eq!(c.events, 1);
//! assert_eq!(c.states, 1);
//! ```

/// Parsed Paraver `.prv` summary.
#[derive(Debug, Clone)]
pub struct Paraver {
    /// Trace duration reported by the header (integer timespan).
    pub duration: u64,
    /// Number of applications declared in the header.
    pub applications: usize,
    /// Total typed body records (`T:…`).
    pub records: usize,
    /// `1:` state records.
    pub states: usize,
    /// `2:` event records.
    pub events: usize,
    /// `3:` communication records.
    pub comms: usize,
    /// `#Paraver CFG` configuration lines.
    pub configs: usize,
}

fn header_duration(t: &str) -> Option<u64> {
    let l = t
        .lines()
        .find(|l| l.trim_start().starts_with("#Paraver ("))?;
    let close = l.find("):")?;
    l[close + 2..].split(':').next()?.parse().ok()
}

fn header_apps(t: &str) -> usize {
    let Some(l) = t.lines().find(|l| l.trim_start().starts_with("#Paraver (")) else {
        return 0;
    };
    let Some(close) = l.find("):") else { return 0 };
    // fields: duration : nNodes(...) : nApps : appList
    let mut it = l[close + 2..].split(':');
    let _ = it.next();
    let _ = it.next();
    it.next().and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn rec_type(tr: &str) -> Option<u8> {
    let colon = tr.find(':')?;
    let t: u8 = tr[..colon].parse().ok()?;
    ((1..=4).contains(&t)).then_some(t)
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Whether the buffer starts with a Paraver trace header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .find(|l| !l.trim().is_empty())
        .is_some_and(|l| l.trim_start().starts_with("#Paraver ("))
}

impl Paraver {
    /// Parses a Paraver trace summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            duration: header_duration(t).unwrap_or(0),
            applications: header_apps(t),
            records: 0,
            states: 0,
            events: 0,
            comms: 0,
            configs: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                if tr.starts_with("#Paraver") && !tr.starts_with("#Paraver (") {
                    c.configs += 1;
                }
                continue;
            }
            let Some(ty) = rec_type(tr) else { continue };
            c.records += 1;
            match ty {
                1 => c.states += 1,
                2 => c.events += 1,
                3 => c.comms += 1,
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"#Paraver (19/02/2010 at 16:22):5000000010:2(1,2):1:1(1:1)
#Paraver CFG
# comment line
2:0:1:1:1:0:1000000:9
2:0:1:1:1:1:1100000:7
1:0:1:1:1:0:999999:1
3:0:1:1:1:1:2:1:1:1:0:100:200:500
4:0:1:1:1:1:2:1
"#;
        assert!(detect(b));
        let c = Paraver::parse(b).unwrap();
        assert_eq!(c.duration, 5_000_000_010);
        assert_eq!(c.applications, 1);
        assert_eq!(c.records, 5);
        assert_eq!(c.events, 2);
        assert_eq!(c.states, 1);
        assert_eq!(c.comms, 1);
        assert_eq!(c.configs, 1);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"2:0:1:1:1:0:1:9\n"));
        assert!(!detect(b"#Paraver"));
        assert!(Paraver::parse(b"hello").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}

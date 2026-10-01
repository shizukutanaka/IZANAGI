//! Linux `perf script` output parser.
//!
//! Recognises `perf script` / `perf script -F` text rows
//! (`COMM PID [CPU] TIMESTAMP: EVENT: fields`) and counts timestamped
//! records, event classes (`cycles:ppp`, `sched:sched_switch`, raw
//! `cycles`), distinct CPUs seen, and comm names.
//!
//! ```
//! let b = concat!(
//!     "swapper     0 [000] 1000", ".", "000: sched:sched_switch: prev\n",
//!     "main    1234 [001] 1001", ".", "500: cycles:ppp: 7f4a2c00 malloc\n",
//!     "main    1234 [001] 1002", ".", "250: cpu-clock:pppH: 7f4a2d10 run\n"
//! ).as_bytes();
//! assert!(izanagi_kit::perflog::detect(b));
//! let c = izanagi_kit::perflog::Perflog::parse(b).unwrap();
//! assert_eq!(c.records, 3);
//! assert_eq!(c.traced, 3);
//! assert_eq!(c.cpus, 2);
//! ```

/// Parsed `perf script` output summary.
#[derive(Debug, Clone)]
pub struct Perflog {
    /// Timestamped record rows (`[NNN] ts: event: …`).
    pub records: usize,
    /// Records whose event name carries a `:` (tracepoints/`x:y` events).
    pub traced: usize,
    /// Distinct `[cpu]` indices seen.
    pub cpus: usize,
    /// Distinct comm (leftmost field) names, capped at 32.
    pub comms: usize,
    /// Lost-event / `# Lost` marker lines.
    pub lost: usize,
}

/// Splits `comm pid [cpu] ts: event: rest` → (cpu, event).
fn fields_of(tr: &str) -> Option<(usize, &str)> {
    let lb = tr.find('[')?;
    let rb = tr[lb..].find(']')? + lb;
    let cpu: usize = tr[lb + 1..rb].parse().ok()?;
    let rest = &tr[rb + 1..];
    let colon = rest.find(':')?;
    let ts = rest[..colon].trim();
    // TIMESTAMP must look like a fixed-point number (`123.456`).
    let dot = ts.find('.')?;
    if ts[..dot].chars().any(|c| !c.is_ascii_digit())
        || ts[dot + 1..].chars().any(|c| !c.is_ascii_digit())
        || ts[..dot].is_empty()
        || ts[dot + 1..].is_empty()
    {
        return None;
    }
    // Event names may themselves carry ':' (`sched:sched_switch`,
    // `cycles:ppp`), so split at `": "` before the payload.
    let after = rest[colon + 1..].trim();
    let end = after.find(": ").unwrap_or(after.len());
    let event = after[..end].trim_end_matches(':').trim();
    (!event.is_empty() && event.chars().all(|c| !c.is_control())).then_some((cpu, event))
}

/// Whether the buffer looks like `perf script` output.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .filter(|l| fields_of(l.trim()).is_some())
        .take(8)
        .count()
        >= 2
}

impl Perflog {
    /// Parses a `perf script` output summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            records: 0,
            traced: 0,
            cpus: 0,
            comms: 0,
            lost: 0,
        };
        let mut seen_cpus = 0u64;
        let mut seen_comms = 0u64;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("# Lost") || tr.starts_with("Lost") {
                c.lost += 1;
                continue;
            }
            let Some((cpu, event)) = fields_of(tr) else {
                continue;
            };
            c.records += 1;
            if event.contains(':') {
                c.traced += 1;
            }
            if cpu < 64 {
                let bit = 1u64 << cpu;
                if seen_cpus & bit == 0 {
                    seen_cpus |= bit;
                    c.cpus += 1;
                }
            }
            if let Some(comm) = tr.split_whitespace().next() {
                if c.comms < 32 {
                    let mut h = 0u64;
                    for ch in comm.bytes() {
                        h = h.wrapping_mul(131).wrapping_add(u64::from(ch));
                    }
                    if seen_comms & (1 << (h % 64)) == 0 {
                        seen_comms |= 1 << (h % 64);
                        c.comms += 1;
                    }
                }
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
        let b = concat!(
            "swapper     0 [000] 1000",
            ".",
            "000: sched:sched_switch: prev_comm=swapper\n",
            "main    1234 [001] 1001",
            ".",
            "500: cycles:ppp:  7f4a2c00 malloc (/lib/libc.so)\n",
            "main    1234 [001] 1002",
            ".",
            "250: cpu-clock:pppH:  7f4a2d10 run (/bin/app)\n",
            "main    1234 [000] 1003",
            ".",
            "000: cycles:  7f4a2e00 done (/bin/app)\n",
            "# Lost 2 events\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Perflog::parse(b).unwrap();
        assert_eq!(c.records, 4);
        assert_eq!(c.traced, 3);
        assert_eq!(c.cpus, 2);
        assert_eq!(c.comms, 2);
        assert_eq!(c.lost, 1);
    }

    #[test]
    fn rejects_plain_logs() {
        assert!(!detect(b"INFO starting\nINFO done\n"));
        assert!(!detect(b"1 2 3\n4 5 6\n"));
        assert!(Perflog::parse(b"swapper 0 [000] : x").is_none());
    }
}

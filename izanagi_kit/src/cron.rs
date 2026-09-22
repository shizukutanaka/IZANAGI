//! Cron schedule → next fire times, evaluated against a UTC epoch
//! instant in integer milliseconds.
//!
//! Scheduled real-world events inside the sim — daily quest resets,
//! weekly tournaments, "open the shop every Friday" — are usually wired
//! as ad-hoc `tick % N` checks that no config file can express. A
//! standard 5-field cron expression (`min hour dom mon dow`, with `*`,
//! `*/n`, `a`, `a-b`, `a-b/n`, comma lists, and `jan`–`dec`/`sun`–`sat`
//! names) parsed once and queried as a pure function of the instant:
//! same input, same answer, on every platform.
//!
//! Day-of-month vs day-of-week follows the POSIX/Vixie rule: when
//! **both** are restricted the match is an OR, otherwise an AND — a
//! `0 9 1 * fri` means "9:00 on the 1st of every month *and* every
//! Friday".
//!
//! ```
//! use izanagi_kit::cron::Cron;
//! let daily_9am = Cron::parse("0 9 * * *").unwrap();
//! // 2026-09-22 00:00:00Z = 1790035200000 ms → next is 09:00 same day.
//! let next = daily_9am.next_after(1790035200000).unwrap();
//! assert_eq!(next, 1790035200000 + 9 * 3600_000);
//! ```

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;

/// A parsed cron expression — five field bitsets plus the two `*`
/// flags the POSIX OR-rule needs.
#[derive(Clone)]
pub struct Cron {
    mins: u64,   // bits 0..60
    hours: u32,  // bits 0..24
    doms: u32,   // bits 1..32
    months: u16, // bits 1..13
    dows: u8,    // bits 0..7
    dom_star: bool,
    dow_star: bool,
}

/// `(year, month, day)` — civil date, Gregorian proleptic.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    // Howard Hinnant's algorithm — all integer ops.
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// Days in `y`-`m`.
fn days_in_month(y: i64, m: u32) -> u32 {
    const DIM: [u32; 13] = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if m == 2 && (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)) {
        29
    } else {
        DIM[m as usize]
    }
}

/// 1970-01-01 was a Thursday (dow 4).
fn dow_of_day(days: i64) -> u32 {
    ((days % 7 + 7 + 4) % 7) as u32
}

/// Parse one field into a bitset. `lo..=hi` is the legal range;
/// `names` maps lowercase abbreviations to values (e.g. `jan` → 1).
fn parse_field(text: &str, lo: u32, hi: u32, names: &[(&str, u32)]) -> Option<u64> {
    let mut bits = 0u64;
    for part in text.split(',') {
        if part.is_empty() {
            return None;
        }
        let (range_text, step) = match part.split_once('/') {
            Some((r, s)) => (r, s.parse::<u32>().ok().filter(|&n| n > 0)?),
            None => (part, 1),
        };
        let (a, b) = match range_text.split_once('-') {
            Some((a, b)) => (parse_value(a, names)?, parse_value(b, names)?),
            None if range_text == "*" => (lo, hi),
            None => {
                let v = parse_value(range_text, names)?;
                if v < lo || v > hi {
                    return None;
                }
                if step == 1 {
                    bits |= 1u64 << v;
                    continue;
                }
                (v, hi) // `a/n` → a..hi step n (Quartz-compatible)
            }
        };
        if a > b || a < lo || b > hi {
            return None;
        }
        let mut v = a;
        while v <= b {
            bits |= 1u64 << v;
            v += step;
        }
    }
    (bits != 0).then_some(bits)
}

fn parse_value(t: &str, names: &[(&str, u32)]) -> Option<u32> {
    let lower = t.to_ascii_lowercase();
    for &(name, v) in names {
        if lower == name {
            return Some(v);
        }
    }
    t.parse::<u32>().ok()
}

const MONTH_NAMES: [(&str, u32); 12] = [
    ("jan", 1),
    ("feb", 2),
    ("mar", 3),
    ("apr", 4),
    ("may", 5),
    ("jun", 6),
    ("jul", 7),
    ("aug", 8),
    ("sep", 9),
    ("oct", 10),
    ("nov", 11),
    ("dec", 12),
];
const DOW_NAMES: [(&str, u32); 7] = [
    ("sun", 0),
    ("mon", 1),
    ("tue", 2),
    ("wed", 3),
    ("thu", 4),
    ("fri", 5),
    ("sat", 6),
];

impl Cron {
    /// Parse a 5-field cron expression (`min hour dom mon dow`).
    /// Returns `None` on malformed or out-of-range fields.
    pub fn parse(text: &str) -> Option<Self> {
        let fields: Vec<&str> = text.split_whitespace().collect();
        if fields.len() != 5 {
            return None;
        }
        let mins = parse_field(fields[0], 0, 59, &[])?;
        let hours = parse_field(fields[1], 0, 23, &[])?;
        let dom_star = fields[2] == "*";
        let dow_star = fields[4] == "*";
        let doms = parse_field(fields[2], 1, 31, &[])?;
        let months = parse_field(fields[3], 1, 12, &MONTH_NAMES)?;
        // dow accepts 0-6 and 7 ≡ Sunday.
        let mut dows = parse_field(fields[4], 0, 7, &DOW_NAMES)?;
        if dows & (1 << 7) != 0 {
            dows |= 1; // Sunday bit
        }
        Some(Self {
            mins,
            hours: hours as u32,
            doms: doms as u32,
            months: months as u16,
            dows: (dows & 0x7f) as u8,
            dom_star,
            dow_star,
        })
    }

    /// Does the civil date/time `(y, m, d, dow, hour, min)` fire?
    /// `dom_star`/`dow_star` drive the POSIX OR-rule.
    fn fires(&self, dow: u32, dom: u32, month: u32, hour: u32, min: u32, dim: u32) -> bool {
        if self.mins & (1 << min) == 0 || self.hours & (1 << hour) == 0 {
            return false;
        }
        if self.months & (1 << month) == 0 {
            return false;
        }
        self.day_matches(dom, dow, dim)
    }

    /// The first fire time strictly after `after` (epoch ms), aligned to
    /// the minute. `None` only when no fire exists within a 5-year
    /// horizon — which means the schedule is impossible (e.g. `30 2 *`)
    /// or so rare it is below the search bound.
    pub fn next_after(&self, after: i64) -> Option<i64> {
        // First whole-minute boundary strictly after `after`.
        let mut day = after.div_euclid(DAY_MS);
        let mut minute_of_day = (after.rem_euclid(DAY_MS)) / MINUTE_MS + 1;
        if minute_of_day >= 24 * 60 {
            day += 1;
            minute_of_day = 0;
        }
        // At most ~5 years of days — every valid schedule fires inside
        // it (a leap Feb-29 schedule is the sparsest possible).
        for _ in 0..(366 * 5 + 2) {
            let (y, m, d) = civil_from_days(day);
            let dim = days_in_month(y, m);
            let dow = dow_of_day(day);
            if self.months & (1 << m) != 0 && self.day_matches(d, dow, dim) {
                // Walk set (hour, minute) pairs ascending from
                // `minute_of_day`.
                let start_hour = (minute_of_day / 60) as u32;
                for hour in start_hour..24 {
                    if self.hours & (1 << hour) == 0 {
                        continue;
                    }
                    let mstart = if hour == start_hour {
                        (minute_of_day % 60) as u32
                    } else {
                        0
                    };
                    let mut min = mstart;
                    while min < 60 {
                        if self.mins & (1 << min) != 0 && self.fires(dow, d, m, hour, min, dim) {
                            return Some(
                                day * DAY_MS + hour as i64 * HOUR_MS + min as i64 * MINUTE_MS,
                            );
                        }
                        min += 1;
                    }
                }
            }
            day += 1;
            minute_of_day = 0;
        }
        None
    }

    /// Day-level predicate with the POSIX OR-rule.
    fn day_matches(&self, dom: u32, dow: u32, dim: u32) -> bool {
        let dom_hit = dom <= dim && (self.doms & (1 << dom)) != 0;
        let dow_hit = (self.dows & (1 << dow)) != 0;
        match (self.dom_star, self.dow_star) {
            (true, true) => true,
            (true, false) => dow_hit,
            (false, true) => dom_hit,
            (false, false) => dom_hit || dow_hit,
        }
    }

    /// The next `n` fire times after `after`, ascending.
    pub fn next_n_after(&self, after: i64, n: usize) -> Vec<i64> {
        let mut out = Vec::with_capacity(n);
        let mut cursor = after;
        for _ in 0..n {
            match self.next_after(cursor) {
                Some(t) => {
                    out.push(t);
                    cursor = t;
                }
                None => break,
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    /// Reference: scan minute-by-minute. Naive but obviously correct —
    /// the oracle the fast path is measured against.
    fn brute_next(c: &Cron, after: i64, horizon_days: i64) -> Option<i64> {
        let mut t = after / MIN * MIN + MIN; // first minute boundary > after
        let end = after + horizon_days * DAY_MS;
        while t < end {
            let day = t.div_euclid(DAY_MS);
            let (y, m, d) = civil_from_days(day);
            let rem = t.rem_euclid(DAY_MS);
            let hour = (rem / HOUR_MS) as u32;
            let min = ((rem % HOUR_MS) / MIN) as u32;
            if c.fires(dow_of_day(day), d, m, hour, min, days_in_month(y, m)) {
                return Some(t);
            }
            t += MIN;
        }
        None
    }

    /// Days since 1970-01-01 for `y-m-d` (oracle inverse of
    /// `civil_from_days` — only the round-trip test uses it).
    fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
        let doy = (153 * mp + 2) / 5 + d as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    #[test]
    fn civil_conversions_round_trip() {
        // Epoch boundary conditions.
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(dow_of_day(0), 4); // Thursday
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(
            days_from_civil(2000, 2, 29),
            days_from_civil(2000, 3, 1) - 1
        );
        for d in [-30_000i64, -1, 0, 1, 14_000, 20_000] {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
    }

    #[test]
    fn parse_rejects_and_accepts() {
        assert!(Cron::parse("* * * *").is_none());
        assert!(Cron::parse("* * * * * *").is_none());
        assert!(Cron::parse("60 * * * *").is_none());
        assert!(Cron::parse("* 24 * * *").is_none());
        assert!(Cron::parse("* * 0 * *").is_none());
        assert!(Cron::parse("* * * 13 *").is_none());
        assert!(Cron::parse("*/0 * * * *").is_none());
        assert!(Cron::parse("a * * * *").is_none());
        assert!(Cron::parse("0 9 * jan mon").is_some());
        assert!(Cron::parse("0 9 * jan,mar mon,wed").is_some());
        assert!(Cron::parse("*/15 9-17/2 * * mon-fri").is_some());
        // Sunday aliases: 0 and 7 are the same day.
        let sun0 = Cron::parse("0 0 * * 0").unwrap();
        let sun7 = Cron::parse("0 0 * * 7").unwrap();
        let sun = Cron::parse("0 0 * * sun").unwrap();
        // 2026-09-20 is a Sunday — all three fire at the same instant.
        let sat_night = 1_789_855_200_000i64; // 2026-09-19 22:00Z
        assert_eq!(sun0.next_after(sat_night), sun7.next_after(sat_night));
        assert_eq!(sun0.next_after(sat_night), sun.next_after(sat_night));
    }

    #[test]
    fn known_schedules() {
        // Every minute.
        let c = Cron::parse("* * * * *").unwrap();
        assert_eq!(c.next_after(1_790_035_200_000), Some(1_790_035_260_000));
        // 9:00 daily — 2026-09-22 00:00Z → same day 09:00Z.
        let c = Cron::parse("0 9 * * *").unwrap();
        assert_eq!(
            c.next_after(1_790_035_200_000),
            Some(1_790_035_200_000 + 9 * HOUR_MS)
        );
        // Same schedule, asked mid-day 10:00 → tomorrow 9:00.
        assert_eq!(
            c.next_after(1_790_035_200_000 + 10 * HOUR_MS),
            Some(1_790_035_200_000 + 33 * HOUR_MS)
        );
        // Weekday mornings only.
        let wd = Cron::parse("30 8 * * mon-fri").unwrap();
        // 2026-09-19 (Sat) 09:00Z → next is Mon 2026-09-21 08:30Z.
        assert_eq!(wd.next_after(1_789_808_400_000), Some(1_789_979_400_000));
        // Feb 29: fires only on leap years.
        let leap = Cron::parse("0 0 29 2 *").unwrap();
        let t = leap.next_after(1_700_000_000_000).unwrap(); // ~2023-11
        let day = t.div_euclid(DAY_MS);
        assert_eq!(civil_from_days(day), (2024, 2, 29));
        // Impossible schedule: Feb 31 never exists → `None`, not a hang.
        let never = Cron::parse("0 0 31 2 *").unwrap();
        assert_eq!(never.next_after(1_790_035_200_000), None);
    }

    #[test]
    fn matches_brute_force_oracle() {
        // Dense + sparse schedules, checked against minute-by-minute
        // scan over a 60-day window.
        let schedules = [
            "* * * * *",
            "*/15 * * * *",
            "7 3 * * *",
            "0 9 1 * *",
            "0 9 * * fri",
            "0 9 1 * fri", // POSIX OR case
            "30 8 * * mon-fri",
            "0 0 29 2 *", // leap day — sparse, needs the day loop
            "45 23 * * sun",
            "*/7 */3 * * *",
        ];
        let starts = [
            1_790_035_200_000i64, // 2026-09-22
            0,                    // epoch
            1_700_000_000_000,
        ];
        for sched in schedules {
            let c = Cron::parse(sched).unwrap();
            for &s in &starts {
                // Compare the first 3 firings within a 60-day window.
                let mut cur_fast = s;
                let mut cur_brute = s;
                let horizon = if sched == "0 0 29 2 *" { 2_000 } else { 60 };
                for _ in 0..3 {
                    let f = c.next_after(cur_fast);
                    let b = brute_next(&c, cur_brute, horizon);
                    assert_eq!(f, b, "{sched} after {cur_fast}");
                    match f {
                        Some(t) => {
                            cur_fast = t;
                            cur_brute = t;
                        }
                        None => break,
                    }
                }
            }
        }
    }

    #[test]
    fn next_n_is_chain_of_next_after() {
        let c = Cron::parse("15,45 * * * *").unwrap();
        let v = c.next_n_after(1_790_035_200_000, 4);
        assert_eq!(v.len(), 4);
        for w in v.windows(2) {
            assert!(w[1] > w[0]);
        }
        assert_eq!(v[0], 1_790_035_200_000 + 15 * MIN);
        // Empty for impossible schedules.
        let impossible = Cron::parse("0 0 31 2 *").unwrap(); // Feb 31
        assert_eq!(impossible.next_after(0), None);
        assert_eq!(impossible.next_n_after(0, 3), Vec::<i64>::new());
    }
}

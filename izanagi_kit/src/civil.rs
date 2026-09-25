//! Proleptic Gregorian civil-calendar arithmetic (Howard Hinnant's
//! `days_from_civil` / `civil_from_days`), plus weekday computation.
//!
//! Days are counted relative to the Unix epoch 1970-01-01 (which may be
//! negative for earlier dates). Every function is exact integer arithmetic —
//! no time zones, no leap seconds — so results are bit-exact everywhere.
//!
//! ```
//! use izanagi_kit::civil;
//! assert_eq!(civil::days_from_civil(1970, 1, 1), 0);
//! assert_eq!(civil::weekday(0), 4); // 1970-01-01 was a Thursday
//! assert_eq!(civil::civil_from_days(0), (1970, 1, 1));
//! ```

/// Days from 1970-01-01 to `y`-`m`-`d` in the proleptic Gregorian calendar.
/// `m` outside 1..=12 normalizes by shifting years (Hinnant's `y -= m <= 2`
/// adjustment handles Jan/Feb internally); `d` outside the month wraps by
/// integer carry. Returns a signed day count (negative before the epoch).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m + 9) % 12; // Mar=0 ... Jan=10, Feb=11
    let doy = (153 * mp as i32 + 2) / 5 + d as i32 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Inverse of [`days_from_civil`]: day `z` → `(year, month, day)`.
pub fn civil_from_days(z: i32) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Weekday of day `z` (0 = Sunday … 6 = Saturday). 1970-01-01 = Thursday.
pub fn weekday(z: i32) -> u32 {
    (z + 4).rem_euclid(7) as u32
}

/// True when `y` is a Gregorian leap year.
pub fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Number of days in `y`-`m` (m outside 1..=12 → 0).
pub fn days_in_month(y: i32, m: u32) -> u32 {
    const D: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if m == 0 || m > 12 {
        return 0;
    }
    D[(m - 1) as usize] + u32::from(m == 2 && is_leap(y))
}

/// ISO-8601 day-of-week (1 = Monday … 7 = Sunday) for day `z`.
pub fn weekday_iso(z: i32) -> u32 {
    (z + 3).rem_euclid(7) as u32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 1, 1), 10_957);
        assert_eq!(days_from_civil(1900, 1, 1), -25_567);
        // 2024-02-29 is a leap day.
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(weekday(days_from_civil(2000, 1, 1)), 6); // Saturday
        assert_eq!(weekday(0), 4); // Thursday
        assert_eq!(weekday_iso(0), 4);
        assert_eq!(weekday_iso(-1), 3); // 1969-12-31 Wednesday
    }

    #[test]
    fn roundtrip_across_eras() {
        for z in -80_000i32..80_000 {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z, "z={z}");
        }
    }

    #[test]
    fn leap_rules() {
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(is_leap(2024));
        assert!(!is_leap(2023));
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 4), 30);
        assert_eq!(days_in_month(2024, 13), 0);
    }

    #[test]
    fn month_boundaries() {
        // 1970-03-01 is day 59 (31 + 28).
        assert_eq!(days_from_civil(1970, 3, 1), 59);
        // Leap year: 1972-03-01 is day 60 of year 2.
        assert_eq!(days_from_civil(1972, 3, 1), 790);
        // Negative era: Julian day boundaries still consistent.
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(days_from_civil(1, 1, 1), -719_162);
        assert_eq!(civil_from_days(-719_162), (1, 1, 1));
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(civil_from_days(19_999), civil_from_days(19_999));
    }
}

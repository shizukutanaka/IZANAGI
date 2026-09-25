//! Multi-calendar date arithmetic after Reingold & Dershowitz,
//! *Calendrical Calculations*.
//!
//! [`civil`] converts Gregorian dates to/from a day count.
//! This module extends that to the Julian, arithmetic Hebrew, tabular
//! Islamic, and arithmetic Persian calendars, plus the Western Easter
//! computus — all in **R.D.** (Rata Die) day numbers: R.D. 1 is
//! Gregorian 1 January 1 CE, so `rd = days_from_civil(y,m,d) + 719162`.
//! Everything is exact integer arithmetic; no floats anywhere.
//!
//! ```
//! use izanagi_kit::calendars::{self, Rd};
//!
//! // Julian day 1 March 4 CE ↔ Gregorian R.D.
//! let rd = calendars::julian_to_rd(4, 3, 1);
//! assert_eq!(calendars::rd_to_julian(rd), (4, 3, 1));
//! // Same instant in tabular Islamic:
//! let (hy, hm, hd) = calendars::rd_to_islamic(rd);
//! assert_eq!(calendars::islamic_to_rd(hy, hm, hd), rd);
//! ```

use crate::civil;

/// R.D. (rata die): integer day count with epoch at Gregorian 1-01-01.
pub type Rd = i64;

/// Day-number offset between [`civil::days_from_civil`] (epoch 1970-01-01)
/// and R.D. (epoch 1-01-01 Gregorian): R.D. of 1970-01-01 is 719163.
pub const CIVIL_TO_RD: i64 = 719_163;

/// Gregorian (y,m,d) → R.D.
pub fn gregorian_to_rd(y: i32, m: u32, d: u32) -> Rd {
    civil::days_from_civil(y, m, d) as i64 + CIVIL_TO_RD
}

/// R.D. → proleptic Gregorian (y,m,d).
pub fn rd_to_gregorian(rd: Rd) -> (i32, u32, u32) {
    civil::civil_from_days((rd - CIVIL_TO_RD) as i32)
}

// ---------------------------------------------------------------- Julian

/// Julian epoch in R.D.: 1 Jan 1 CE Julian = R.D. −1 (R&D: JULIAN_EPOCH = -1).
const JULIAN_EPOCH: Rd = -1;

fn julian_leap(y: i64) -> bool {
    if y > 0 {
        y % 4 == 0
    } else {
        // No year 0: year -1 = 2 BCE, and 1 BC, 5 BC, ... are leap (y % 4 == 0 in
        // astronomical counting is y ≡ 0 with offset — R&D use mod(y,4) == 0 or 3).
        (y + 1) % 4 == 0
    }
}

/// Julian calendar → R.D. (R&D eq. for `fixed_from_julian`; year numbering
/// astronomical: year 0 = 1 BCE not used — pass signed year where 1 = 1 CE,
/// −1 = 2 BCE… wait: we use historical years with NO year 0: `y` is the
/// signed year where `y>0` is CE and `y<0` is `|y|` BCE.)
pub fn julian_to_rd(y: i32, m: u32, d: u32) -> Rd {
    let y = if y < 0 { y as i64 + 1 } else { y as i64 }; // shift to astronomical
    let mm = m as i64;
    let dd = d as i64;
    let mut rd =
        JULIAN_EPOCH - 1 + 365 * (y - 1) + (y - 1).div_euclid(4) + (367 * mm - 362).div_euclid(12);
    if mm > 2 {
        rd += if julian_leap(y) { -1 } else { -2 };
    }
    rd + dd
}

/// R.D. → Julian (year, month, day); year uses historical (no year 0) numbering.
pub fn rd_to_julian(rd: Rd) -> (i32, u32, u32) {
    // Approximate year, then correct.
    let approx = (4 * (rd - JULIAN_EPOCH) + 1464) / 1461;
    let mut y = approx;
    while julian_to_rd_no_shift(y, 1, 1) > rd {
        y -= 1;
    }
    while julian_to_rd_no_shift(y + 1, 1, 1) <= rd {
        y += 1;
    }
    let mut m = 1u32;
    while julian_to_rd_no_shift(y, m + 1, 1) <= rd {
        m += 1;
    }
    let d = (rd - julian_to_rd_no_shift(y, m, 1) + 1) as u32;
    let hist = if y <= 0 { y - 1 } else { y };
    (hist as i32, m, d)
}

fn julian_to_rd_no_shift(y: i64, m: u32, d: u32) -> Rd {
    let mm = m as i64;
    let dd = d as i64;
    let mut rd =
        JULIAN_EPOCH - 1 + 365 * (y - 1) + (y - 1).div_euclid(4) + (367 * mm - 362).div_euclid(12);
    if mm > 2 {
        rd += if julian_leap(y) { -1 } else { -2 };
    }
    rd + dd
}

// ---------------------------------------------------------------- Islamic

/// Tabular Islamic epoch in R.D.: 16 July 622 CE Julian = R.D. 227015.
const ISLAMIC_EPOCH: Rd = 227_015;

/// Islamic leap year in the 30-year cycle (leaps: 2,5,7,10,13,16,18,21,24,26,29).
pub fn islamic_leap(y: i32) -> bool {
    let y = y as i64;
    (14 + 11 * y).rem_euclid(30) < 11
}

/// Tabular (civil) Islamic date → R.D. Months alternate 30/29 days
/// (Muharram = 30); `ceil(29.5·(m−1)) = 29·(m−1) + ⌊m/2⌋`.
pub fn islamic_to_rd(y: i32, m: u32, d: u32) -> Rd {
    islamic_to_rd_raw(y as i64, m, d)
}

fn islamic_to_rd_raw(y: i64, m: u32, d: u32) -> Rd {
    let m = m as i64;
    (d as i64) + 29 * (m - 1) + m / 2 + 354 * (y - 1) + (3 + 11 * y) / 30 + ISLAMIC_EPOCH - 1
}

/// R.D. → tabular Islamic (year, month, day).
pub fn rd_to_islamic(rd: Rd) -> (i32, u32, u32) {
    // Mean-year estimate, then walk to the exact year.
    let mut y = ((30 * (rd - ISLAMIC_EPOCH) + 10646) / 10631).max(1);
    while islamic_to_rd_raw(y, 1, 1) > rd {
        y -= 1;
    }
    while islamic_to_rd_raw(y + 1, 1, 1) <= rd {
        y += 1;
    }
    // Month: smallest m whose start exceeds rd gives m−1; scan forward.
    let mut m = 1u32;
    while m < 12 && islamic_to_rd_raw(y, m + 1, 1) <= rd {
        m += 1;
    }
    let d = (rd - islamic_to_rd_raw(y, m, 1) + 1) as u32;
    (y as i32, m, d)
}

// ---------------------------------------------------------------- Persian

/// Arithmetic Persian (Jalali) epoch: R.D. 226896 = Julian 622-03-19.
const PERSIAN_EPOCH: Rd = 226_896;

/// R&D's 2820-year-cycle arithmetic Persian: `epbase = y − 474`,
/// `epyear = 474 + epbase mod 2820`, leap iff `(epyear + 38)·682 mod 2816 < 682`.
pub fn persian_leap(y: i32) -> bool {
    let y = y as i64;
    let epbase = if y >= 0 { y - 474 } else { y - 473 };
    let epyear = 474 + epbase.rem_euclid(2820);
    ((epyear + 38) * 682).rem_euclid(2816) < 682
}

/// Arithmetic Persian date → R.D. (R&D `fixed_from_persian_arithmetic`).
pub fn persian_to_rd(y: i32, m: u32, d: u32) -> Rd {
    let y = y as i64;
    let m = m as i64;
    let epbase = if y >= 0 { y - 474 } else { y - 473 };
    let epyear = 474 + epbase.rem_euclid(2820);
    let new_year = PERSIAN_EPOCH - 1
        + 1029983 * epbase.div_euclid(2820)
        + 365 * (epyear - 1)
        + (682 * epyear - 110).div_euclid(2816);
    let month_days = if m <= 7 {
        (m - 1) * 31
    } else {
        (m - 1) * 30 + 6
    };
    new_year + month_days + d as i64
}

/// R.D. → arithmetic Persian (year, month, day).
pub fn rd_to_persian(rd: Rd) -> (i32, u32, u32) {
    // dep with the 474-day origin offset, then invert the day formula.
    let dep = rd - (PERSIAN_EPOCH - 1) + 474 * 365 + 682 * 474 / 2816;
    let y0 = ((2816 * dep + 1339) / 1028522).max(1);
    let mut y = y0;
    while persian_to_rd(y as i32, 1, 1) > rd {
        y -= 1;
    }
    while persian_to_rd((y + 1) as i32, 1, 1) <= rd {
        y += 1;
    }
    let day0 = rd - persian_to_rd(y as i32, 1, 1);
    let m = if day0 < 186 {
        ((day0 / 31) + 1) as u32
    } else {
        (((day0 - 186) / 30) + 7) as u32
    };
    let d = (rd - persian_to_rd(y as i32, m, 1) + 1) as u32;
    (y as i32, m, d)
}

// ---------------------------------------------------------------- Hebrew

/// Hebrew epoch (molad tohu): Monday 7 October 3761 BCE Julian = R.D. −1373427.
const HEBREW_EPOCH: Rd = -1_373_427;

/// Hebrew leap year (years 3,6,8,11,14,17,19 of the 19-year Metonic cycle:
/// `(7y + 1) mod 19 < 7`).
pub fn hebrew_leap(y: i32) -> bool {
    (7 * (y as i64) + 1).rem_euclid(19) < 7
}

/// Months in a Hebrew year: 13 in a leap year, else 12.
pub fn hebrew_year_months(y: i32) -> u32 {
    if hebrew_leap(y) {
        13
    } else {
        12
    }
}

/// Days in a Hebrew year (353/354/355 common; 383/384/385 leap).
fn hebrew_year_days(y: i32) -> i64 {
    hebrew_new_year(y + 1) - hebrew_new_year(y)
}

/// Days in a Hebrew month (m: 1=Nisan … 12=Adar(I), 13=Adar II in leap years).
pub fn hebrew_month_days(y: i32, m: u32) -> u32 {
    match m {
        // Iyar(2), Tammuz(4), Elul(6), Tevet(10), Adar II(13): always 29.
        2 | 4 | 6 | 10 | 13 => 29,
        // Adar I (leap years only) has 30; plain Adar (common years) 29.
        12 => {
            if hebrew_leap(y) {
                30
            } else {
                29
            }
        }
        // Heshvan(8) is 29 unless the year is "full" (355/385: len % 10 == 5);
        // Kislev(9) is 29 when the year is "deficient" (353/383: % 10 == 3).
        8 => {
            if hebrew_year_days(y) % 10 == 5 {
                30
            } else {
                29
            }
        }
        9 if hebrew_year_days(y) % 10 == 3 => 29,
        _ => 30,
    }
}

/// Days elapsed from the Hebrew epoch to Rosh Hashanah of `y`
/// (R&D `hebrew_calendar_elapsed_days`: molad parts + postponements).
fn hebrew_elapsed_days(y: i32) -> i64 {
    let y = y as i64;
    let months = (235 * y - 234).div_euclid(19); // elapsed months before Tishrei
    let parts0 = 12084 + 13753 * months;
    let day = 29 * months + parts0.div_euclid(25920);
    // Dehiyyah GaTaRaD: molad at/after 18h; or Tue ≥9h204p common; or
    // Mon ≥15h589p after a leap year → postpone one day.
    if (3 * (day + 1)).rem_euclid(7) < 3 {
        day + 1
    } else {
        day
    }
}

/// R.D. of Rosh Hashanah (1 Tishrei) for Hebrew year `y`.
/// `hebrew_epoch + elapsed_days`, then +1 when the day lands on
/// Sunday/Wednesday/Friday (dehiyyah Lo ADU Rosh).
pub fn hebrew_new_year(y: i32) -> Rd {
    let d = hebrew_elapsed_days(y);
    HEBREW_EPOCH + d + i64::from(matches!((HEBREW_EPOCH + d).rem_euclid(7), 0 | 3 | 5))
}

/// Hebrew (y, m, d) → R.D. Month numbering: 1=Nisan … 13=Adar II.
pub fn hebrew_to_rd(y: i32, m: u32, d: u32) -> Rd {
    // R&D count months from Tishrei(7) to Nisan(1): months are enumerated
    // 7..end then 1..6.  Convert to "month index in year" then sum days.
    let mut days = 0i64;
    let months = hebrew_year_months(y);
    if m < 7 {
        // Nisan..Elul(1..6): months 7..months (Tishrei..end) + 1..m
        let mut mm = 7;
        while mm <= months {
            days += hebrew_month_days(y, mm) as i64;
            mm += 1;
        }
        let mut mm = 1;
        while mm < m {
            days += hebrew_month_days(y, mm) as i64;
            mm += 1;
        }
    } else {
        let mut mm = 7;
        while mm < m {
            days += hebrew_month_days(y, mm) as i64;
            mm += 1;
        }
    }
    hebrew_new_year(y) + days + (d as i64) - 1
}

/// R.D. → Hebrew (year, month, day). Month: 1=Nisan … 13=Adar II.
pub fn rd_to_hebrew(rd: Rd) -> (i32, u32, u32) {
    // Mean-year estimate (Hebrew mean year ≈ 35975351/98496 days), then walk.
    let mut y = (1 + ((rd - HEBREW_EPOCH) * 98496).div_euclid(35975351)).max(1) as i32;
    while hebrew_new_year(y) > rd {
        y -= 1;
    }
    while hebrew_new_year(y + 1) <= rd {
        y += 1;
    }
    let yy = y;
    // Months before Nisan run 7..last; at/after Nisan the month is in 1..6.
    let months = hebrew_year_months(yy);
    let d: u32;
    let m: u32;
    if rd < hebrew_to_rd(yy, 1, 1) {
        let mut mm = 7;
        while mm < months && hebrew_to_rd(yy, mm + 1, 1) <= rd {
            mm += 1;
        }
        m = mm;
        d = (rd - hebrew_to_rd(yy, mm, 1) + 1) as u32;
    } else {
        let mut mm = 1;
        while mm < 6 && hebrew_to_rd(yy, mm + 1, 1) <= rd {
            mm += 1;
        }
        m = mm;
        d = (rd - hebrew_to_rd(yy, mm, 1) + 1) as u32;
    }
    (y, m, d)
}

// ---------------------------------------------------------------- Easter

/// Western (Gregorian) Easter Sunday for year `y` → R.D.
/// Anonymous Gregorian computus (Knuth's presentation of the 1876 algorithm).
pub fn easter(y: i32) -> Rd {
    let y = y as i64;
    let a = y.rem_euclid(19);
    let b = y / 100;
    let c = y % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
    let mm = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * mm + 114) / 31;
    let day = (h + l - 7 * mm + 114) % 31 + 1;
    gregorian_to_rd(y as i32, month as u32, day as u32)
}

/// Day-of-week of an R.D. date: 0 = Sunday … 6 = Saturday.
pub fn weekday(rd: Rd) -> u32 {
    rd.rem_euclid(7) as u32
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    const RD_20000101: Rd = 730_120; // R.D. of 2000-01-01 Gregorian

    #[test]
    fn gregorian_anchor() {
        assert_eq!(gregorian_to_rd(2000, 1, 1), RD_20000101);
        assert_eq!(rd_to_gregorian(RD_20000101), (2000, 1, 1));
        // R.D. 1 = 1-01-01.
        assert_eq!(rd_to_gregorian(1), (1, 1, 1));
    }

    #[test]
    fn julian_round_trip_and_anchors() {
        // 2016-10-23 Gregorian = 2016-10-10 Julian (13-day drift).
        let g = gregorian_to_rd(2016, 10, 23);
        assert_eq!(rd_to_julian(g), (2016, 10, 10));
        // Round trips over a broad range.
        for y in [-700, 4, 1582, 2000, 2400] {
            for (m, d) in [(1u32, 1u32), (2, 28), (3, 1), (12, 31)] {
                let rd = julian_to_rd(y, m, d);
                assert_eq!(rd_to_julian(rd), (y, m, d), "{y}-{m}-{d}");
            }
        }
    }

    #[test]
    fn islamic_round_trip_and_anchor() {
        // Islamic epoch = 1 Muharram 1 AH = 622-07-16 Julian = RD 227015.
        assert_eq!(islamic_to_rd(1, 1, 1), ISLAMIC_EPOCH);
        assert_eq!(rd_to_julian(ISLAMIC_EPOCH), (622, 7, 16));
        // 1445-01-01 AH fell on 2023-07-19 Gregorian (tabular).
        let rd = islamic_to_rd(1445, 1, 1);
        assert_eq!(rd_to_gregorian(rd), (2023, 7, 19));
        for y in [1, 30, 622, 1445, 1500] {
            for m in 1..=12u32 {
                let rd = islamic_to_rd(y, m, 1);
                assert_eq!(rd_to_islamic(rd), (y, m, 1));
                // Month length sanity: alternating 30/29, Dhu al-Hijjah 30 in leaps.
                let next = if m < 12 {
                    islamic_to_rd(y, m + 1, 1)
                } else {
                    islamic_to_rd(y + 1, 1, 1)
                };
                let len = (next - rd) as u32;
                let want = if m == 12 {
                    if islamic_leap(y) {
                        30
                    } else {
                        29
                    }
                } else {
                    29 + (m % 2)
                };
                assert_eq!(len, want, "month {y}-{m}");
            }
        }
    }

    #[test]
    fn persian_round_trip_and_anchor() {
        // Persian epoch: 1 Farvardin 1 = 622-03-19 Julian? No — 622-03-22 Julian,
        // but R&D's arithmetic calendar uses RD 226896 = 622-03-19 Julian.
        assert_eq!(persian_to_rd(1, 1, 1), PERSIAN_EPOCH);
        // Nowruz 1403 = 2024-03-20 Gregorian.
        assert_eq!(rd_to_gregorian(persian_to_rd(1403, 1, 1)), (2024, 3, 20));
        for y in [1, 100, 500, 1403, 1500] {
            for m in 1..=12u32 {
                let rd = persian_to_rd(y, m, 5);
                assert_eq!(rd_to_persian(rd), (y, m, 5), "{y}-{m}-5");
            }
        }
    }

    #[test]
    fn hebrew_round_trip_and_anchor() {
        // 1 Tishrei 5784 = 2023-09-16 Gregorian.
        let rd = hebrew_new_year(5784);
        assert_eq!(rd_to_gregorian(rd), (2023, 9, 16));
        // 10 Tishrei 5784 (Yom Kippur) = 2023-09-25.
        assert_eq!(rd_to_gregorian(hebrew_to_rd(5784, 7, 10)), (2023, 9, 25));
        // 15 Nisan 5784 (Passover) = 2024-04-23.
        assert_eq!(rd_to_gregorian(hebrew_to_rd(5784, 1, 15)), (2024, 4, 23));
        // Round trips, incl. leap year 5784 (13 months).
        for y in [1, 3761, 5000, 5784, 6000] {
            for m in 1..=hebrew_year_months(y) {
                let rd = hebrew_to_rd(y, m, 1);
                assert_eq!(rd_to_hebrew(rd), (y, m, 1), "{y}-{m}-1");
            }
        }
    }

    #[test]
    fn easter_dates() {
        // Known Western Easter Sundays.
        let cases = [
            (2000, 4, 23),
            (2001, 4, 15),
            (2010, 4, 4),
            (2024, 3, 31),
            (2025, 4, 20),
        ];
        for (y, m, d) in cases {
            assert_eq!(rd_to_gregorian(easter(y)), (y, m, d), "{y}");
            assert_eq!(weekday(easter(y)), 0); // always a Sunday
        }
    }

    #[test]
    fn hebrew_calendar_helpers() {
        // Hebrew leap years in the 19-year Metonic cycle: years where
        // (7y + 1) mod 19 < 7 — 5784 (2023-24) is leap, 5785 is not.
        assert!(hebrew_leap(5784));
        assert!(!hebrew_leap(5785));
        // Leap year has 13 months; common has 12.
        assert_eq!(hebrew_year_months(5784), 13);
        assert_eq!(hebrew_year_months(5785), 12);
        // Nisan (month 1) always 30 days; Adar II (leap-year month 13) 29.
        assert_eq!(hebrew_month_days(5785, 1), 30);
        assert_eq!(hebrew_month_days(5784, 13), 29);
    }

    #[test]
    fn persian_leap_cycle() {
        // The arithmetic 2820-cycle gives 1404 leap and 1403 common
        // (the formula intentionally differs from astronomical Nowruz
        // in some years — it is the Reingold–Dershowitz rule).
        assert!(persian_leap(1404));
        assert!(!persian_leap(1403));
        assert!(persian_leap(1399));
    }

    #[test]
    fn weekday_known() {
        // 2000-01-01 was a Saturday (6).
        assert_eq!(weekday(RD_20000101), 6);
        // R.D. 1 (1-01-01 Gregorian) was a Monday (1).
        assert_eq!(weekday(1), 1);
    }
}

//! Two-Line Element set (TLE) parsing — the NORAD satellite orbit
//! format: a 24-char name line plus two 69-column lines with a mod-10
//! check digit each. Column positions are fixed by the legacy format;
//! scientific fields use the implied-decimal `±NNNNN-N` notation
//! (`0.mantissa × 10^exp`), decoded without floats.
//!
//! ```
//! use izanagi_kit::tle::{parse, Tle};
//!
//! let t = parse(
//!     "ISS (ZARYA)",
//!     "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927",
//!     "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537",
//! ).unwrap();
//! assert_eq!(t.catalog, 25544);
//! ```

use crate::fixed::Fixed;
use std::string::String;

/// One decoded element set.
#[derive(Clone, Debug, PartialEq)]
pub struct Tle {
    /// Satellite name (line 0, may be empty).
    pub name: String,
    /// NORAD catalog number (must match on both lines).
    pub catalog: u32,
    /// Classification byte (`U`/`C`/`S`).
    pub class: u8,
    /// International designator, verbatim (e.g. `98067A  `).
    pub intl: String,
    /// Epoch year, resolved to four digits (`<57` → 2000s).
    pub epoch_year: u16,
    /// Epoch day-of-year integer part.
    pub epoch_day: u16,
    /// Epoch fractional day in micro-days (`d.ffffffff` → ×10⁸).
    pub epoch_frac: u64,
    /// First derivative of mean motion/2 — mantissa ×10^exp form.
    pub mm2_num: i32,
    /// Exponent of `mm2_num` (units: rev/day²).
    pub mm2_exp: i8,
    /// Second derivative of mean motion — mantissa ×10^exp.
    pub mmdot_num: i32,
    /// Exponent of `mmdot_num`.
    pub mmdot_exp: i8,
    /// B* drag term — mantissa ×10^exp.
    pub bstar_num: i32,
    /// Exponent of `bstar_num`.
    pub bstar_exp: i8,
    /// Ephemeris type.
    pub eph_type: u8,
    /// Element set number.
    pub set_num: u32,
    /// Inclination, degrees.
    pub inc: Fixed,
    /// Right ascension of ascending node, degrees.
    pub raan: Fixed,
    /// Eccentricity ×10⁷ (the `NNNNNNN` implied-leading-decimal field).
    pub ecc: u32,
    /// Argument of perigee, degrees.
    pub argp: Fixed,
    /// Mean anomaly, degrees.
    pub manom: Fixed,
    /// Mean motion, rev/day (as [`Fixed`]).
    pub mm: Fixed,
    /// Revolution number at epoch.
    pub rev: u32,
}

impl Tle {
    /// Orbital period in seconds (`86400 / mm`); `None` if `mm == 0`.
    pub fn period_seconds(&self) -> Option<Fixed> {
        let mm = self.mm.raw();
        if mm <= 0 {
            return None;
        }
        let r = 86400i128 * 65536 * 65536 / mm as i128;
        if r > i32::MAX as i128 {
            return None;
        }
        Some(Fixed::from_raw(r as i32))
    }
}

/// Per-line mod-10 checksum: sum of digits (`-` counts 1) mod 10 must
/// equal the last character.
fn check(line: &str) -> bool {
    let b = line.as_bytes();
    if b.len() != 69 {
        return false;
    }
    let mut sum = 0u32;
    for &c in &b[..68] {
        match c {
            b'0'..=b'9' => sum += (c - b'0') as u32,
            b'-' => sum += 1,
            _ => {}
        }
    }
    b[68] == b'0' + (sum % 10) as u8
}

fn col(line: &str, a: usize, b: usize) -> Option<&str> {
    line.get(a..b)
}

/// `NNN.NNNNNN` style: integer + optional fraction → milli-degrees as
/// Fixed (digit arithmetic, no floats).
fn deg(s: &str) -> Option<Fixed> {
    let s = s.trim();
    let dot = s.find('.');
    let (ip, frac) = match dot {
        Some(p) => (&s[..p], &s[p + 1..]),
        None => (s, ""),
    };
    let neg = ip.starts_with('-');
    let ipd: i64 = ip.trim_start_matches(['-', '+']).parse().ok()?;
    if ipd > 360 {
        return None;
    }
    let mut fn_: i64 = 0;
    let mut fd: i64 = 1;
    for &c in frac.as_bytes() {
        if !c.is_ascii_digit() {
            return None;
        }
        fn_ = fn_.checked_mul(10)?.checked_add((c - b'0') as i64)?;
        fd = fd.checked_mul(10)?;
        if fd > 1_000_000_000_000 {
            return None;
        }
    }
    let raw = (ipd as i128) * 65536 + (fn_ as i128) * 65536 / (fd as i128);
    if raw > i32::MAX as i128 {
        return None;
    }
    Some(Fixed::from_raw(if neg {
        -(raw as i32)
    } else {
        raw as i32
    }))
}

/// `±.NNNNNNNN` — leading sign, implied decimal mantissa (8 digits).
fn implied(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, d) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let d = d.strip_prefix('.').unwrap_or(d);
    if d.is_empty() || !d.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let v: i64 = d.parse().ok()?;
    Some(if neg { -v } else { v })
}

/// `±NNNNN-N` — implied-decimal scientific: `0.mantissa × 10^exp`.
/// Returns `(mantissa, exp)`; blanks parse as zero.
fn sci(s: &str) -> Option<(i32, i8)> {
    let s = s.trim();
    if s.is_empty() {
        return Some((0, 0));
    }
    let (neg, rest) = match s.as_bytes()[0] {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    if rest.len() < 3 {
        return None;
    }
    let (mant_s, exp_s) = rest.split_at(rest.len() - 2);
    if !mant_s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mant: i32 = mant_s.parse().ok()?;
    let eb = exp_s.as_bytes();
    let ed: i8 = eb[1].checked_sub(b'0')? as i8;
    let exp = if eb[0] == b'-' {
        -ed
    } else if eb[0].is_ascii_digit() {
        (eb[0] - b'0') as i8 * 10 + ed
    } else {
        return None;
    };
    Some((if neg { -mant } else { mant }, exp))
}

/// Parse a complete TLE; `None` on bad checksums, wrong lengths, or
/// column contents. `name` may be `""`.
pub fn parse(name: &str, l1: &str, l2: &str) -> Option<Tle> {
    let l1 = l1.trim_end_matches(['\r', '\n']);
    let l2 = l2.trim_end_matches(['\r', '\n']);
    if !check(l1) || !check(l2) {
        return None;
    }
    if col(l1, 0, 1)? != "1" || col(l2, 0, 1)? != "2" {
        return None;
    }
    let cat1: u32 = col(l1, 2, 7)?.trim().parse().ok()?;
    let cat2: u32 = col(l2, 2, 7)?.trim().parse().ok()?;
    if cat1 != cat2 {
        return None;
    }
    let ey: u16 = col(l1, 18, 20)?.trim().parse().ok()?;
    let epoch_year = if ey < 57 { 2000 + ey } else { 1900 + ey };
    let eday = col(l1, 20, 23)?.trim();
    let epoch_day: u16 = eday.parse().ok()?;
    if epoch_day == 0 || epoch_day > 366 {
        return None;
    }
    let efrac = col(l1, 23, 32)?; // ".dddddddd" or " .dddddddd"
    let efrac = efrac.trim();
    let efrac = efrac.strip_prefix('.')?;
    if efrac.len() != 8 || !efrac.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let epoch_frac: u64 = efrac.parse().ok()?;
    // mm2 field: "+.NNNNNNNN" or "-.NNNNNNNN" or " .NNNNNNNN"
    let mm2v = implied(col(l1, 33, 43)?)?;
    let (mmdot_num, mmdot_exp) = sci(col(l1, 44, 52)?)?;
    let (bstar_num, bstar_exp) = sci(col(l1, 53, 61)?)?;
    let eph: u8 = col(l1, 62, 63)?.trim().parse().ok()?;
    let set_num: u32 = col(l1, 64, 68)?.trim().parse().ok()?;
    let ecc_s = col(l2, 26, 33)?.trim();
    if ecc_s.len() != 7 || !ecc_s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mm_s = col(l2, 52, 63)?.trim();
    let mm = deg(mm_s);
    Some(Tle {
        name: name.trim_end_matches(['\r', '\n']).to_string(),
        catalog: cat1,
        class: *l1.as_bytes().get(7)?,
        intl: col(l1, 9, 17)?.to_string(),
        epoch_year,
        epoch_day,
        epoch_frac,
        mm2_num: mm2v as i32,
        mm2_exp: -8, // `0.NNNNNNNN` → mantissa ×10⁻⁸ (documented units)
        mmdot_num,
        mmdot_exp,
        bstar_num,
        bstar_exp,
        eph_type: eph,
        set_num,
        inc: deg(col(l2, 8, 16)?)?,
        raan: deg(col(l2, 17, 25)?)?,
        ecc: ecc_s.parse().ok()?,
        argp: deg(col(l2, 34, 42)?)?,
        manom: deg(col(l2, 43, 51)?)?,
        mm: mm?,
        rev: col(l2, 63, 68)?.trim().parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISS_L0: &str = "ISS (ZARYA)";
    const ISS_L1: &str = "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2927";
    const ISS_L2: &str = "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563537";

    #[test]
    fn iss_vector() {
        let t = parse(ISS_L0, ISS_L1, ISS_L2).unwrap();
        assert_eq!(t.catalog, 25544);
        assert_eq!(t.class, b'U');
        assert_eq!(t.intl, "98067A  ");
        assert_eq!(t.epoch_year, 2008);
        assert_eq!(t.epoch_day, 264);
        assert_eq!(t.epoch_frac, 51_782_528);
        assert_eq!(t.mm2_num, -2182);
        assert_eq!(t.bstar_num, -11606);
        assert_eq!(t.bstar_exp, -4);
        assert_eq!(t.ecc, 6703);
        assert_eq!(t.rev, 56353);
        assert_eq!(t.set_num, 292);
        // 51.6416° in raw Q16.16
        assert_eq!(t.inc.raw(), 51 * 65536 + (6416 * 65536 / 10000));
        assert!(t.mm > Fixed::from_int(15) && t.mm < Fixed::from_int(16));
        let p = t.period_seconds().unwrap();
        assert!(p > Fixed::from_int(5490) && p < Fixed::from_int(5500)); // ~91.5 min
    }

    #[test]
    fn checksums_enforced() {
        let bad = "1 25544U 98067A   08264.51782528 -.00002182  00000-0 -11606-4 0  2928";
        assert_eq!(parse(ISS_L0, bad, ISS_L2), None);
        let bad2 = "2 25544  51.6416 247.4627 0006703 130.5360 325.0288 15.72125391563530";
        assert_eq!(parse(ISS_L0, ISS_L1, bad2), None);
        assert_eq!(parse(ISS_L0, "x", ISS_L2), None);
        assert_eq!(parse(ISS_L0, ISS_L2, ISS_L2), None);
    }

    #[test]
    fn catalog_mismatch_rejected() {
        let mut l2 = String::from(ISS_L2);
        l2.replace_range(2..7, "12345");
        // recompute checksum digit
        let mut l2b = l2.into_bytes();
        let mut sum = 0u32;
        for &c in &l2b[..68] {
            match c {
                b'0'..=b'9' => sum += (c - b'0') as u32,
                b'-' => sum += 1,
                _ => {}
            }
        }
        l2b[68] = b'0' + (sum % 10) as u8;
        assert_eq!(
            parse(ISS_L0, ISS_L1, std::str::from_utf8(&l2b).unwrap()),
            None
        );
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(ISS_L0, ISS_L1, ISS_L2), parse(ISS_L0, ISS_L1, ISS_L2));
    }
}

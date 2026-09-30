//! Minor Planet Center (`*.mpc`/obs report) 80-column record census.
//!
//! Fixed-width observation lines: packed number (cols 1-5), packed
//! provisional designation (6-12), discovery `*` (13), notes (15-16),
//! `YYYY MM DD` date (18-24), `HH MM SS` RA (33-40), signed Dec (45-),
//! magnitude+band (66-71) and a 3-char observatory code (78-80).
//!
//! ```
//! let s = b"      K20A0A*    2020 01 01     12 34 56    +12 34 56             15 V       703\n00001            2020 02 03     01 23 45    -01 23 45                        711\n";
//! assert!(izanagi_kit::mpc::detect(s));
//! let m = izanagi_kit::mpc::Mpc::parse(s).unwrap();
//! assert_eq!(m.records, 2);
//! assert_eq!(m.numbered, 1);
//! assert_eq!(m.provisional, 1);
//! assert_eq!(m.discoveries, 1);
//! assert_eq!(m.codes, 2);
//! ```

/// Parsed census of an MPC observation file.
#[derive(Debug, Clone)]
pub struct Mpc {
    /// Fixed-width observation records.
    pub records: usize,
    /// Lines with a numeric packed permanent number.
    pub numbered: usize,
    /// Lines with a packed provisional designation.
    pub provisional: usize,
    /// Lines with a `*` discovery flag.
    pub discoveries: usize,
    /// Lines with note columns populated.
    pub notes: usize,
    /// Lines whose leading char is a letter (packed special body).
    pub comets: usize,
    /// Lines matching the `YYYY MM` date shape.
    pub dates: usize,
    /// Lines with both RA and signed-Dec fields present.
    pub radec: usize,
    /// Lines with a magnitude field.
    pub mags: usize,
    /// Lines with an alphabetic magnitude band.
    pub bands: usize,
    /// Distinct 3-char observatory codes.
    pub codes: usize,
    /// Smallest observed year, or 0.
    pub first_year: usize,
}

fn digit(l: &str, a: usize, b: usize) -> bool {
    l.as_bytes()
        .get(a..b)
        .is_some_and(|w| !w.is_empty() && w.iter().all(|c| c.is_ascii_digit()))
}

fn date_ok(l: &str) -> bool {
    l.len() >= 75 && digit(l, 17, 21) && l.as_bytes()[21] == b' ' && digit(l, 22, 24)
}

fn ra_ok(l: &str) -> bool {
    l.len() >= 75
        && digit(l, 32, 34)
        && l.as_bytes()[34] == b' '
        && digit(l, 35, 37)
        && l.as_bytes()[37] == b' '
        && digit(l, 38, 40)
}

fn shape_ok(t: &str) -> bool {
    let mut saw = false;
    for l in t.lines() {
        if l.trim().is_empty() {
            continue;
        }
        if l.len() < 75 || !date_ok(l) || !ra_ok(l) {
            return false;
        }
        saw = true;
    }
    saw
}

/// Reports whether `b` looks like an MPC observation file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    shape_ok(t)
}

impl Mpc {
    /// Parses `b` as an MPC file, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !shape_ok(t) {
            return None;
        }
        let mut m = Mpc {
            records: 0,
            numbered: 0,
            provisional: 0,
            discoveries: 0,
            notes: 0,
            comets: 0,
            dates: 0,
            radec: 0,
            mags: 0,
            bands: 0,
            codes: 0,
            first_year: 0,
        };
        let mut codes = std::collections::BTreeSet::new();
        let mut year: Option<usize> = None;
        for l in t.lines() {
            if l.trim().is_empty() {
                continue;
            }
            m.records += 1;
            let num = l[0..5].trim();
            if !num.is_empty() && num.bytes().all(|c| c.is_ascii_digit()) {
                m.numbered += 1;
            }
            if !l[5..12].trim().is_empty() {
                m.provisional += 1;
            }
            if l.as_bytes()[12] == b'*' {
                m.discoveries += 1;
            }
            if !l[14..16].trim().is_empty() {
                m.notes += 1;
            }
            if l.as_bytes()[0].is_ascii_alphabetic() {
                m.comets += 1;
            }
            if date_ok(l) {
                m.dates += 1;
                let y: usize = l[17..21].trim().parse().unwrap_or(0);
                year = Some(year.map_or(y, |v| v.min(y)));
            }
            if ra_ok(l) && matches!(l.as_bytes()[44], b'+' | b'-') {
                m.radec += 1;
            }
            let mag = l[65..71.min(l.len())].trim();
            if !mag.is_empty() {
                m.mags += 1;
            }
            if l[65..71.min(l.len())]
                .bytes()
                .any(|c| c.is_ascii_alphabetic())
            {
                m.bands += 1;
            }
            if let Some(code) = l.split_whitespace().last() {
                if code.len() == 3 && code.bytes().all(|c| c.is_ascii_alphanumeric()) {
                    codes.insert(code);
                }
            }
        }
        m.codes = codes.len();
        m.first_year = year.unwrap_or(0);
        Some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"      K20A0A*    2020 01 01     12 34 56    +12 34 56             15 V       703\n00001            2020 02 03     01 23 45    -01 23 45                        711\n";

    #[test]
    fn parses_mpc() {
        assert!(detect(S));
        let m = Mpc::parse(S).unwrap();
        assert_eq!(m.records, 2);
        assert_eq!(m.numbered, 1);
        assert_eq!(m.provisional, 1);
        assert_eq!(m.discoveries, 1);
        assert_eq!(m.codes, 2);
    }

    #[test]
    fn rejects_non_mpc() {
        assert!(!detect(b"just text"));
        assert!(Mpc::parse(b"").is_none());
    }
}

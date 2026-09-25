//! NMEA 0183 sentence parsing — the GPS serial format: `$TALKER`
//! (GP=GPS, GN=GNSS, GL=GLONASS, …) + sentence name + comma fields +
//! `*XX` XOR checksum. Parsing is strict on the checksum and total —
//! malformed lines return `None`.
//!
//! Typed accessors decode the two workhorse sentences:
//! [`gga`] → fix quality/position/altitude, [`rmc`] → recommended
//! minimum (position, speed over ground in knots, course, UTC
//! date/time). Angles are [`Fixed`] degrees; `ddmm.mmmm` is converted
//! to degrees by decimal-digit arithmetic — no floats anywhere.
//!
//! ```
//! use izanagi_kit::nmea::{parse, gga};
//!
//! let s = parse("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47")
//!     .unwrap();
//! let fix = gga(&s).unwrap();
//! assert_eq!(fix.sats, 8);
//! ```

use crate::fixed::Fixed;
use std::string::String;
use std::vec::Vec;

/// One decoded sentence: `$GPGGA,…*CS` → `talker="GP"`,
/// `kind="GGA"`, `fields=[…]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Sentence {
    /// Two-char talker id (`GP`, `GN`, `GL`, `IN`, …).
    pub talker: String,
    /// Sentence kind after the talker (`GGA`, `RMC`, `GSV`, …).
    pub kind: String,
    /// Comma-separated fields (may be empty strings).
    pub fields: Vec<String>,
}

/// XOR of every byte between `$` and `*` — the NMEA checksum.
pub fn checksum(body: &[u8]) -> u8 {
    body.iter().fold(0u8, |a, &b| a ^ b)
}

/// Parse one line `$BODY*CS` (or `!BODY*CS`). `None` on bad framing,
/// bad hex, or checksum mismatch. Checksum may be omitted entirely
/// (some emitters do), in which case it is not verified.
pub fn parse(line: &str) -> Option<Sentence> {
    let l = line.trim_end_matches(['\r', '\n']);
    let b = l.as_bytes();
    if b.len() < 6 || !(b[0] == b'$' || b[0] == b'!') {
        return None;
    }
    let star = l.rfind('*');
    let (body, fields_end) = match star {
        Some(p) => {
            if b.len() < p + 3 {
                return None;
            }
            let hi = (b[p + 1] as char).to_digit(16)?;
            let lo = (b[p + 2] as char).to_digit(16)?;
            if checksum(&b[1..p]) != ((hi << 4 | lo) as u8) {
                return None;
            }
            (&l[1..p], p)
        }
        None => (&l[1..], b.len()),
    };
    let _ = fields_end;
    let head_end = body.find(',')?;
    let (head, rest) = body.split_at(head_end);
    if head.len() < 5 {
        return None;
    }
    let fields = rest[1..]
        .split(',')
        .map(String::from)
        .collect::<Vec<String>>();
    Some(Sentence {
        talker: head[..2].to_string(),
        kind: head[2..].to_string(),
        fields,
    })
}

/// Build a canonical `$talker kind,f1,…*CS` line (LF-free).
pub fn build(talker: &str, kind: &str, fields: &[&str]) -> String {
    let mut body = talker.to_string();
    body.push_str(kind);
    for f in fields {
        body.push(',');
        body.push_str(f);
    }
    std::format!("${}*{:02X}", body, checksum(body.as_bytes()))
}

/// Decoded GGA fix.
#[derive(Clone, Debug, PartialEq)]
pub struct Gga {
    /// Latitude, degrees (negative = south).
    pub lat: Fixed,
    /// Longitude, degrees (negative = west).
    pub lon: Fixed,
    /// UTC time of fix as seconds-since-midnight×1000.
    pub time_ms: u64,
    /// Fix quality (0=invalid, 1=GPS, 2=DGPS, …).
    pub quality: u32,
    /// Satellites in use.
    pub sats: u32,
    /// Horizontal dilution of precision.
    pub hdop: Fixed,
    /// Altitude above MSL (metres, decoded from `,M`).
    pub alt: Fixed,
}

/// Decode a GGA sentence; `None` for wrong kind or bad fields.
pub fn gga(s: &Sentence) -> Option<Gga> {
    if s.kind != "GGA" {
        return None;
    }
    let f = &s.fields;
    if f.len() < 10 {
        return None;
    }
    Some(Gga {
        time_ms: parse_time(&f[0])?,
        lat: parse_latlon(&f[1], &f[2], 2)?,
        lon: parse_latlon(&f[3], &f[4], 3)?,
        quality: f[5].parse().ok()?,
        sats: f[6].parse().ok()?,
        hdop: parse_num(&f[7])?,
        alt: parse_num(&f[8])?,
    })
}

/// Decoded RMC (recommended minimum).
#[derive(Clone, Debug, PartialEq)]
pub struct Rmc {
    /// Latitude degrees.
    pub lat: Fixed,
    /// Longitude degrees.
    pub lon: Fixed,
    /// UTC time as milliseconds since midnight.
    pub time_ms: u64,
    /// `A`ctive (fix) or `V`oid.
    pub active: bool,
    /// Speed over ground, knots.
    pub speed_kn: Fixed,
    /// Course over ground, degrees.
    pub course: Fixed,
    /// UTC date as `YYMMDD`.
    pub date: u64,
}

/// Decode an RMC sentence.
pub fn rmc(s: &Sentence) -> Option<Rmc> {
    if s.kind != "RMC" {
        return None;
    }
    let f = &s.fields;
    if f.len() < 9 {
        return None;
    }
    Some(Rmc {
        time_ms: parse_time(&f[0])?,
        active: f[1] == "A",
        lat: parse_latlon(&f[2], &f[3], 2)?,
        lon: parse_latlon(&f[4], &f[5], 3)?,
        speed_kn: parse_num(&f[6])?,
        course: parse_num(&f[7])?,
        date: f[8].parse().ok()?,
    })
}

/// `hhmmss.sss` → milliseconds since midnight.
fn parse_time(s: &str) -> Option<u64> {
    if s.len() < 6 {
        return None;
    }
    let h: u64 = s[0..2].parse().ok()?;
    let m: u64 = s[2..4].parse().ok()?;
    let sec: u64 = s[4..6].parse().ok()?;
    if h > 23 || m > 59 || sec > 60 {
        return None;
    }
    let ms = if s.len() > 7 && s.as_bytes()[6] == b'.' {
        let mut v: u64 = 0;
        let mut d: u32 = 0;
        for &c in &s.as_bytes()[7..] {
            if !c.is_ascii_digit() {
                return None;
            }
            if d < 3 {
                v = v * 10 + (c - b'0') as u64;
            }
            d += 1;
        }
        while d < 3 {
            v *= 10;
            d += 1;
        }
        v / (10u64).pow(d.saturating_sub(3))
    } else {
        0
    };
    Some((h * 3600 + m * 60 + sec) * 1000 + ms)
}

/// `ddmm.mmmm`/`dddmm.mmmm` + hemisphere → signed degrees.
/// `deg_digits` is 2 for latitude, 3 for longitude.
fn parse_latlon(num: &str, hemi: &str, deg_digits: usize) -> Option<Fixed> {
    if !matches!(hemi, "N" | "S" | "E" | "W") {
        return None;
    }
    let neg = num.starts_with('-');
    let s = num.trim_start_matches(['-', '+']);
    let dot = s.find('.');
    let intpart = match dot {
        Some(p) => &s[..p],
        None => s,
    };
    // The last two integer digits are whole minutes, the rest degrees.
    if intpart.len() < deg_digits + 2 || intpart.len() != deg_digits + 2 {
        return None;
    }
    let (deg_s, min_s) = intpart.split_at(deg_digits);
    let deg: i64 = deg_s.parse().ok()?;
    let mw: i64 = min_s.parse().ok()?;
    if deg > 180 || mw >= 60 {
        return None;
    }
    let mut frac_num: i64 = 0;
    let mut frac_den: i64 = 1;
    if let Some(p) = dot {
        for &c in &s.as_bytes()[p + 1..] {
            if !c.is_ascii_digit() {
                return None;
            }
            frac_num = frac_num.checked_mul(10)?.checked_add((c - b'0') as i64)?;
            frac_den = frac_den.checked_mul(10)?;
            if frac_den > 1_000_000_000_000 {
                return None;
            }
        }
    }
    // raw = deg*65536 + minutes/60*65536 in one i128 division
    let min_raw =
        ((mw as i128) * (frac_den as i128) + frac_num as i128) * 65536 / (frac_den as i128) / 60;
    let total = deg as i128 * 65536 + min_raw;
    if total > i32::MAX as i128 {
        return None;
    }
    let v = Fixed::from_raw(total as i32);
    Some(if neg || hemi == "S" || hemi == "W" {
        -v
    } else {
        v
    })
}

/// Decimal digit string → Fixed (no float). Rejects overflow.
fn parse_num(s: &str) -> Option<Fixed> {
    let b = s.as_bytes();
    let mut i = 0;
    let neg = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    let mut raw: i64 = 0;
    let mut frac = false;
    let mut frac_den: i64 = 1;
    let mut saw = false;
    while i < b.len() {
        match b[i] {
            b'0'..=b'9' => {
                let d = (b[i] - b'0') as i64;
                raw = raw.checked_mul(10)?.checked_add(d)?;
                if frac {
                    frac_den = frac_den.checked_mul(10)?;
                }
                if raw > (i32::MAX as i64) * frac_den {
                    return None;
                }
                saw = true;
            }
            b'.' if !frac => frac = true,
            _ => return None,
        }
        i += 1;
    }
    if !saw {
        return None;
    }
    let v = raw.checked_mul(65536)? / frac_den;
    if v > i32::MAX as i64 || v < i32::MIN as i64 {
        return None;
    }
    Some(Fixed::from_raw(if neg { -(v as i32) } else { v as i32 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gga_vector() {
        let s = parse("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47").unwrap();
        assert_eq!(
            (&s.talker, &s.kind),
            (&"GP".to_string(), &"GGA".to_string())
        );
        let g = gga(&s).unwrap();
        assert_eq!(g.sats, 8);
        assert_eq!(g.quality, 1);
        assert_eq!(g.time_ms, 12 * 3600 * 1000 + 35 * 60 * 1000 + 19 * 1000);
        // 48°07.038' = 48 + 7.038/60 deg
        let expect = Fixed::from_raw((48 * 65536) + (7 * 65536 + 38 * 65536 / 1000) / 60);
        assert_eq!(g.lat, expect);
        assert!(g.lon > Fixed::from_int(11) && g.lon < Fixed::from_int(12));
        assert!(g.alt > Fixed::from_int(545) && g.alt < Fixed::from_int(546));
    }

    #[test]
    fn rmc_vector() {
        let s =
            parse("$GPRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*68").unwrap();
        let r = rmc(&s).unwrap();
        assert!(r.active);
        assert_eq!(r.date, 191194);
        assert!(r.lat > Fixed::from_int(49));
        assert!(r.lon < Fixed::ZERO); // W → negative
        assert!(r.course > Fixed::from_int(54));
    }

    #[test]
    fn checksum_enforced() {
        assert_eq!(
            parse("$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*48"),
            None
        );
        assert_eq!(parse("GPGGA,nope"), None);
        assert_eq!(parse("$GP"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn build_roundtrips() {
        let line = build("GP", "GLL", &["4916.45", "N", "12311.12", "W"]);
        let s = parse(&line).unwrap();
        assert_eq!(s.kind, "GLL");
        assert_eq!(s.fields.len(), 4);
        assert!(build("GN", "TXT", &["01", "01", "02", "hello"]).len() > 4);
    }

    #[test]
    fn no_checksum_accepted() {
        let s = parse("$GPTXT,01,01,02,hi").unwrap();
        assert_eq!(s.fields[3], "hi");
    }

    #[test]
    fn determinism_twice() {
        let l = "$GPRMC,225446,A,4916.45,N,12311.12,W,000.5,054.7,191194,020.3,E*68";
        assert_eq!(parse(l), parse(l));
        assert_eq!(rmc(&parse(l).unwrap()), rmc(&parse(l).unwrap()));
    }
}

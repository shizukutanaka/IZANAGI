//! DTED (MIL-PRF-89020) — DoD digital terrain elevation cell.
//!
//! The User Header Label occupies the first 80 bytes: `UHL` + version
//! digit, longitude `DDDMMSSH` at 4..12, latitude `DDMMSSH` at 12..20.
//!
//! ```
//! let mut d = vec![b' '; 3428];
//! d[..4].copy_from_slice(b"UHL1");
//! d[4..12].copy_from_slice(b"1394500E"); // 139°45′00″E
//! d[12..20].copy_from_slice(b"0360000N"); // 36°00′00″N
//! let t = izanagi_kit::dted::parse(&d).unwrap();
//! assert_eq!(t.version, 1);
//! assert_eq!(t.lon_hemi, 'E');
//! assert_eq!(t.lat_hemi, 'N');
//! ```

/// Parsed DTED user header label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dted {
    /// UHL version digit (usually `1`).
    pub version: u8,
    /// Longitude degrees.
    pub lon_deg: u16,
    /// Longitude minutes.
    pub lon_min: u8,
    /// Longitude seconds.
    pub lon_sec: u8,
    /// Longitude hemisphere (`'E'` or `'W'`).
    pub lon_hemi: char,
    /// Latitude degrees.
    pub lat_deg: u16,
    /// Latitude minutes.
    pub lat_min: u8,
    /// Latitude seconds.
    pub lat_sec: u8,
    /// Latitude hemisphere (`'N'` or `'S'`).
    pub lat_hemi: char,
}

fn num(d: &[u8], a: usize, b: usize) -> Option<u32> {
    let s = std::str::from_utf8(d.get(a..b)?).ok()?;
    if !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Parse the DTED header; `None` without `UHL` + valid coordinates.
pub fn parse(d: &[u8]) -> Option<Dted> {
    if d.len() < 20 || !d.starts_with(b"UHL") {
        return None;
    }
    let version = d[3];
    if !version.is_ascii_digit() {
        return None;
    }
    let lon_deg = num(d, 4, 7)? as u16;
    let lon_min = num(d, 7, 9)? as u8;
    let lon_sec = num(d, 9, 11)? as u8;
    let lon_hemi = *d.get(11)? as char;
    let lat_deg = num(d, 12, 15)? as u16;
    let lat_min = num(d, 15, 17)? as u8;
    let lat_sec = num(d, 17, 19)? as u8;
    let lat_hemi = *d.get(19)? as char;
    if lon_deg > 180 || lat_deg > 90 || lon_min > 59 || lat_min > 59 || lon_sec > 59 || lat_sec > 59
    {
        return None;
    }
    if !matches!(lon_hemi, 'E' | 'W') || !matches!(lat_hemi, 'N' | 'S') {
        return None;
    }
    Some(Dted {
        version: version - b'0',
        lon_deg,
        lon_min,
        lon_sec,
        lon_hemi,
        lat_deg,
        lat_min,
        lat_sec,
        lat_hemi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uhl(ver: u8, lon: &[u8; 8], lat: &[u8; 8]) -> Vec<u8> {
        let mut d = vec![b' '; 80];
        d[..3].copy_from_slice(b"UHL");
        d[3] = ver;
        d[4..12].copy_from_slice(lon);
        d[12..20].copy_from_slice(lat);
        d
    }

    #[test]
    fn basic() {
        let t = parse(&uhl(b'1', b"0013000W", b"0513000N")).unwrap();
        assert_eq!(
            (t.lon_deg, t.lon_min, t.lon_sec, t.lon_hemi),
            (1, 30, 0, 'W')
        );
        assert_eq!((t.lat_deg, t.lat_hemi), (51, 'N'));
        assert_eq!(t.version, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"XYZ1").is_none());
        assert!(parse(&uhl(b'1', b"1810000E", b"0000000N")).is_none()); // lon > 180
        assert!(parse(&uhl(b'1', b"0013000Q", b"0513000N")).is_none()); // bad hemi
        assert!(parse(&uhl(b'x', b"0013000W", b"0513000N")).is_none()); // bad version
    }
}

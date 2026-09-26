//! miniSEED fixed header (SEED/FSDH data record) parsing.
//!
//! Fixed header (48 bytes): `seq(6 ASCII) data_hdr_ind(1) reserved(1)
//! station(5) loc(2) chan(3) net(2) year(u16BE) doy(u16BE)
//! hour min sec unused ms(u16BE) nsamp(u16BE) rate_factor(i16BE)
//! rate_mul(i16BE) act(1) io(1) qual(1) ncorr(1) time_corr(i32BE)
//! data_at(u16BE) blockette_at(u16BE)`.
//!
//! ```
//! use izanagi_kit::mseed::parse;
//!
//! let mut rec = vec![0u8; 64];
//! rec[..6].copy_from_slice(b"000001");  // sequence
//! rec[6] = b'D';                        // data indicator
//! rec[8..13].copy_from_slice(b"ABCDE"); // station
//! rec[13..15].copy_from_slice(b"00");   // loc
//! rec[15..18].copy_from_slice(b"HHZ");  // chan
//! rec[18..20].copy_from_slice(b"JP");   // net
//! rec[20] = 0x07; rec[21] = 0xE9;       // year 2025
//! rec[22] = 0x00; rec[23] = 0x64;       // doy 100
//! rec[30] = 0x00; rec[31] = 0x40;       // nsamp 64
//! rec[32] = 0x00; rec[33] = 0x64;       // rate factor 100
//! rec[34] = 0x00; rec[35] = 0x01;       // multiplier 1
//! rec[44] = 0x00; rec[45] = 0x40;       // data offset 64
//! rec[46] = 0x00; rec[47] = 0x30;       // blockette offset 48
//! let m = parse(&rec).unwrap();
//! assert_eq!(m.station(), "ABCDE");
//! assert_eq!(m.nsamp, 64);
//! ```

/// Parsed miniSEED fixed header.
pub struct Mseed<'a> {
    /// Sequence number (6 ASCII digits).
    pub seq: &'a [u8],
    /// Data header indicator (`D`/`R`/`Q` data, `M`/`V`/`E` header-only...).
    pub indicator: u8,
    /// Station code (5 bytes).
    pub station_code: &'a [u8],
    /// Location id (2 bytes).
    pub loc: &'a [u8],
    /// Channel code (3 bytes).
    pub chan: &'a [u8],
    /// Network code (2 bytes).
    pub net: &'a [u8],
    /// Year.
    pub year: u16,
    /// Day of year (1-366).
    pub doy: u16,
    /// Hour.
    pub hour: u8,
    /// Minute.
    pub min: u8,
    /// Second.
    pub sec: u8,
    /// Milliseconds.
    pub ms: u16,
    /// Sample count in this record.
    pub nsamp: u16,
    /// Sample-rate factor.
    pub rate_factor: i16,
    /// Sample-rate multiplier.
    pub rate_mul: i16,
    /// Number of time corrections.
    pub ncorr: u8,
    /// Data begin offset.
    pub data_at: u16,
    /// First blockette offset.
    pub blockette_at: u16,
}

impl Mseed<'_> {
    /// Station code as a `str` (ASCII).
    pub fn station(&self) -> &str {
        core::str::from_utf8(self.station_code).unwrap_or("")
    }
    /// Signed rate product `rate_factor × rate_mul` (treating 0 multiplier
    /// as 1). A positive value is samples per second; a negative value means
    /// seconds per sample, per SEED convention.
    pub fn rate_product(&self) -> i64 {
        let f = i64::from(self.rate_factor);
        let m = if self.rate_mul == 0 {
            1
        } else {
            i64::from(self.rate_mul)
        };
        f * m
    }
}

/// Parses a miniSEED fixed header (first 48 bytes).
pub fn parse(d: &[u8]) -> Option<Mseed<'_>> {
    if d.len() < 48 {
        return None;
    }
    if !d[..6]
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b' ')
    {
        return None;
    }
    let be16 = |o: usize| ((d[o] as u16) << 8) | d[o + 1] as u16;
    let be16s = |o: usize| be16(o) as i16;
    let year = be16(20);
    let doy = be16(22);
    if year == 0 || doy == 0 || doy > 366 {
        return None;
    }
    let hour = d[24];
    let min = d[25];
    let sec = d[26];
    if hour > 23 || min > 59 || sec > 60 {
        return None;
    }
    let data_at = be16(44);
    let blockette_at = be16(46);
    Some(Mseed {
        seq: &d[0..6],
        indicator: d[6],
        station_code: &d[8..13],
        loc: &d[13..15],
        chan: &d[15..18],
        net: &d[18..20],
        year,
        doy,
        hour,
        min,
        sec,
        ms: be16(28),
        nsamp: be16(30),
        rate_factor: be16s(32),
        rate_mul: be16s(34),
        ncorr: d[39],
        data_at,
        blockette_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> Vec<u8> {
        let mut r = vec![0u8; 512];
        r[..6].copy_from_slice(b"000123");
        r[6] = b'D';
        r[8..13].copy_from_slice(b"HIJ  ");
        r[13..15].copy_from_slice(b"10");
        r[15..18].copy_from_slice(b"BHZ");
        r[18..20].copy_from_slice(b"JP");
        r[20..22].copy_from_slice(&[0x07, 0xE8]); // 2024
        r[22..24].copy_from_slice(&[0x01, 0x2C]); // doy 300
        r[24] = 12;
        r[25] = 34;
        r[26] = 56;
        r[28..30].copy_from_slice(&[0x01, 0xF4]); // ms 500
        r[30..32].copy_from_slice(&[0x03, 0xE8]); // nsamp 1000
        r[32..34].copy_from_slice(&[0x00, 0xC8]); // factor 200
        r[34..36].copy_from_slice(&[0xFF, 0xFF]); // mult -1
        r[44..46].copy_from_slice(&[0x00, 0x40]); // data 64
        r[46..48].copy_from_slice(&[0x00, 0x30]); // blk 48
        r
    }

    #[test]
    fn parses() {
        let h = hdr();
        let m = parse(&h).unwrap();
        assert_eq!(m.seq, b"000123");
        assert_eq!(m.indicator, b'D');
        assert_eq!(m.station(), "HIJ  ");
        assert_eq!(m.chan, b"BHZ");
        assert_eq!(m.year, 2024);
        assert_eq!(m.doy, 300);
        assert_eq!(m.ms, 500);
        assert_eq!(m.nsamp, 1000);
        assert_eq!(m.rate_factor, 200);
        assert_eq!(m.rate_mul, -1);
        assert_eq!(m.rate_product(), -200);
        assert_eq!(m.data_at, 64);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0; 47]).is_none());
        let mut h = hdr();
        h[22] = 1; // doy 300 stays valid — make doy 0
        h[22] = 0;
        h[23] = 0;
        assert!(parse(&h).is_none());
        let mut h = hdr();
        h[25] = 99; // minute out of range
        assert!(parse(&h).is_none());
    }
}

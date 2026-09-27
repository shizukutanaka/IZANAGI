//! EDF — European Data Format (biosignal recordings).
//!
//! A 256-byte ASCII header (version, patient/recording ids, dates,
//! `header_bytes`, `num_records`, `duration`, `num_signals`) followed
//! by `ns`-wide signal descriptor blocks (16-byte labels first).
//!
//! ```
//! use izanagi_kit::edf::parse;
//!
//! let mut h = Vec::new();
//! h.extend_from_slice(b"0       ");                                   // version
//! h.extend_from_slice(b"patient                                                                         ");
//! h.extend_from_slice(b"recording                                                                       ");
//! h.extend_from_slice(b"01.01.00");                                    // startdate
//! h.extend_from_slice(b"12.00.00");                                    // starttime
//! h.extend_from_slice(b"512     ");                                    // header bytes
//! h.extend_from_slice(&[b' '; 44]);                                    // reserved
//! h.extend_from_slice(b"100     ");                                    // num records
//! h.extend_from_slice(b"8       ");                                    // duration (s, text)
//! h.extend_from_slice(b"2   ");                                        // signals
//! h.extend_from_slice(b"EEG Fpz         EMG             ");            // labels
//! while h.len() < 512 { h.push(b' '); }
//! let e = parse(&h).unwrap();
//! assert_eq!(e.num_signals, 2);
//! assert_eq!(e.labels[0], "EEG Fpz");
//! ```

/// A parsed EDF header.
#[derive(Clone, Debug)]
pub struct Edf {
    /// `version` field (always `"0"`).
    pub version: String,
    /// Patient identification (trimmed).
    pub patient: String,
    /// Recording identification.
    pub recording: String,
    /// `dd.mm.yy` start date, verbatim.
    pub startdate: String,
    /// `hh.mm.ss` start time, verbatim.
    pub starttime: String,
    /// Total header size in bytes (256 + 256×ns).
    pub header_bytes: u32,
    /// Number of data records.
    pub num_records: i64,
    /// Record duration, verbatim (ASCII seconds).
    pub duration: String,
    /// Number of signals.
    pub num_signals: u32,
    /// Per-signal labels (first `ns × 16` bytes of the signal block).
    pub labels: Vec<String>,
}

fn field(d: &[u8], a: usize, b: usize) -> Option<String> {
    let s = std::str::from_utf8(d.get(a..b)?).ok()?;
    Some(s.trim().to_string())
}

/// Parse an EDF header. `None` when too short or numeric fields are
/// malformed; signal labels included when `header_bytes` fits.
pub fn parse(d: &[u8]) -> Option<Edf> {
    if d.len() < 256 {
        return None;
    }
    let version = field(d, 0, 8)?;
    if version != "0" {
        return None;
    }
    let header_bytes = field(d, 184, 192)?.parse().ok()?;
    let num_records = field(d, 236, 244)?.parse().ok()?;
    let num_signals: u32 = field(d, 252, 256)?.parse().ok()?;
    if num_signals == 0 {
        return None;
    }
    let hb = header_bytes as usize;
    if hb < 256 || hb > d.len() {
        return None;
    }
    let ns = num_signals as usize;
    let label_bytes = ns.checked_mul(16)?;
    if 256 + label_bytes > hb {
        return None;
    }
    let mut labels = Vec::with_capacity(ns);
    for i in 0..ns {
        labels.push(field(d, 256 + i * 16, 256 + (i + 1) * 16)?);
    }
    Some(Edf {
        version,
        patient: field(d, 8, 88)?,
        recording: field(d, 88, 168)?,
        startdate: field(d, 168, 176)?,
        starttime: field(d, 176, 184)?,
        header_bytes,
        num_records,
        duration: field(d, 244, 252)?,
        num_signals,
        labels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> Vec<u8> {
        let mut h = Vec::new();
        h.extend_from_slice(b"0       ");
        h.extend_from_slice(b"P");
        h.extend_from_slice(&[b' '; 79]);
        h.extend_from_slice(b"R");
        h.extend_from_slice(&[b' '; 79]);
        h.extend_from_slice(b"02.03.04");
        h.extend_from_slice(b"05.06.07");
        h.extend_from_slice(b"768     ");
        h.extend_from_slice(&[b' '; 44]);
        h.extend_from_slice(b"-1      ");
        h.extend_from_slice(b"30      ");
        h.extend_from_slice(b"2   ");
        h.extend_from_slice(b"EEG Fpz         EMG             ");
        while h.len() < 768 {
            h.push(b' ');
        }
        h
    }

    #[test]
    fn parses() {
        let e = parse(&hdr()).unwrap();
        assert_eq!(e.num_records, -1);
        assert_eq!(e.num_signals, 2);
        assert_eq!(e.labels, vec!["EEG Fpz", "EMG"]);
        assert_eq!(e.startdate, "02.03.04");
        assert_eq!(e.header_bytes, 768);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[b' '; 100]).is_none());
        let mut h = hdr();
        h[0] = b'1';
        assert!(parse(&h).is_none());
        let mut h2 = hdr();
        h2[252..256].copy_from_slice(b"0   ");
        assert!(parse(&h2).is_none());
    }
}

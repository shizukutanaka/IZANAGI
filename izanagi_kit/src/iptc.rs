//! IPTC-IIM — `0x1C <record> <dataset> <u16 len>` marker blocks;
//! counts application/caption datasets (1:90 model version, 2:05 object
//! name, 2:120 caption, 2:25 keywords, 2:55 date).
//!
//! ```
//! let d = b"\x1c\x01\x5a\x00\x02\x00\x00\x1c\x02\x78\x00\x04cap!\x1c\x02\x05\x00\x03nme";
//! let i = izanagi_kit::iptc::parse(d).unwrap();
//! assert_eq!(i.datasets, 3);
//! assert_eq!(i.records, 2);
//! assert!(izanagi_kit::iptc::detect(d));
//! ```

/// Census of an IPTC-IIM stream.
#[derive(Debug, Clone)]
pub struct Iptc {
    /// `0x1C` markers consumed.
    pub markers: usize,
    /// Valid datasets decoded.
    pub datasets: usize,
    /// Record numbers seen (1=env, 2=app, 3=object, 7=pre-edit, 8=obj, 9=post-edit).
    pub records: usize,
    /// Record 2 (application) datasets.
    pub app: usize,
    /// `2:05` object name.
    pub object_names: usize,
    /// `2:25` keywords.
    pub keywords: usize,
    /// `2:55` date-created.
    pub dates: usize,
    /// `2:80` byline.
    pub bylines: usize,
    /// `2:120` caption/abstract.
    pub captions: usize,
    /// Extended (>0x7fff) length fields.
    pub extended: usize,
    /// Total bytes skipped inside datasets.
    pub payload_bytes: usize,
}

/// Detects IPTC-IIM: a `0x1C rr ds` marker with a sane length.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let mut i = 0;
    while i + 5 <= b.len() {
        if b[i] == 0x1c {
            let len = ((b[i + 3] as usize) << 8) | b[i + 4] as usize;
            if len <= b.len() - i - 5 || (b[i + 3] & 0x80) != 0 {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Parses an IPTC-IIM stream; `None` when no `0x1C` block is found.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Iptc> {
    if !detect(b) {
        return None;
    }
    let mut i = 0usize;
    let mut x = Iptc {
        markers: 0,
        datasets: 0,
        records: 0,
        app: 0,
        object_names: 0,
        keywords: 0,
        dates: 0,
        bylines: 0,
        captions: 0,
        extended: 0,
        payload_bytes: 0,
    };
    let mut seen_rec = Vec::<u8>::new();
    while i + 5 <= b.len() {
        if b[i] != 0x1c {
            i += 1;
            continue;
        }
        let rec = b[i + 1];
        let ds = b[i + 2];
        let len = ((b[i + 3] as usize) << 8) | b[i + 4] as usize;
        x.markers += 1;
        if (b[i + 3] & 0x80) != 0 {
            x.extended += 1;
            i += 5 + (len & 0x7fff);
            continue;
        }
        if i + 5 + len > b.len() {
            break;
        }
        x.datasets += 1;
        x.payload_bytes += len;
        if !seen_rec.contains(&rec) {
            seen_rec.push(rec);
            x.records = seen_rec.len();
        }
        if rec == 2 {
            x.app += 1;
            match ds {
                0x05 => x.object_names += 1,
                0x19 => x.keywords += 1,
                0x37 => x.dates += 1,
                0x50 => x.bylines += 1,
                0x78 => x.captions += 1,
                _ => {}
            }
        }
        i += 5 + len;
    }
    (x.datasets > 0).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"\x1c\x01\x5a\x00\x02\x00\x00\x1c\x02\x78\x00\x04cap!\x1c\x02\x05\x00\x03nme\x1c\x02\x19\x00\x02kw";

    #[test]
    fn parses() {
        let i = parse(D).unwrap();
        assert_eq!(i.markers, 4);
        assert_eq!(i.datasets, 4);
        assert_eq!(i.records, 2);
        assert_eq!(i.app, 3);
        assert_eq!(i.object_names, 1);
        assert_eq!(i.keywords, 1);
        assert_eq!(i.captions, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"\x1c"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain").is_none());
    }
}

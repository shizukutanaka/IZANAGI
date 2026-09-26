//! JPEG (ISO/IEC 10918-1 / JFIF) marker-segment walker.
//!
//! Reads the marker stream — SOI, APPn, DQT, DHT, SOF0–15, DRI, SOS,
//! RSTn, EOI — and reports each segment's position and payload length.
//! Dimensions come from the first SOF segment (any of the 16 coding
//! process variants). Entropy-coded data after SOS is skipped by
//! scanning for the next non-stuffed `FF xx` marker.
//!
//! ```
//! let mut j = vec![0xFF, 0xD8]; // SOI
//! j.extend_from_slice(&[0xFF, 0xE0, 0, 16]); // APP0 len=16
//! j.extend_from_slice(b"JFIF\0\x01\x02\x00\x00\x01\x00\x01\x00\x00");
//! j.extend_from_slice(&[0xFF, 0xC0, 0, 11]); // SOF0 len=11
//! j.extend_from_slice(&[8, 0, 16, 0, 24, 1, 1, 0x22, 0]); // 24x16, 1 comp
//! j.extend_from_slice(&[0xFF, 0xD9]); // EOI
//! let p = izanagi_kit::jpeg::parse(&j).unwrap();
//! assert_eq!((p.width, p.height), (24, 16));
//! assert_eq!(p.markers.len(), 4);
//! ```

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(((*d.get(at)? as u16) << 8) | *d.get(at + 1)? as u16)
}

/// One marker segment.
#[derive(Debug, Clone)]
pub struct Marker {
    /// Marker code byte (`0xD8` SOI, `0xE0..0xEF` APPn, `0xC0..0xCF`
    /// SOF variants minus C4/C8/CC, `0xDA` SOS, `0xD9` EOI, …).
    pub code: u8,
    /// Absolute offset of the `0xFF` byte.
    pub at: usize,
    /// Absolute offset of the segment payload (right after the length
    /// field; == `at + 2` for standalone markers).
    pub data_at: usize,
    /// Payload length in bytes (0 for standalone markers).
    pub len: usize,
}

/// Parsed JPEG stream info.
#[derive(Debug)]
pub struct Jpeg {
    /// Segments in file order (SOI included, entropy data excluded).
    pub markers: Vec<Marker>,
    /// Image width from the first SOF, or 0 when no SOF was found.
    pub width: usize,
    /// Image height.
    pub height: usize,
    /// Bits per sample (SOF precision field).
    pub precision: u8,
    /// Number of color components.
    pub components: u8,
    /// Where the first SOS entropy data begins (None if no SOS).
    pub entropy_at: Option<usize>,
}

/// Marker has no length field: SOI, EOI, RST0–7, TEM.
pub fn standalone(code: u8) -> bool {
    matches!(code, 0x01 | 0xD8 | 0xD9) || (0xD0..=0xD7).contains(&code)
}

/// Marker is a SOF variant (0xC0..0xCF except DHT C4, JPG C8, DAC CC).
pub fn is_sof(code: u8) -> bool {
    (0xC0..=0xCF).contains(&code) && !matches!(code, 0xC4 | 0xC8 | 0xCC)
}

/// Walks the marker stream. `None` on missing SOI, an `FF xx` where
/// `xx` is 0x00 (that's stuffed entropy data, only legal after SOS),
/// or a truncated length field.
pub fn parse(d: &[u8]) -> Option<Jpeg> {
    if d.len() < 4 || d[0] != 0xFF || d[1] != 0xD8 {
        return None;
    }
    let mut markers = vec![Marker {
        code: 0xD8,
        at: 0,
        data_at: 2,
        len: 0,
    }];
    let mut at = 2;
    let mut w = 0usize;
    let mut h = 0usize;
    let mut prec = 0u8;
    let mut comps = 0u8;
    let mut entropy = None;
    let mut in_scan = false;

    while at < d.len() {
        if in_scan {
            // entropy-coded segment: hunt for the next real marker
            // (FF00 stuffed bytes and FF fill runs are skipped).
            // RSTn restarts are recorded inline and scanning continues;
            // anything else hands control back to the main loop.
            let mut i = at;
            loop {
                if i + 1 >= d.len() {
                    at = d.len();
                    break;
                }
                if d[i] == 0xFF {
                    let n = d[i + 1];
                    if n == 0x00 {
                        i += 2; // stuffed data byte
                        continue;
                    }
                    if n == 0xFF {
                        i += 1; // fill byte
                        continue;
                    }
                    if (0xD0..=0xD7).contains(&n) {
                        markers.push(Marker {
                            code: n,
                            at: i,
                            data_at: i + 2,
                            len: 0,
                        });
                        i += 2; // entropy resumes after an RSTn
                        continue;
                    }
                    at = i;
                    in_scan = false;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if d[at] != 0xFF {
            // Garbage between segments — JFIF requires FFs; degrade.
            at += 1;
            continue;
        }
        // Skip FF fill bytes.
        while at + 1 < d.len() && d[at + 1] == 0xFF {
            at += 1;
        }
        if at + 1 >= d.len() {
            break;
        }
        let code = d[at + 1];
        if code == 0x00 {
            return None; // stuffed byte outside a scan
        }
        if standalone(code) {
            markers.push(Marker {
                code,
                at,
                data_at: at + 2,
                len: 0,
            });
            at += 2;
            if code == 0xD9 {
                break; // EOI
            }
            continue;
        }
        let seg_len = be16(d, at + 2)? as usize;
        if seg_len < 2 {
            return None;
        }
        let data_at = at + 4;
        let payload = seg_len - 2;
        if data_at + payload > d.len() {
            return None;
        }
        markers.push(Marker {
            code,
            at,
            data_at,
            len: payload,
        });
        if is_sof(code) && payload >= 6 {
            prec = d[data_at];
            h = be16(d, data_at + 1)? as usize;
            w = be16(d, data_at + 3)? as usize;
            comps = d[data_at + 5];
        }
        at = data_at + payload;
        if code == 0xDA {
            entropy = Some(at);
            in_scan = true;
        }
    }
    Some(Jpeg {
        markers,
        width: w,
        height: h,
        precision: prec,
        components: comps,
        entropy_at: entropy,
    })
}

/// Convenience: APPn marker bodies (code 0xE0+n) whose payload starts
/// with `prefix` — e.g. `app(&j, b"JFIF\0")` or `b"Exif\0\0"`.
pub fn app<'a>(d: &'a [u8], j: &Jpeg, prefix: &[u8]) -> Vec<&'a [u8]> {
    let mut out = Vec::new();
    for m in &j.markers {
        if (0xE0..=0xEF).contains(&m.code)
            && m.data_at + m.len <= d.len()
            && d[m.data_at..m.data_at + m.len].starts_with(prefix)
        {
            out.push(&d[m.data_at..m.data_at + m.len]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut j = vec![0xFF, 0xD8];
        j.extend_from_slice(&[0xFF, 0xE0, 0, 16]);
        j.extend_from_slice(b"JFIF\0\x01\x02\x00\x00\x01\x00\x01\x00\x00");
        j.extend_from_slice(&[0xFF, 0xE1, 0, 8]); // APP1 Exif
        j.extend_from_slice(b"Exif\0\0");
        j.extend_from_slice(&[0xFF, 0xDB, 0, 5]); // DQT len=5
        j.extend_from_slice(&[0, 1, 2]);
        j.extend_from_slice(&[0xFF, 0xC0, 0, 11]);
        j.extend_from_slice(&[8, 0, 16, 0, 24, 1, 1, 0x22, 0]);
        j.extend_from_slice(&[0xFF, 0xDA, 0, 8]); // SOS
        j.extend_from_slice(&[1, 1, 0, 0, 63, 0]);
        // entropy data incl. stuffed FF00 and an RST0, then EOI
        j.extend_from_slice(&[0x11, 0xFF, 0x00, 0x22, 0xFF, 0xD0, 0x33]);
        j.extend_from_slice(&[0xFF, 0xD9]);
        j
    }

    #[test]
    fn walks_markers() {
        let j = fixture();
        let p = parse(&j).unwrap();
        let codes: Vec<u8> = p.markers.iter().map(|m| m.code).collect();
        assert_eq!(codes, vec![0xD8, 0xE0, 0xE1, 0xDB, 0xC0, 0xDA, 0xD0, 0xD9]);
        assert_eq!(
            (p.width, p.height, p.precision, p.components),
            (24, 16, 8, 1)
        );
        assert!(p.entropy_at.is_some());
    }

    #[test]
    fn app_prefix_selects() {
        let j = fixture();
        let p = parse(&j).unwrap();
        assert_eq!(app(&j, &p, b"JFIF\0").len(), 1);
        assert_eq!(app(&j, &p, b"Exif").len(), 1);
        assert_eq!(app(&j, &p, b"XX").len(), 0);
    }

    #[test]
    fn sof_variants() {
        assert!(is_sof(0xC0));
        assert!(is_sof(0xC2));
        assert!(!is_sof(0xC4));
        assert!(!is_sof(0xC8));
        assert!(!is_sof(0xCC));
        assert!(!is_sof(0xDA));
        assert!(standalone(0xD9));
        assert!(standalone(0xD3));
        assert!(!standalone(0xC0));
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"JFIF").is_none());
        let mut bad = fixture();
        bad[4] = 0xFF;
        bad[5] = 0x00; // stuffed byte outside scan
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture();
        bad2.truncate(6); // mid-APP0
        assert!(parse(&bad2).is_none());
    }
}

//! LightWave Object `.lwo` / `.lwo2` — IFF `FORM` chunk container.
//!
//! `FORM` + `u32be` size + form type (`LWO2` for LightWave 6+,
//! `LWOB` for 5.x objects, `LWLO` for layered 5.x), then
//! `id4 + u32be length` chunks, each padded to an even size.
//! Known chunks: `LAYR`, `PNTS`, `POLS`, `SURF`, `TAGS`, `BBOX`,
//! `PTAG`, `VMAP`, `VMAD`, `CLIP`, `ENVL`.
//!
//! ```
//! let mut d = b"FORM".to_vec();
//! d.extend_from_slice(&[0, 0, 0, 20]);
//! d.extend_from_slice(b"LWO2");
//! d.extend_from_slice(b"TAGS");
//! d.extend_from_slice(&[0, 0, 0, 8]);
//! d.extend_from_slice(b"WOOD\0GRAY\0");
//! let f = izanagi_kit::lwo::parse(&d).unwrap();
//! assert_eq!(f.form, "LWO2");
//! assert_eq!(f.chunks, vec!["TAGS"]);
//! assert_eq!(f.tag_lists, 1);
//! ```
//!
//! Reference: LightWave LWO2 file format specification (NewTek SDK);
//! Blender `io_import_lwo` notes. Integer-only.

/// Parsed `FORM` header + chunk census.
#[derive(Debug, Clone, PartialEq)]
pub struct Lwo {
    /// Form type: `LWO2`, `LWOB` or `LWLO`.
    pub form: String,
    /// Declared `FORM` payload size (excludes the 8-byte header).
    pub declared: u32,
    /// Chunk four-codes in file order.
    pub chunks: Vec<String>,
    /// `LAYR` layer chunks.
    pub layers: u32,
    /// `PNTS` point-list chunks.
    pub point_lists: u32,
    /// `POLS` polygon-list chunks.
    pub polygon_lists: u32,
    /// `SURF` surface chunks.
    pub surfaces: u32,
    /// `TAGS` tag-string chunks.
    pub tag_lists: u32,
    /// `BBOX` bounding-box chunks.
    pub bounding_boxes: u32,
    /// A trailing partial header or over-long final chunk was seen.
    pub truncated_tail: bool,
}

fn u32be(d: &[u8], i: usize) -> u32 {
    (d[i] as u32) << 24 | (d[i + 1] as u32) << 16 | (d[i + 2] as u32) << 8 | d[i + 3] as u32
}

/// Parse the `FORM` header and walk even-padded chunks.
/// `None` when the `FORM` header or the LightWave form type is absent.
pub fn parse(d: &[u8]) -> Option<Lwo> {
    if d.len() < 12 || &d[..4] != b"FORM" {
        return None;
    }
    let form = &d[8..12];
    if !(form == b"LWO2" || form == b"LWOB" || form == b"LWLO") {
        return None;
    }
    let mut f = Lwo {
        form: String::from_utf8_lossy(form).into_owned(),
        declared: u32be(d, 4),
        chunks: Vec::new(),
        layers: 0,
        point_lists: 0,
        polygon_lists: 0,
        surfaces: 0,
        tag_lists: 0,
        bounding_boxes: 0,
        truncated_tail: false,
    };
    let mut i = 12;
    while i + 8 <= d.len() {
        let id = &d[i..i + 4];
        if !id.iter().all(|b| (0x20..0x7f).contains(b)) {
            f.truncated_tail = true;
            break;
        }
        let len = u32be(d, i + 4) as usize;
        let name = String::from_utf8_lossy(id).into_owned();
        match id {
            b"LAYR" => f.layers += 1,
            b"PNTS" => f.point_lists += 1,
            b"POLS" => f.polygon_lists += 1,
            b"SURF" => f.surfaces += 1,
            b"TAGS" => f.tag_lists += 1,
            b"BBOX" => f.bounding_boxes += 1,
            _ => {}
        }
        f.chunks.push(name);
        let adv = 8 + len + (len & 1);
        if i + adv > d.len() {
            f.truncated_tail = true;
            break;
        }
        i += adv;
    }
    if i < d.len() && i + 8 > d.len() {
        f.truncated_tail = true;
    }
    if f.chunks.is_empty() {
        return None;
    }
    Some(f)
}

/// `true` when a `FORM` header names a LightWave form type.
pub fn detect(d: &[u8]) -> bool {
    d.len() >= 12
        && &d[..4] == b"FORM"
        && (&d[8..12] == b"LWO2" || &d[8..12] == b"LWOB" || &d[8..12] == b"LWLO")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Vec<u8> {
        let mut d = b"FORM".to_vec();
        d.extend_from_slice(&[0, 0, 0, 52]);
        d.extend_from_slice(b"LWO2");
        for (id, payload) in [
            (&b"LAYR"[..], &b"\x00\x00flags"[..]),
            (&b"PNTS"[..], &b"\x00\x01\x02\x03\x04"[..]),
            (&b"SURF"[..], &b"surf\0"[..]),
        ] {
            d.extend_from_slice(id);
            d.extend_from_slice(&[0, 0, 0, payload.len() as u8]);
            d.extend_from_slice(payload);
            if payload.len() & 1 == 1 {
                d.push(0);
            }
        }
        d
    }

    #[test]
    fn parses() {
        let f = parse(&doc()).unwrap();
        assert_eq!(f.form, "LWO2");
        assert_eq!(f.chunks, vec!["LAYR", "PNTS", "SURF"]);
        assert_eq!(f.layers, 1);
        assert_eq!(f.point_lists, 1);
        assert_eq!(f.surfaces, 1);
        assert!(!f.truncated_tail);
    }

    #[test]
    fn odd_payload_padded_and_truncation() {
        // payload len 5 → one pad byte consumed
        let mut d = b"FORM".to_vec();
        d.extend_from_slice(&[0, 0, 0, 18]);
        d.extend_from_slice(b"LWOB");
        d.extend_from_slice(b"TAGS");
        d.extend_from_slice(&[0, 0, 0, 5]);
        d.extend_from_slice(b"abcde");
        d.push(0); // pad
        let f = parse(&d).unwrap();
        assert_eq!(f.tag_lists, 1);
        assert!(!f.truncated_tail);
        // over-long final chunk → flagged, chunk still recorded
        let mut t = doc();
        t.extend_from_slice(b"POLS");
        t.extend_from_slice(&[0x7f, 0, 0, 0]);
        let f = parse(&t).unwrap();
        assert!(f.truncated_tail);
        assert_eq!(f.polygon_lists, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"FORM\x00\x00\x00\x04AIFF").is_none()); // wrong form type
        assert!(parse(b"RIFF\x00\x00\x00\x04LWO2").is_none());
        assert!(parse(b"FORM\x00\x00\x00\x04LWO2").is_none()); // zero chunks
    }

    #[test]
    fn detect_works() {
        assert!(detect(&doc()));
        assert!(!detect(b"FORM\x00\x00\x00\x04ILBM"));
    }
}

//! EBML (RFC 9559) — the variable-length container under Matroska
//! and WebM. Element IDs and data sizes are both VINTs whose first
//! byte's leading zeros encode the width; an all-ones size means
//! "unknown" (streams until the parent ends).
//!
//! ```
//! // EBML header element: id 0x1A45DFA3 containing DocType "webm".
//! let mut d = Vec::new();
//! d.extend_from_slice(&[0x1A, 0x45, 0xDF, 0xA3]); // EBML id
//! d.push(0x87); // size: 1-byte VINT, 7-byte payload
//! d.extend_from_slice(&[0x42, 0x82, 0x84]); // DocType id + size
//! d.extend_from_slice(b"webm");
//! let f = izanagi_kit::ebml::parse(&d).unwrap();
//! assert_eq!(f[0].id, 0x1A45DFA3);
//! let kids = izanagi_kit::ebml::children(&d, &f[0]).unwrap();
//! assert_eq!(izanagi_kit::ebml::utf8(&d, &kids[0]), Some("webm".to_string()));
//! ```

/// An EBML element reference.
#[derive(Debug, Clone)]
pub struct Elem {
    /// Element ID — the VINT *with* its length-marker bits kept.
    pub id: u64,
    /// Offset of the element's data (after id + size VINTs).
    pub at: usize,
    /// Payload size in bytes (`None` = unknown-size).
    pub size: Option<usize>,
    /// Whether the payload is itself an element list.
    pub master: bool,
}

/// Reads a VINT at `at`. Returns `(raw_value_with_marker_bit,
/// marker_stripped_value, byte_width)`. `None` when truncated or the
/// first byte is `0` (a VINT can never start with 0x00).
pub fn vint(d: &[u8], at: usize) -> Option<(u64, u64, usize)> {
    let b0 = *d.get(at)?;
    if b0 == 0 {
        return None;
    }
    let w = b0.leading_zeros() as usize + 1;
    if w > 8 || at + w > d.len() {
        return None;
    }
    let mut raw = b0 as u64;
    // width-8 VINTs consume the whole first byte as the marker
    let mut val = if w == 8 {
        0u64
    } else {
        (b0 & (0xFF >> w)) as u64
    };
    for i in 1..w {
        raw = (raw << 8) | d[at + i] as u64;
        val = (val << 8) | d[at + i] as u64;
    }
    Some((raw, val, w))
}

/// Whether `val` is the all-ones "unknown size" VINT value for its
/// width (`w` bytes → `2^(7w) - 1`).
fn unknown(val: u64, w: usize) -> bool {
    val == (1u64 << (7 * w)) - 1
}

/// Element IDs that are master elements in Matroska/WebM.
pub fn is_master(id: u64) -> bool {
    matches!(
        id,
        0x1A45DFA3 // EBML header
            | 0x18538067 // Segment
            | 0x1549A966 // Info
            | 0x1654AE6B // Tracks
            | 0x1F43B675 // Cluster
            | 0x1254C367 // Tags
            | 0x1C53BB6B // Cues
            | 0x1941A469 // Attachments
            | 0x1043A770 // Chapters
            | 0x7373 // Tag
            | 0x63C0 // Tag + sub? (TagTargets)
            | 0xAE // TrackEntry
            | 0x114D9B74 // SeekHead
            | 0x45B9 // EditionEntry
            | 0x8F // ChapterAtom? — actually 0xB6 is ChapterAtom
            | 0xB6 // ChapterAtom
            | 0x20 // BlockGroup? no — 0xA0
            | 0xA0 // BlockGroup
            | 0xA1 // Block
            | 0xEC // Void is not master; fine — kept for robustness
    )
}

/// Reads one element at `at`; `None` on truncation or bad VINT.
fn read_elem(d: &[u8], at: usize, end: usize) -> Option<(Elem, usize)> {
    if at >= end || at >= d.len() {
        return None;
    }
    let (raw_id, _, iw) = vint(d, at)?;
    let mut p = at + iw;
    let (_, val, sw) = vint(d, p)?;
    p += sw;
    if p > end {
        return None;
    }
    let size = if unknown(val, sw) {
        None
    } else {
        Some(val as usize)
    };
    let e = Elem {
        id: raw_id,
        at: p,
        size,
        master: is_master(raw_id),
    };
    let total = p.checked_add(size.unwrap_or(end - p))?.min(end);
    if total < p || p + size.unwrap_or(0) > end && size.is_some() {
        return None;
    }
    Some((e, total))
}

/// Top-level element list (up to `end` or the file end).
pub fn parse(d: &[u8]) -> Option<Vec<Elem>> {
    if d.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at < d.len() {
        let (e, nx) = read_elem(d, at, d.len())?;
        if nx <= at {
            return None;
        }
        out.push(e);
        at = nx;
    }
    Some(out)
}

/// Children of a master element.
pub fn children(d: &[u8], e: &Elem) -> Option<Vec<Elem>> {
    if !e.master {
        return None;
    }
    let end = e.at + e.size.unwrap_or(d.len() - e.at);
    let mut out = Vec::new();
    let mut at = e.at;
    while at < end {
        let (c, nx) = read_elem(d, at, end)?;
        if nx <= at {
            return None;
        }
        out.push(c);
        at = nx;
    }
    Some(out)
}

/// Payload bytes of an element.
pub fn data<'a>(d: &'a [u8], e: &Elem) -> Option<&'a [u8]> {
    d.get(e.at..e.at + e.size.unwrap_or(d.len() - e.at))
}

/// Unsigned-integer value (1–8 byte big-endian payload).
pub fn uint(d: &[u8], e: &Elem) -> Option<u64> {
    let p = data(d, e)?;
    if p.is_empty() || p.len() > 8 {
        return None;
    }
    let mut v = 0u64;
    for &b in p {
        v = (v << 8) | b as u64;
    }
    Some(v)
}

/// Signed-integer value.
pub fn int(d: &[u8], e: &Elem) -> Option<i64> {
    let p = data(d, e)?;
    if p.is_empty() || p.len() > 8 {
        return None;
    }
    let neg = p[0] & 0x80 != 0;
    let mut v: i128 = if neg { -1i128 } else { 0 };
    for &b in p {
        v = (v << 8) | b as i128;
    }
    i64::try_from(v).ok()
}

/// ASCII payload.
pub fn text(d: &[u8], e: &Elem) -> Option<String> {
    let p = data(d, e)?;
    if p.iter().any(|&b| !b.is_ascii()) {
        return None;
    }
    Some(String::from_utf8_lossy(p).into_owned())
}

/// UTF-8 payload.
pub fn utf8(d: &[u8], e: &Elem) -> Option<String> {
    let p = data(d, e)?;
    core::str::from_utf8(p).ok().map(|s| s.to_string())
}

/// Float payload as raw IEEE bits (`4`→f32 bits, `8`→f64 bits) —
/// float types are banned, so callers receive `u64`.
pub fn float_bits(d: &[u8], e: &Elem) -> Option<u64> {
    let p = data(d, e)?;
    match p.len() {
        4 => uint(d, e),
        8 => uint(d, e),
        _ => None,
    }
}

/// Date payload — nanoseconds since 2001-01-01 (i64).
pub fn date(d: &[u8], e: &Elem) -> Option<i64> {
    int(d, e)
}

/// First element with the id in a list.
pub fn find(es: &[Elem], id: u64) -> Option<&Elem> {
    es.iter().find(|e| e.id == id)
}

/// All elements with the id.
pub fn find_all(es: &[Elem], id: u64) -> Vec<&Elem> {
    es.iter().filter(|e| e.id == id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DocType "webm" inside EBML + a Segment with unknown size.
    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        // EBML { DocType }
        d.extend_from_slice(&[0x1A, 0x45, 0xDF, 0xA3]);
        d.push(0x87); // 7-byte payload
        d.extend_from_slice(&[0x42, 0x82, 0x84]);
        d.extend_from_slice(b"webm");
        // Segment (unknown size) containing Info{Timecode uint 0}
        d.extend_from_slice(&[0x18, 0x53, 0x80, 0x67]);
        d.push(0xFF); // unknown size
        d.extend_from_slice(&[0x15, 0x49, 0xA9, 0x66]); // Info id
        d.push(0x83);
        d.extend_from_slice(&[0xE7, 0x81, 0x00]); // Timecode id+size+0
        d
    }

    #[test]
    fn parses_elements() {
        let d = fixture();
        let top = parse(&d).unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].id, 0x1A45DFA3);
        assert_eq!(top[1].id, 0x18538067);
        assert!(top[1].size.is_none()); // unknown
        let kids = children(&d, &top[0]).unwrap();
        assert_eq!(utf8(&d, &kids[0]), Some("webm".to_string()));
        let segs = children(&d, &top[1]).unwrap();
        assert_eq!(find_all(&segs, 0x1549A966).len(), 1);
        let info = find(&segs, 0x1549A966).unwrap();
        let info_kids = children(&d, info).unwrap();
        let tc = find(&info_kids, 0xE7).unwrap();
        assert_eq!(uint(&d, tc), Some(0));
        assert!(is_master(0x1A45DFA3));
        assert!(!is_master(0x42));
    }

    #[test]
    fn vint_widths_and_unknown() {
        assert_eq!(vint(&[0x80], 0), Some((0x80, 0, 1)));
        assert_eq!(vint(&[0x40, 0x01], 0), Some((0x4001, 1, 2)));
        assert_eq!(vint(&[0xFF], 0), Some((0xFF, 0x7F, 1))); // unknown size
        assert!(vint(&[0x00], 0).is_none());
        assert!(vint(&[], 0).is_none());
        assert_eq!(
            vint(&[0x01, 0, 0, 0, 0, 0, 0, 0], 0),
            Some((0x0100_0000_0000_0000, 0, 8))
        );
    }

    #[test]
    fn typed_payloads() {
        // 1-byte int -1
        let mut d = Vec::new();
        d.extend_from_slice(&[0x8E, 0x81]); // id + size 1
        d.push(0xFF);
        let e = parse(&d).unwrap();
        assert_eq!(int(&d, &e[0]), Some(-1));
        assert_eq!(date(&d, &e[0]), Some(-1));
        assert!(text(&d, &e[0]).is_none()); // 0xFF is not ASCII
                                            // 8-byte float bits
        let mut d2 = Vec::new();
        d2.extend_from_slice(&[0x44, 0x89, 0x88]); // Duration id + size 8
        d2.extend_from_slice(&[0x40, 0x59, 0, 0, 0, 0, 0, 0]); // 100.0 f64
        let e2 = parse(&d2).unwrap();
        assert_eq!(float_bits(&d2, &e2[0]), Some(0x4059_0000_0000_0000));
    }

    #[test]
    fn malformed_degrades() {
        assert_eq!(parse(&[]).unwrap().len(), 0);
        assert!(parse(&[0x00]).is_none()); // invalid VINT
        assert!(parse(&[0x1A]).is_none()); // truncated id
        let mut bad = fixture();
        bad[4] = 0x7F; // declared payload way beyond the file
        assert!(parse(&bad).is_none());
    }
}

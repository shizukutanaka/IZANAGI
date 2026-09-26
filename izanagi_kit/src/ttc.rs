//! TrueType Collection (.ttc) wrapper.
//!
//! A `.ttc` is `ttcf` + version + font count + a table of u32BE
//! offsets, each pointing at a normal sfnt/TrueType file inside the
//! same blob. Parsing each member is delegated to
//! [`crate::ttf::parse`]. Version 2 collections append a digital
//! signature table (offset + length + tag) after the offset table.
//!
//! ```
//! // Minimal 2-font collection: header(16B) + 2 offsets, fonts at 24,64.
//! let mut d = vec![0u8; 128];
//! d[0..4].copy_from_slice(b"ttcf");
//! d[4..8].copy_from_slice(&[0, 1, 0, 0]); // version 1.0
//! d[8..12].copy_from_slice(&[0, 0, 0, 2]); // numFonts
//! d[12..16].copy_from_slice(&[0, 0, 0, 24]);
//! d[16..20].copy_from_slice(&[0, 0, 0, 64]);
//! // sfnt at 24: flavor 0x00010000, 0 tables
//! d[24..28].copy_from_slice(&[0, 1, 0, 0]);
//! d[28..30].copy_from_slice(&[0, 0]); // numTables=0
//! // sfnt at 64
//! d[64..68].copy_from_slice(&[0, 1, 0, 0]);
//! d[68..70].copy_from_slice(&[0, 0]);
//! let t = izanagi_kit::ttc::parse(&d).unwrap();
//! assert_eq!(t.offsets, vec![24, 64]);
//! assert_eq!(izanagi_kit::ttc::fonts(&d).unwrap().len(), 2);
//! ```

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        ((*d.get(at)? as u32) << 24)
            | ((*d.get(at + 1)? as u32) << 16)
            | ((*d.get(at + 2)? as u32) << 8)
            | *d.get(at + 3)? as u32,
    )
}

/// A parsed TTC header.
#[derive(Debug)]
pub struct Ttc {
    /// Version: `0x0001_0000` or `0x0002_0000`.
    pub version: u32,
    /// Offsets of each member font inside the blob.
    pub offsets: Vec<u32>,
    /// Version-2 digital signature: `(tag, offset, length)` if present.
    pub signature: Option<(u32, u32, u32)>,
}

/// Parses the `ttcf` header and offset table.
/// `None` on bad magic, version 0, or out-of-range offsets.
pub fn parse(d: &[u8]) -> Option<Ttc> {
    if d.len() < 12 || &d[0..4] != b"ttcf" {
        return None;
    }
    let version = be32(d, 4)?;
    let count = be32(d, 8)? as usize;
    if count == 0 || count > 1024 {
        return None;
    }
    let mut offsets = Vec::with_capacity(count);
    for i in 0..count {
        let o = be32(d, 12 + i * 4)? as usize;
        if o >= d.len() {
            return None;
        }
        offsets.push(o as u32);
    }
    let mut signature = None;
    if version == 0x0002_0000 {
        let tag = be32(d, 12 + count * 4)?;
        let len = be32(d, 16 + count * 4)?;
        let off = be32(d, 20 + count * 4)?;
        signature = Some((tag, off, len));
    }
    Some(Ttc {
        version,
        offsets,
        signature,
    })
}

/// Parses every member font. `None` on the first member that fails
/// [`crate::ttf::parse`] (each offset must point at a valid sfnt).
pub fn fonts(d: &[u8]) -> Option<Vec<crate::ttf::Ttf>> {
    let t = parse(d)?;
    let mut out = Vec::with_capacity(t.offsets.len());
    for &o in &t.offsets {
        out.push(crate::ttf::parse(d.get(o as usize..)?)?);
    }
    Some(out)
}

/// Parses only member `i`.
pub fn font(d: &[u8], i: usize) -> Option<crate::ttf::Ttf> {
    let t = parse(d)?;
    let o = *t.offsets.get(i)? as usize;
    crate::ttf::parse(d.get(o..)?)
}

/// `true` when `d` is a TTC (vs a plain sfnt).
pub fn is_collection(d: &[u8]) -> bool {
    d.len() >= 4 && &d[0..4] == b"ttcf"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[0..4].copy_from_slice(b"ttcf");
        d[4..8].copy_from_slice(&[0, 1, 0, 0]);
        d[8..12].copy_from_slice(&[0, 0, 0, 2]);
        d[12..16].copy_from_slice(&[0, 0, 0, 24]);
        d[16..20].copy_from_slice(&[0, 0, 0, 64]);
        d[24..28].copy_from_slice(&[0, 1, 0, 0]);
        d[28..30].copy_from_slice(&[0, 0]);
        d[64..68].copy_from_slice(&[0, 1, 0, 0]);
        d[68..70].copy_from_slice(&[0, 0]);
        d
    }

    #[test]
    fn parses_header() {
        let d = fixture();
        let t = parse(&d).unwrap();
        assert_eq!(t.version, 0x0001_0000);
        assert_eq!(t.offsets, vec![24, 64]);
        assert!(t.signature.is_none());
        assert!(is_collection(&d));
        assert!(!is_collection(b"OTTO"));
    }

    #[test]
    fn resolves_fonts() {
        let d = fixture();
        let fs = fonts(&d).unwrap();
        assert_eq!(fs.len(), 2);
        assert!(font(&d, 0).is_some());
        assert!(font(&d, 9).is_none());
    }

    #[test]
    fn v2_signature() {
        // v2 appends DSIG (tag, sig-len, sig-off) after the offsets.
        let mut d = vec![0u8; 128];
        d[0..4].copy_from_slice(b"ttcf");
        d[4..8].copy_from_slice(&[0, 2, 0, 0]);
        d[8..12].copy_from_slice(&[0, 0, 0, 1]);
        d[12..16].copy_from_slice(&[0, 0, 0, 36]); // font at 36
        d[16..20].copy_from_slice(b"DSIG");
        d[20..24].copy_from_slice(&[0, 0, 0, 8]); // sig len
        d[24..28].copy_from_slice(&[0, 0, 0, 100]); // sig off
        d[36..40].copy_from_slice(&[0, 1, 0, 0]);
        d[40..42].copy_from_slice(&[0, 0]);
        let t = parse(&d).unwrap();
        assert_eq!(t.signature, Some((0x4453_4947, 100, 8))); // "DSIG"
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"ttcX").is_none());
        let mut bad = fixture();
        bad[11] = 0; // count = 0
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture();
        bad2[15] = 0xFF; // offset > len
        assert!(parse(&bad2).is_none());
    }
}

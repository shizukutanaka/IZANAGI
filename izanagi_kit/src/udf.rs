//! UDF — Universal Disk Format (OSTA/ECMA-167) volume recognition.
//! Over the ISO 9660 descriptors, UDF places a Volume Recognition
//! Sequence at sector 16+: descriptors `{type: 0, id: "BEA01"/"NSR02"/
//! "NSR03"/"TEA01", version: 1}` one per 2048-byte sector, ending at
//! `TEA01`. The Anchor Volume Descriptor Pointer sits at sector 256
//! with tag id 2.
//!
//! ```
//! use izanagi_kit::udf::{parse, descriptors, SECTOR};
//! let mut d = vec![0u8; 258 * SECTOR];
//! for (i, id) in [b"BEA01", b"NSR03", b"TEA01"].iter().enumerate() {
//!     let at = (16 + i) * SECTOR;
//!     d[at] = 0; d[at + 1..at + 6].copy_from_slice(&id[..]); d[at + 6] = 1;
//! }
//! let u = parse(&d).unwrap();
//! assert_eq!(u.nsr.as_slice(), b"NSR03");
//! assert_eq!(descriptors(&d).len(), 3);
//! ```

/// Sector size (UDF keeps the 2048-byte physical sector).
pub const SECTOR: usize = 2048;

/// Sector the Volume Recognition Sequence starts at.
pub const VRS_SECTOR: usize = 16;

/// Sector the Anchor Volume Descriptor Pointer lives at.
pub const AVDP_SECTOR: usize = 256;

/// A parsed Volume Recognition Sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Udf {
    /// The `NSR02`/`NSR03` descriptor id actually used.
    pub nsr: Vec<u8>,
    /// Whether a `BEA01` begin descriptor was seen.
    pub bea: bool,
    /// Absolute offset of the `TEA01` terminator.
    pub terminator_at: usize,
}

/// Descriptor ids recognised in the VRS, in wire order.
pub const IDS: [&[u8; 5]; 4] = [b"BEA01", b"NSR02", b"NSR03", b"TEA01"];

fn desc_at(d: &[u8], sec: usize) -> Option<([u8; 5], usize)> {
    let at = sec.checked_mul(SECTOR)?;
    let tag = *d.get(at)?;
    let id = d.get(at + 1..at + 6)?;
    let ver = *d.get(at + 6)?;
    if tag == 0 && ver == 1 && IDS.iter().any(|w| *w == id) {
        Some((id.try_into().ok()?, at))
    } else {
        None
    }
}

/// Walk the VRS: `(id, offset)` pairs up to and including `TEA01`.
pub fn descriptors(d: &[u8]) -> Vec<([u8; 5], usize)> {
    let mut out = Vec::new();
    let mut sec = VRS_SECTOR;
    while let Some((id, at)) = desc_at(d, sec) {
        out.push((id, at));
        if &id == b"TEA01" {
            break;
        }
        sec += 1;
        if sec > 96 {
            break; // VRS is bounded in practice
        }
    }
    out
}

/// Parse the recognition sequence: `BEA01` (optional per OSTA),
/// `NSR02`/`NSR03`, `TEA01`. `None` when the sequence is absent.
pub fn parse(d: &[u8]) -> Option<Udf> {
    let ds = descriptors(d);
    let mut bea = false;
    let mut nsr = None;
    let mut term = None;
    for (id, at) in &ds {
        match id {
            b"BEA01" => bea = true,
            b"NSR02" | b"NSR03" => nsr = Some(id.to_vec()),
            b"TEA01" => term = Some(*at),
            _ => {}
        }
    }
    Some(Udf {
        nsr: nsr?,
        bea,
        terminator_at: term?,
    })
}

/// Check the Anchor Volume Descriptor Pointer: `{tag_id: u16 LE = 2}`
/// at sector 256.
pub fn has_avdp(d: &[u8]) -> bool {
    let at = AVDP_SECTOR * SECTOR;
    d.get(at..at + 2) == Some(&[2, 0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 260 * SECTOR];
        for (i, id) in [b"BEA01", b"NSR03", b"TEA01"].iter().enumerate() {
            let at = (16 + i) * SECTOR;
            d[at] = 0;
            d[at + 1..at + 6].copy_from_slice(&id[..]);
            d[at + 6] = 1;
        }
        d[AVDP_SECTOR * SECTOR] = 2;
        d
    }

    #[test]
    fn vrs_and_anchor() {
        let d = fixture();
        let u = parse(&d).unwrap();
        assert!(u.bea);
        assert_eq!(u.nsr.as_slice(), b"NSR03");
        assert_eq!(u.terminator_at, 18 * SECTOR);
        assert_eq!(descriptors(&d).len(), 3);
        assert!(has_avdp(&d));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[17 * SECTOR + 1] = b'X'; // breaks NSR03 → no NSR → None
        assert!(parse(&d).is_none());
        // non-descriptor in the middle truncates before TEA01
        let mut d2 = fixture();
        d2[17 * SECTOR] = 1;
        assert_eq!(descriptors(&d2).len(), 1);
        assert!(parse(&d2).is_none());
    }
}

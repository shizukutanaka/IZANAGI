//! ICC color profile (ICC.1 / ISO 15076-1) header + tag table.
//!
//! A profile is a 128-byte big-endian header — total size, CMM
//! signature, packed-BCD version, device class, data color space,
//! PCS, creation date/time, `acsp` signature, platform, flags,
//! manufacturer/model, rendering intent and XYZ illuminant —
//! then a u32 tag count and `{sig, offset, size}` triples.
//!
//! ```
//! use izanagi_kit::icc::{parse, DEVICE_MNTR};
//! let mut d = vec![0u8; 132];
//! d[0..4].copy_from_slice(&[0, 0, 0, 132]); // declared size
//! d[36..40].copy_from_slice(b"acsp");
//! d[8] = 0x04; d[9] = 0x30;      // version 4.3
//! d[12..16].copy_from_slice(&DEVICE_MNTR);
//! d[16..20].copy_from_slice(b"RGB ");
//! d[20..24].copy_from_slice(b"XYZ ");
//! let p = parse(&d).unwrap();
//! assert_eq!(p.version(), (4, 3));
//! ```

/// Header size in bytes; the tag table follows it.
pub const HEADER: usize = 128;
/// Device class `mntr` — display profile.
pub const DEVICE_MNTR: [u8; 4] = *b"mntr";
/// Device class `prnt` — printer.
pub const DEVICE_PRNT: [u8; 4] = *b"prnt";
/// Device class `scnr` — scanner.
pub const DEVICE_SCNR: [u8; 4] = *b"scnr";
/// Device class `link` — device link.
pub const DEVICE_LINK: [u8; 4] = *b"link";
/// Device class `nmcl` — named color.
pub const DEVICE_NMCL: [u8; 4] = *b"nmcl";
/// Device class `spac` — color space.
pub const DEVICE_SPAC: [u8; 4] = *b"spac";
/// Device class `abst` — abstract.
pub const DEVICE_ABST: [u8; 4] = *b"abst";

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (u32::from(*d.get(at)?) << 24)
            | (u32::from(*d.get(at + 1)?) << 16)
            | (u32::from(*d.get(at + 2)?) << 8)
            | u32::from(*d.get(at + 3)?),
    )
}
fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some((u16::from(*d.get(at)?) << 8) | u16::from(*d.get(at + 1)?))
}

/// A parsed ICC header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Icc {
    /// Declared total profile size.
    pub size: u32,
    /// Preferred CMM signature.
    pub cmm: [u8; 4],
    /// Device class (`DEVICE_*`).
    pub device_class: [u8; 4],
    /// Data color space signature (`RGB `, `CMYK`, `Lab `, …).
    pub color_space: [u8; 4],
    /// Profile connection space (`XYZ ` or `Lab `).
    pub pcs: [u8; 4],
    /// Creation time as `[year, month, day, hour, min, sec]`.
    pub created: [u16; 6],
    /// Primary platform signature (`APPL`, `MSFT`, `SGI `, `SUNW`, 0).
    pub platform: [u8; 4],
    /// Flags (embedded / independent data).
    pub flags: u32,
    /// Device manufacturer signature.
    pub manufacturer: [u8; 4],
    /// Device model signature.
    pub model: [u8; 4],
    /// Rendering intent (0 perceptual, 1 colorimetric, 2 saturation,
    /// 3 absolute).
    pub intent: u32,
    /// PCS illuminant XYZ as raw s15Fixed16 u32 words.
    pub illuminant_raw: [u32; 3],
    /// Profile ID (MD5) or zero.
    pub profile_id: [u8; 16],
    /// Raw version bytes `[major_bcd, minor_bugfix_bcd]`.
    pub version_raw: [u8; 2],
    /// Number of tag-table entries.
    pub tag_count: u32,
}

impl Icc {
    /// Version as `(major, minor)` unpacked from BCD nibbles.
    pub fn version(&self) -> (u8, u8) {
        (self.version_raw[0], self.version_raw[1] >> 4)
    }
    /// Tag-table entry `i` as `(signature, offset, size)`, bounds
    /// checked against the buffer. `None` past `tag_count`.
    pub fn tag(&self, d: &[u8], i: usize) -> Option<([u8; 4], usize, usize)> {
        if i as u32 >= self.tag_count {
            return None;
        }
        let at = HEADER + 4 + i * 12;
        let mut sig = [0u8; 4];
        sig.copy_from_slice(d.get(at..at + 4)?);
        let off = be32(d, at + 4)? as usize;
        let size = be32(d, at + 8)? as usize;
        if off.checked_add(size)? > d.len() {
            return None;
        }
        Some((sig, off, size))
    }
}

/// Parse an ICC header. `None` without `acsp` at offset 36 or on
/// truncation.
pub fn parse(d: &[u8]) -> Option<Icc> {
    let h = d.get(..HEADER)?;
    if h.get(36..40)? != b"acsp" {
        return None;
    }
    let mut created = [0u16; 6];
    for (i, c) in created.iter_mut().enumerate() {
        *c = be16(h, 24 + i * 2)?;
    }
    let mut profile_id = [0u8; 16];
    profile_id.copy_from_slice(h.get(84..100)?);
    let cp = |a: usize| -> [u8; 4] {
        let mut v = [0u8; 4];
        v.copy_from_slice(&h[a..a + 4]);
        v
    };
    let size = be32(h, 0)?;
    if size as usize > d.len() || size < HEADER as u32 {
        return None;
    }
    Some(Icc {
        size,
        cmm: cp(4),
        device_class: cp(12),
        color_space: cp(16),
        pcs: cp(20),
        created,
        platform: cp(40),
        flags: be32(h, 44)?,
        manufacturer: cp(48),
        model: cp(52),
        intent: be32(h, 64)?,
        illuminant_raw: [be32(h, 68)?, be32(h, 72)?, be32(h, 76)?],
        profile_id,
        version_raw: [*h.get(8)?, *h.get(9)?],
        tag_count: be32(d, HEADER)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 128 + 4 + 12 * 2 + 32];
        let w = |d: &mut [u8], o: usize, v: u32| {
            for i in 0..4 {
                d[o + i] = (v >> ((3 - i) * 8)) as u8;
            }
        };
        let n = d.len() as u32;
        w(&mut d, 0, n);
        d[4..8].copy_from_slice(b"appl");
        d[8] = 0x04;
        d[9] = 0x30;
        d[12..16].copy_from_slice(&DEVICE_MNTR);
        d[16..20].copy_from_slice(b"RGB ");
        d[20..24].copy_from_slice(b"XYZ ");
        let w16 = |d: &mut [u8], o: usize, v: u16| {
            d[o] = (v >> 8) as u8;
            d[o + 1] = v as u8;
        };
        w16(&mut d, 24, 2026);
        w16(&mut d, 26, 9);
        w16(&mut d, 28, 26);
        d[36..40].copy_from_slice(b"acsp");
        w(&mut d, 64, 1); // media-relative colorimetric
        w(&mut d, 68, 0x0000_F6D6); // D50 X
        w(&mut d, 128, 2); // tag count
        d[132..136].copy_from_slice(b"desc");
        w(&mut d, 136, 160);
        w(&mut d, 140, 16);
        d[144..148].copy_from_slice(b"cprt");
        w(&mut d, 148, 176);
        w(&mut d, 152, 12);
        d
    }

    #[test]
    fn fields_and_tags() {
        let d = fixture();
        let p = parse(&d).unwrap();
        assert_eq!(p.size as usize, d.len());
        assert_eq!(p.cmm, *b"appl");
        assert_eq!(p.version(), (4, 3));
        assert_eq!(p.device_class, DEVICE_MNTR);
        assert_eq!(p.color_space, *b"RGB ");
        assert_eq!(p.pcs, *b"XYZ ");
        assert_eq!(p.created[0], 2026);
        assert_eq!(p.intent, 1);
        assert_eq!(p.illuminant_raw[0], 0xF6D6);
        assert_eq!(p.tag_count, 2);
        assert_eq!(p.tag(&d, 0), Some((*b"desc", 160, 16)));
        assert_eq!(p.tag(&d, 1), Some((*b"cprt", 176, 12)));
        assert_eq!(p.tag(&d, 2), None);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 64]).is_none());
        let mut d = fixture();
        d[36] = b'X';
        assert!(parse(&d).is_none());
        // declared size beyond buffer
        let mut d2 = fixture();
        d2[3] = 0xFF;
        assert!(parse(&d2).is_none());
        // tag range escaping the buffer -> None entry
        let mut d3 = fixture();
        d3[139] = 0xFF; // tag0 offset = 0xFF0000A0-ish
        assert!(parse(&d3).unwrap().tag(&d3, 0).is_none());
    }
}

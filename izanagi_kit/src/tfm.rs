//! TFM (TeX Font Metric, DEK): big-endian `u16` word file —
//! `lf lh bc ec` + sizes `nw nh nd ni nl nk ne np`, header
//! words `check_sum` + `design_size`, then `char_info`,
//! `width`/`height`/`depth`/`italic`, `lig_kern`, `kern`,
//! `exten`, `param` tables. File length is exactly `4 * lf`.
//!
//! ```
//! // lf=10, lh=2, bc=0, ec=0, one width word.
//! let d: Vec<u8> = vec![
//!     0, 10, 0, 2, 0, 0, 0, 0, // lf lh bc ec
//!     0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // nw..np
//!     0xAB, 0xCD, 0xEF, 0x01, // check_sum
//!     0x00, 0x0A, 0x00, 0x00, // design_size fixword
//!     0, 0, 0, 0, // char_info
//!     0, 0, 0, 0, // width
//! ];
//! let t = izanagi_kit::tfm::parse(&d).unwrap();
//! assert_eq!(t.chars, 1);
//! assert_eq!(t.checksum, 0xABCDEF01);
//! assert_eq!(t.widths, 1);
//! assert!(izanagi_kit::tfm::detect(&d));
//! ```

/// Census of a TFM file.
#[derive(Debug, Clone, PartialEq)]
pub struct Tfm {
    /// `lf` — total length in 4-byte words.
    pub lf: u32,
    /// `lh` — header words (≥2: check_sum + design_size).
    pub lh: u32,
    /// `ec − bc + 1` — declared character codes.
    pub chars: u32,
    /// Width table words (`nw`).
    pub widths: u32,
    /// Height table words (`nh`).
    pub heights: u32,
    /// Depth table words (`nd`).
    pub depths: u32,
    /// Italic-correction table words (`ni`).
    pub italics: u32,
    /// `lig_kern` program words (`nl`).
    pub lig_kerns: u32,
    /// `kern` table words (`nk`).
    pub kerns: u32,
    /// `exten` table words (`ne`).
    pub extens: u32,
    /// `param` (fontdimen) words (`np`).
    pub params: u32,
    /// `check_sum` header word.
    pub checksum: u32,
    /// `design_size` fixword (raw integer).
    pub design_size: u32,
}

fn w16(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 8) | b[i + 1] as u32
}

fn w32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}

/// Sizes consistent: `len == 4*lf` and `lf` decomposes into the tables.
fn header_ok(b: &[u8]) -> Option<(u32, u32, u32, u32)> {
    if b.len() < 24 {
        return None;
    }
    let (lf, lh, bc, ec) = (w16(b, 0), w16(b, 2), w16(b, 4), w16(b, 6));
    if ec < bc || ec > 255 || lh < 2 {
        return None;
    }
    let (nw, nh, nd, ni) = (w16(b, 8), w16(b, 10), w16(b, 12), w16(b, 14));
    let (nl, nk, ne, np) = (w16(b, 16), w16(b, 18), w16(b, 20), w16(b, 22));
    let tables = lh + (ec - bc + 1) + nw + nh + nd + ni + nl + nk + ne + np;
    if 6 + tables != lf || 4 * lf as usize != b.len() {
        return None;
    }
    Some((lf, lh, ec - bc + 1, ec))
}

/// `true` on a self-consistent TFM header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    header_ok(b).is_some()
}

/// Census; `None` unless the header equation and length hold.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Tfm> {
    let (lf, lh, chars, _) = header_ok(b)?;
    Some(Tfm {
        lf,
        lh,
        chars,
        widths: w16(b, 8),
        heights: w16(b, 10),
        depths: w16(b, 12),
        italics: w16(b, 14),
        lig_kerns: w16(b, 16),
        kerns: w16(b, 18),
        extens: w16(b, 20),
        params: w16(b, 22),
        checksum: w32(b, 24),
        design_size: w32(b, 28),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // lf = 6 + lh2 + chars3 + nw1 + nd1 + nl1 = 14 words = 56 B.
        let mut d = vec![
            0, 14, 0, 2, 0, 0, 0, 2, // lf lh bc ec
            0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, // nw nh nd ni nl nk ne np
            0xDE, 0xAD, 0xBE, 0xEF, // check_sum
            0x00, 0x0A, 0x00, 0x00, // design_size
        ];
        d.extend_from_slice(&[0; 4 * 3]); // char_info 3
        d.extend_from_slice(&[0, 0x40, 0, 0]); // width
        d.extend_from_slice(&[0; 4]); // depth
        d.extend_from_slice(&[0; 4]); // lig_kern
        assert_eq!(d.len(), 56);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"cmr10"));
        let mut bad = fixture();
        bad.pop();
        assert!(!detect(&bad));
    }

    #[test]
    fn parses() {
        let t = parse(&fixture()).unwrap();
        assert_eq!(t.lf, 14);
        assert_eq!(t.lh, 2);
        assert_eq!(t.chars, 3);
        assert_eq!(t.widths, 1);
        assert_eq!(t.depths, 1);
        assert_eq!(t.lig_kerns, 1);
        assert_eq!(t.checksum, 0xDEAD_BEEF);
        assert_eq!(t.design_size, 0x000A_0000);
    }

    #[test]
    fn rejects() {
        let mut d = fixture();
        d[0] = 0;
        d[1] = 99; // lf no longer consistent
        assert!(parse(&d).is_none());
        assert!(parse(b"").is_none());
    }
}

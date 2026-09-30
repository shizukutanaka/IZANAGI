//! JFM (Japanese Font Metric, pTeX): TFM variant whose first
//! word is `id` (9 = 横組 yoko, 11 = 縦組 tate) followed by
//! `nt` (char_type entries), `lf lh bc ec`, sizes
//! `nw nh nd ni nl nk ng np`, header `check_sum` +
//! `design_size`, `char_type`/`char_info`/`width…`/`glue`/
//! `param` tables. File length is exactly `4 * lf`.
//!
//! ```
//! // id=9, nt=1, lf=11, lh=2, bc=0, ec=0 — 44 B.
//! let d: Vec<u8> = vec![
//!     0, 9, 0, 1, 0, 11, 0, 2, 0, 0, 0, 0, // id nt lf lh bc ec
//!     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // nw..np
//!     0x00, 0x00, 0x00, 0x01, // check_sum
//!     0x00, 0x0A, 0x00, 0x00, // design_size
//!     0, 0, 0, 0, // char_type
//!     0, 0, 0, 0, // char_info
//! ];
//! let j = izanagi_kit::jfm::parse(&d).unwrap();
//! assert!(j.horizontal());
//! assert_eq!(j.chars, 1);
//! assert!(izanagi_kit::jfm::detect(&d));
//! ```

/// Census of a JFM file.
#[derive(Debug, Clone, PartialEq)]
pub struct Jfm {
    /// `id` word (9 = yoko horizontal, 11 = tate vertical).
    pub id: u32,
    /// `nt` — char_type table entries.
    pub nt: u32,
    /// `lf` — total length in 4-byte words.
    pub lf: u32,
    /// `lh` — header words.
    pub lh: u32,
    /// `ec − bc + 1` — declared character codes.
    pub chars: u32,
    /// Width/height/depth/italic/lig-kern/kern table words.
    pub widths: u32,
    /// Height words (`nh`).
    pub heights: u32,
    /// Depth words (`nd`).
    pub depths: u32,
    /// Italic words (`ni`).
    pub italics: u32,
    /// Ligature/kern program words (`nl`).
    pub lig_kerns: u32,
    /// Kern words (`nk`).
    pub kerns: u32,
    /// Glue words (`ng`) — JFM replaces `exten` with `glue`.
    pub glues: u32,
    /// Param words (`np`).
    pub params: u32,
    /// `check_sum` header word.
    pub checksum: u32,
    /// `design_size` fixword (raw).
    pub design_size: u32,
}

impl Jfm {
    /// `id == 9`: yoko (horizontal) metrics.
    #[must_use]
    pub fn horizontal(&self) -> bool {
        self.id == 9
    }
}

fn w16(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 8) | b[i + 1] as u32
}

fn w32(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | b[i + 3] as u32
}

fn header_ok(b: &[u8]) -> Option<(u32, u32, u32, u32, u32)> {
    if b.len() < 28 {
        return None;
    }
    let (id, nt, lf, lh, bc, ec) = (
        w16(b, 0),
        w16(b, 2),
        w16(b, 4),
        w16(b, 6),
        w16(b, 8),
        w16(b, 10),
    );
    if !matches!(id, 9 | 11) || ec < bc || ec > 255 || lh < 2 {
        return None;
    }
    let (nw, nh, nd, ni) = (w16(b, 12), w16(b, 14), w16(b, 16), w16(b, 18));
    let (nl, nk, ng, np) = (w16(b, 20), w16(b, 22), w16(b, 24), w16(b, 26));
    let tables = lh + nt + (ec - bc + 1) + nw + nh + nd + ni + nl + nk + ng + np;
    if 7 + tables != lf || 4 * lf as usize != b.len() {
        return None;
    }
    Some((id, nt, lf, lh, ec - bc + 1))
}

/// `true` on a self-consistent JFM header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    header_ok(b).is_some()
}

/// Census; `None` unless `id`/`lf` checks hold.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Jfm> {
    let (id, nt, lf, lh, chars) = header_ok(b)?;
    Some(Jfm {
        id,
        nt,
        lf,
        lh,
        chars,
        widths: w16(b, 12),
        heights: w16(b, 14),
        depths: w16(b, 16),
        italics: w16(b, 18),
        lig_kerns: w16(b, 20),
        kerns: w16(b, 22),
        glues: w16(b, 24),
        params: w16(b, 26),
        checksum: w32(b, 28),
        design_size: w32(b, 32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // id=11(tate), nt=2, lh=2, bc=0 ec=1, ng=1 →
        // lf = 7 + 2 + 2 + 2 + 1 = 14 words = 56 B.
        let mut d = vec![
            0, 11, 0, 2, 0, 14, 0, 2, 0, 0, 0, 1, // id nt lf lh bc ec
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, // nw..np (ng=1)
            0x12, 0x34, 0x56, 0x78, // check_sum
            0x00, 0x0A, 0x00, 0x00, // design_size
        ];
        d.extend_from_slice(&[0; 8]); // char_type 2
        d.extend_from_slice(&[0; 8]); // char_info 2
        d.extend_from_slice(&[0; 4]); // glue
        assert_eq!(d.len(), 56);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"jfm"));
        let mut bad = fixture();
        bad[1] = 12; // invalid id
        assert!(!detect(&bad));
    }

    #[test]
    fn parses() {
        let j = parse(&fixture()).unwrap();
        assert_eq!(j.id, 11);
        assert!(!j.horizontal());
        assert_eq!(j.nt, 2);
        assert_eq!(j.chars, 2);
        assert_eq!(j.glues, 1);
        assert_eq!(j.checksum, 0x1234_5678);
    }

    #[test]
    fn rejects() {
        let mut d = fixture();
        d[4] = 0;
        d[5] = 99; // lf inconsistent
        assert!(parse(&d).is_none());
        assert!(parse(b"").is_none());
    }
}

//! HEIF/AVIF/HEIC brand sniffing via ISO BMFF `ftyp`.
//!
//! These formats are ISO BMFF containers; the `ftyp` box (at
//! offset 4 after the 32-bit size) declares the brand:
//! `heic`/`heix`/`hevc`/`heim`/`heis` (HEIC), `avif`/`avis`
//! (AVIF), `mif1`/`msf1` (generic HEIF). This parser reads the
//! box list and reports the primary brand family.
//!
//! ```
//! use izanagi_kit::heif;
//! let mut d = [0, 0, 0, 28].to_vec();
//! d.extend_from_slice(b"ftyp");
//! d.extend_from_slice(b"heic\x00\x00\x00\x00mif1miafheic");
//! let h = heif::parse(&d).unwrap();
//! assert_eq!(h.family, heif::Family::Heic);
//! ```

use std::string::String;
use std::vec::Vec;

/// Brand family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Family {
    /// `heic`/`heix`/`hevc`/`hevx`/`heim`/`heis`/`hevm`/`hevs` — HEVC still image.
    Heic,
    /// `avif`/`avis` — AV1 image.
    Avif,
    /// `mif1`/`msf1` — generic HEIF single/sequence.
    Heif,
    /// Other brand (raw code kept).
    Other,
}

/// A parsed HEIF-style container head.
#[derive(Clone, Debug, PartialEq)]
pub struct Heif {
    /// Primary brand 4cc as text.
    pub brand: String,
    /// Family classification.
    pub family: Family,
    /// Minor version field.
    pub minor: u32,
    /// Compatible brands.
    pub compat: Vec<String>,
    /// Top-level box types after `ftyp` (`meta`, `mdat`…).
    pub box_types: Vec<String>,
}

fn family_of(brand: &str) -> Family {
    match brand {
        "heic" | "heix" | "hevc" | "hevx" | "heim" | "heis" | "hevm" | "hevs" => Family::Heic,
        "avif" | "avis" => Family::Avif,
        "mif1" | "msf1" => Family::Heif,
        _ => Family::Other,
    }
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

fn ascii4(d: &[u8], at: usize) -> Option<String> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(String::from_utf8_lossy(s).into_owned())
}

/// Parses an HEIF-family container: `ftyp` must be the first box
/// and its primary brand must be a known HEIF/HEIC/AVIF brand.
pub fn parse(d: &[u8]) -> Option<Heif> {
    if d.len() < 16 || u32be(d, 0)? < 16 {
        return None;
    }
    if d.get(4..8) != Some(b"ftyp") {
        return None;
    }
    let brand = ascii4(d, 8)?;
    let family = family_of(&brand);
    if family == Family::Other {
        return None;
    }
    let minor = u32be(d, 12)?;
    let box_len = u32be(d, 0)? as usize;
    if box_len > d.len() {
        return None;
    }
    let mut compat = Vec::new();
    let mut at = 16usize;
    while at + 4 <= box_len {
        let b = ascii4(d, at)?;
        compat.push(b);
        at += 4;
    }
    // walk remaining top-level box headers
    let mut box_types = Vec::new();
    let mut pos = box_len;
    while pos + 8 <= d.len() {
        let len = u32be(d, pos)?;
        if len < 8 || pos.checked_add(len as usize)? > d.len() {
            return None;
        }
        box_types.push(ascii4(d, pos + 4)?);
        pos += len as usize;
        if box_types.len() > 256 {
            return None;
        }
    }
    Some(Heif {
        brand,
        family,
        minor,
        compat,
        box_types,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(brand: &[u8]) -> Vec<u8> {
        let mut d = vec![0, 0, 0, 24];
        d.extend_from_slice(b"ftyp");
        d.extend_from_slice(brand);
        d.extend_from_slice(&0u32.to_be_bytes());
        d.extend_from_slice(b"mif1miaf");
        d
    }

    #[test]
    fn detects_families() {
        assert_eq!(parse(&make(b"heic")).unwrap().family, Family::Heic);
        assert_eq!(parse(&make(b"avif")).unwrap().family, Family::Avif);
        assert_eq!(parse(&make(b"mif1")).unwrap().family, Family::Heif);
        let h = parse(&make(b"avis")).unwrap();
        assert_eq!(h.compat, vec!["mif1".to_string(), "miaf".to_string()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 8]).is_none());
        assert!(parse(&make(b"jpeg")).is_none());
        let mut d = make(b"heic");
        d[7] = b'x'; // not ftyp
        assert!(parse(&d).is_none());
    }
}

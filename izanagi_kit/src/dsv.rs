//! DeSmuME `.dsv` save parsing — raw NDS save plus a footer.
//!
//! A `.dsv` appends a 122-byte footer to the raw save data. The
//! footer begins with the marker `|<--Snip` … (a hint that snipping
//! it recovers a raw `.sav`) and stores `raw_size u32LE` 4 bytes
//! before its end; `raw_size + 122 == file size`.
//!
//! ```
//! use izanagi_kit::dsv;
//! let mut d = vec![0u8; 8192 + 122];
//! let f = 8192;
//! d[f..f + 8].copy_from_slice(b"|<--Snip");
//! d[f + 118..f + 122].copy_from_slice(&8192u32.to_le_bytes());
//! let v = dsv::parse(&d).unwrap();
//! assert_eq!(v.raw_len, 8192);
//! ```

/// DeSmuME footer length.
pub const FOOTER_LEN: usize = 122;
/// Footer marker prefix.
pub const MARKER: &[u8; 8] = b"|<--Snip";

/// A parsed `.dsv`.
#[derive(Clone, Debug, PartialEq)]
pub struct Dsv {
    /// Raw save length (bytes before the footer).
    pub raw_len: usize,
    /// Footer start offset.
    pub footer_offset: usize,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a `.dsv`: footer marker + `raw_len` such that
/// `raw_len + 122 == len`. The declared `raw_len` is the region a
/// raw `.sav` would occupy.
pub fn parse(d: &[u8]) -> Option<Dsv> {
    if d.len() <= FOOTER_LEN {
        return None;
    }
    let f = d.len() - FOOTER_LEN;
    let foot = d.get(f..)?;
    if foot.get(..8)? != MARKER {
        return None;
    }
    let raw_len = u32le(foot, FOOTER_LEN - 4)? as usize;
    if raw_len != f {
        return None;
    }
    Some(Dsv {
        raw_len,
        footer_offset: f,
    })
}

/// True when the input carries the DeSmuME footer.
pub fn is_dsv(d: &[u8]) -> bool {
    d.len() > FOOTER_LEN && &d[d.len() - FOOTER_LEN..d.len() - FOOTER_LEN + 8] == MARKER
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture(raw: usize) -> Vec<u8> {
        let mut d = vec![0u8; raw + FOOTER_LEN];
        d[raw..raw + 8].copy_from_slice(MARKER);
        d[raw + 118..raw + 122].copy_from_slice(&(raw as u32).to_le_bytes());
        d
    }

    #[test]
    fn parses_footer() {
        let v = parse(&fixture(32768)).unwrap();
        assert_eq!(v.raw_len, 32768);
        assert_eq!(v.footer_offset, 32768);
        assert!(is_dsv(&fixture(32768)));
        assert!(!is_dsv(&vec![0u8; 32768]));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 64]).is_none()); // shorter than footer
        let mut d = fixture(4096);
        d[4096 + 118..4096 + 122].copy_from_slice(&1000u32.to_le_bytes());
        assert!(parse(&d).is_none()); // raw_len mismatch
        assert!(parse(&vec![0x55u8; 10000]).is_none()); // no marker
    }
}

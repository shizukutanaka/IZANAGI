//! Compiled Windows resource `.res` file scanner.
//!
//! `.res` files begin with a dummy entry whose `data_size` and
//! `header_size` are `0` and `0x20`, followed by `0xFFFF`-prefixed
//! `Type`/`Name` ordinals of `0` — a fixed 32-byte prologue:
//!
//! `00 00 00 00 | 20 00 00 00 | FF FF 00 00 | FF FF 00 00 | zeros…`.
//!
//! After the dummy, each resource entry repeats
//! `data_size u32, header_size u32, TYPE, NAME, …`; this parser validates
//! the prologue and counts `0xFFFF` ordinal-prefixed entry headers.
//!
//! ```
//! let mut f = vec![0u8; 32];
//! f[4..8].copy_from_slice(&0x20u32.to_le_bytes());   // header_size
//! f[8..12].copy_from_slice(&[0xFF, 0xFF, 0, 0]);      // TYPE ordinal
//! f[12..16].copy_from_slice(&[0xFF, 0xFF, 0, 0]);     // NAME ordinal
//! let r = izanagi_kit::res::parse(&f).unwrap();
//! assert_eq!(r.dummy_present, true);
//! ```
//!
//! Reference: Microsoft `.res` binary format notes (the 32-byte dummy
//! prologue and `0xFFFF` ordinal conventions documented in
//! `resfmt.txt` and the Windows SDK headers).

/// Parsed `.res` prologue.
#[derive(Debug, Clone, PartialEq)]
pub struct Res {
    /// `true` when the 32-byte dummy prologue is present.
    pub dummy_present: bool,
    /// Number of `0xFFFF`-prefixed entry-type markers seen at 8-byte
    /// strides after the prologue (a structural hint, not a full walk).
    pub ordinal_markers: usize,
}

fn u32le(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

/// Parse a `.res` file; `None` if the dummy prologue is absent.
pub fn parse(d: &[u8]) -> Option<Res> {
    if d.len() < 32 {
        return None;
    }
    // data_size == 0, header_size == 0x20, TYPE/NAME both ordinal zeros.
    if u32le(d, 0) != 0 || u32le(d, 4) != 0x20 {
        return None;
    }
    if u32le(d, 8) != 0x0000_FFFF || u32le(d, 12) != 0x0000_FFFF {
        return None;
    }
    if u32le(d, 16) != 0 || u32le(d, 20) != 0 || u32le(d, 24) != 0 || u32le(d, 28) != 0 {
        return None;
    }
    // Count 0xFFFF marker words in the remaining body (structural hint).
    let mut ordinal_markers = 0usize;
    let mut i = 32;
    while i + 12 <= d.len() {
        if u32le(d, i) == 0x0000_FFFF {
            ordinal_markers += 1;
        }
        i += 8;
    }
    Some(Res {
        dummy_present: true,
        ordinal_markers,
    })
}

/// `true` if the buffer looks like a `.res` file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res() -> Vec<u8> {
        let mut f = vec![0u8; 64];
        f[4..8].copy_from_slice(&0x20u32.to_le_bytes());
        f[8..12].copy_from_slice(&[0xFF, 0xFF, 0, 0]);
        f[12..16].copy_from_slice(&[0xFF, 0xFF, 0, 0]);
        // body marker at offset 32
        f[32..36].copy_from_slice(&[0xFF, 0xFF, 0, 0]);
        f
    }

    #[test]
    fn parses() {
        let r = parse(&res()).unwrap();
        assert!(r.dummy_present);
        assert_eq!(r.ordinal_markers, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 32]).is_none()); // header_size must be 0x20
        let mut f = res();
        f[4] = 0x21;
        assert!(parse(&f).is_none());
        let mut g = res();
        g[8] = 1;
        assert!(parse(&g).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&res()));
        assert!(!detect(b"\0\0\0\0"));
    }
}

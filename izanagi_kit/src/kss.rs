//! MSX KSS sound file parser.
//!
//! `.kss` files start with `KSCC` (plain KSS) or `KSSX` (extended).
//! The 16-byte common header is `[magic:4][load_addr u16le]
//! [load_size u16le][init_addr u16le][play_addr u16le]
//! [bank_offset u16le][bank_num u8][extra u8]`; KSSX appends
//! `first_song u8, last_song u8, psg_vol u8, scc_vol u8,
//! mgs_vol u8, pac_vol u8` at offset 16.
//!
//! ```
//! let f = b"KSCC\x00\x40\x00\x10\x00\x40\x00\x41\x00\x00\x00\x00";
//! let k = izanagi_kit::kss::parse(f).unwrap();
//! assert!(!k.extended);
//! assert_eq!(k.load_addr, 0x4000);
//! assert_eq!(k.load_size, 0x1000);
//! ```

/// Parsed KSS header.
#[derive(Debug, Clone, PartialEq)]
pub struct Kss {
    /// `KSSX` signature seen (extended header with volume fields).
    pub extended: bool,
    /// Z80 load address of the music data.
    pub load_addr: u16,
    /// Byte size of the loaded data.
    pub load_size: u16,
    /// INIT routine address.
    pub init_addr: u16,
    /// PLAY routine address (called each interrupt).
    pub play_addr: u16,
    /// KSSX: first song index (`0` for plain KSCC files).
    pub first_song: u8,
    /// KSSX: last song index, `None` for plain files.
    pub last_song: Option<u8>,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

/// Parse a KSS file; `None` without `KSCC`/`KSSX` or when truncated.
pub fn parse(d: &[u8]) -> Option<Kss> {
    if d.len() < 16 {
        return None;
    }
    let extended = &d[..4] == b"KSSX";
    if !extended && &d[..4] != b"KSCC" {
        return None;
    }
    let load_addr = u16le(d, 4);
    let load_size = u16le(d, 6);
    let init_addr = u16le(d, 8);
    let play_addr = u16le(d, 10);
    // KSSX must have the extended volume/song bytes too.
    if extended && d.len() < 22 {
        return None;
    }
    let (first_song, last_song) = if extended {
        (d[16], Some(d[17]))
    } else {
        (0, None)
    };
    Some(Kss {
        extended,
        load_addr,
        load_size,
        init_addr,
        play_addr,
        first_song,
        last_song,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"KSCC\x00\x40\x00\x10\x00\x40\x00\x41\x00\x00\x00\x00";
        let k = parse(f).unwrap();
        assert!(!k.extended);
        assert_eq!(k.load_addr, 0x4000);
        assert_eq!(k.load_size, 0x1000);
        assert_eq!(k.init_addr, 0x4000);
        assert_eq!(k.play_addr, 0x4100);
        assert!(k.last_song.is_none());
    }

    #[test]
    fn kssx() {
        let mut f = b"KSSX\x00\x40\x00\x10\x00\x40\x00\x41".to_vec();
        f.extend_from_slice(&[0, 0, 0, 0, 0, 3, 0, 0, 0, 0]);
        let k = parse(&f).unwrap();
        assert!(k.extended);
        assert_eq!(k.first_song, 0);
        assert_eq!(k.last_song, Some(3));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"KSCC").is_none());
        assert!(parse(b"KSSX\x00\x40\x00\x10\x00\x40\x00\x41").is_none());
        assert!(parse(b"NSFE\x00\x40\x00\x10\x00\x40\x00\x41\x00\x00").is_none());
    }
}

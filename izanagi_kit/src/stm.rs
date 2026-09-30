//! Scream Tracker 2 `.stm` module header scanner.
//!
//! `.stm` layout: a 20-byte song name at offset 0, magic `!Scream!` (or the
//! rarer `BMOD2STM`) at offset 20, a `0x1A` byte, file type (1 = song,
//! 2 = module), then `version u16` (e.g. `0x0215` for v2.15 — the
//! tracker writes major·100+minor BCD-ish values read as plain LE u16),
//! `tempo u8`, `patterns u8`, `global_volume u8`, `reserved[13]`, then
//! 32-byte pattern names and instrument blocks.
//!
//! ```
//! let mut f = vec![0u8; 48];
//! f[..4].copy_from_slice(b"Song");
//! f[20..28].copy_from_slice(b"!Scream!");
//! f[28] = 0x1A;
//! f[29] = 2;                       // module
//! f[30..32].copy_from_slice(&0x0215u16.to_le_bytes()); // v2.15
//! f[32] = 125;                     // tempo
//! f[33] = 2;                       // patterns
//! let s = izanagi_kit::stm::parse(&f).unwrap();
//! assert_eq!(s.song_name.as_deref(), Some("Song"));
//! assert_eq!(s.version, 0x0215);
//! assert_eq!(s.patterns, 2);
//! ```
//!
//! Reference: ST2/STM format notes (Jeffrey Lim's original ST2 docs; the
//! widely mirrored `stformat.txt` / `stmsound.txt` format descriptions).

/// Parsed STM header.
#[derive(Debug, Clone, PartialEq)]
pub struct Stm {
    /// Song name, up to 20 bytes (NUL-padded).
    pub song_name: Option<String>,
    /// `true` for `BMOD2STM` marker files, `false` for `!Scream!`.
    pub bmod_marker: bool,
    /// File type byte: 1 = song (no pattern data), 2 = module.
    pub file_type: u8,
    /// Raw version field (e.g. `0x0215` ≈ "v2.15").
    pub version: u16,
    /// Initial tempo (frames per row scaling used by ST2).
    pub tempo: u8,
    /// Declared pattern count.
    pub patterns: u8,
    /// Global volume byte.
    pub global_volume: u8,
}

fn u16le(d: &[u8], off: usize) -> u16 {
    d[off] as u16 | ((d[off + 1] as u16) << 8)
}

/// Parse an `.stm` file; `None` if the magic or type byte is wrong.
pub fn parse(d: &[u8]) -> Option<Stm> {
    if d.len() < 48 {
        return None;
    }
    let bmod_marker = match &d[20..28] {
        b"!Scream!" => false,
        b"BMOD2STM" => true,
        _ => return None,
    };
    if d[28] != 0x1A {
        return None;
    }
    let file_type = d[29];
    if file_type != 1 && file_type != 2 {
        return None;
    }
    let name_len = d[..20].iter().position(|&c| c == 0).unwrap_or(20);
    let song_name = if name_len == 0 {
        None
    } else {
        core::str::from_utf8(&d[..name_len])
            .ok()
            .map(|s| s.to_string())
    };
    Some(Stm {
        song_name,
        bmod_marker,
        file_type,
        version: u16le(d, 30),
        tempo: d[32],
        patterns: d[33],
        global_volume: d[34],
    })
}

/// `true` if the buffer looks like an `.stm` module.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stm() -> Vec<u8> {
        let mut f = vec![0u8; 64];
        f[..4].copy_from_slice(b"Song");
        f[20..28].copy_from_slice(b"!Scream!");
        f[28] = 0x1A;
        f[29] = 2;
        f[30..32].copy_from_slice(&0x0215u16.to_le_bytes());
        f[32] = 125;
        f[33] = 2;
        f[34] = 64;
        f
    }

    #[test]
    fn parses() {
        let s = parse(&stm()).unwrap();
        assert_eq!(s.song_name.as_deref(), Some("Song"));
        assert!(!s.bmod_marker);
        assert_eq!(s.file_type, 2);
        assert_eq!(s.version, 0x0215);
        assert_eq!(s.tempo, 125);
        assert_eq!(s.patterns, 2);
        assert_eq!(s.global_volume, 64);
    }

    #[test]
    fn bmod() {
        let mut f = stm();
        f[20..28].copy_from_slice(b"BMOD2STM");
        let s = parse(&f).unwrap();
        assert!(s.bmod_marker);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 64]).is_none());
        let mut f = stm();
        f[20] = b'x';
        assert!(parse(&f).is_none());
        let mut g = stm();
        g[28] = 0;
        assert!(parse(&g).is_none());
        let mut h = stm();
        h[29] = 9;
        assert!(parse(&h).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(&stm()));
        assert!(!detect(b"!Scream!"));
    }
}

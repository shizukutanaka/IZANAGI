//! Sega Genesis GYM music log parser.
//!
//! GYM is a register-write capture for the YM2612 + SN76489. Early
//! files are raw logs with no header; `GYMX` files prepend a tagged
//! header: `u32 packed_len` then four 32-byte NUL-padded strings —
//! song name, game name, publisher, emulator — followed by a 32-byte
//! dumper field and a 256-byte comment. This parser handles the
//! `GYMX` tagged form only.
//!
//! ```
//! let mut f = b"GYMX\x00\x00\x00\x00".to_vec();
//! f.extend_from_slice(b"SONGNAME                        ");
//! f.resize(200, 0);
//! let g = izanagi_kit::gym::parse(&f).unwrap();
//! assert_eq!(g.song.as_deref(), Some("SONGNAME"));
//! ```

/// Parsed GYMX header.
#[derive(Debug, Clone, PartialEq)]
pub struct Gym {
    /// Packed data length word at offset 4.
    pub packed_len: u32,
    /// Song name (32-byte field), `None` when blank.
    pub song: Option<String>,
    /// Game name, `None` when blank.
    pub game: Option<String>,
    /// Publisher string, `None` when blank.
    pub publisher: Option<String>,
    /// Emulator string, `None` when blank.
    pub emulator: Option<String>,
    /// Byte offset where the register-log data begins.
    pub data_offset: usize,
}

fn field(d: &[u8], off: usize) -> Option<String> {
    if d.len() < off + 32 {
        return None;
    }
    let f = &d[off..off + 32];
    let end = f.iter().position(|&b| b == 0).unwrap_or(32);
    let s = std::str::from_utf8(&f[..end]).ok()?.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Parse a `GYMX` header; `None` for raw GYM logs (no magic) or
/// truncation before the string table.
pub fn parse(d: &[u8]) -> Option<Gym> {
    if d.len() < 136 || &d[..4] != b"GYMX" {
        return None;
    }
    let packed_len = u32::from_le_bytes([d[4], d[5], d[6], d[7]]);
    Some(Gym {
        packed_len,
        song: field(d, 8),
        game: field(d, 40),
        publisher: field(d, 72),
        emulator: field(d, 104),
        data_offset: 424,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gym() -> Vec<u8> {
        let mut f = b"GYMX\x10\x00\x00\x00".to_vec();
        f.extend_from_slice(b"Song");
        f.resize(40, b' ');
        f.extend_from_slice(b"Game");
        f.resize(72, b' ');
        f.extend_from_slice(b"Publisher");
        f.resize(104, b' ');
        f.extend_from_slice(b"Emulator");
        f.resize(424, 0);
        f
    }

    #[test]
    fn basic() {
        let g = parse(&gym()).unwrap();
        assert_eq!(g.packed_len, 0x10);
        assert_eq!(g.song.as_deref(), Some("Song"));
        assert_eq!(g.game.as_deref(), Some("Game"));
        assert_eq!(g.emulator.as_deref(), Some("Emulator"));
        assert_eq!(g.data_offset, 424);
    }

    #[test]
    fn blank_fields() {
        let mut f = b"GYMX\x00\x00\x00\x00".to_vec();
        f.resize(424, 0);
        let g = parse(&f).unwrap();
        assert!(g.song.is_none() && g.game.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"GYMX").is_none());
        assert!(parse(&[b'G'; 200]).is_none());
    }
}

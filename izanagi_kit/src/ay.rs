//! ZX Spectrum AY sound file parser (`ZXAYEMUL`).
//!
//! A `.ay` file opens with the 8-byte signature `ZXAYEMUL`, then
//! `file_version u8, player_version u8, pspecial u16be, pauthor u16be,
//! pmisc u16be, num_songs u8, first_song u8, psongs u16be` — pointers
//! are offsets from the file start into NUL-terminated strings and
//! song structures.
//!
//! ```
//! let f = b"ZXAYEMUL\x03\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x14";
//! let a = izanagi_kit::ay::parse(f).unwrap();
//! assert_eq!(a.file_version, 3);
//! assert_eq!(a.songs, 3); // stored as count-1
//! assert_eq!(a.first_song, 0);
//! ```

/// Parsed AY file header.
#[derive(Debug, Clone, PartialEq)]
pub struct Ay {
    /// Header format version (typically 3).
    pub file_version: u8,
    /// Required player version (typically 0).
    pub player_version: u8,
    /// Song count (`num_songs` byte + 1).
    pub songs: usize,
    /// Song index to start from.
    pub first_song: u8,
    /// Offset of the author-name block (0 = absent).
    pub author_ptr: u16,
    /// Offset of the misc/comment block (0 = absent).
    pub misc_ptr: u16,
    /// Offset of the song-structure table.
    pub songs_ptr: u16,
}

fn u16be(d: &[u8], off: usize) -> u16 {
    ((d[off] as u16) << 8) | d[off + 1] as u16
}

/// Parse an AY file; `None` without `ZXAYEMUL` or when the 20-byte
/// fixed header is truncated.
pub fn parse(d: &[u8]) -> Option<Ay> {
    if d.len() < 20 || &d[..8] != b"ZXAYEMUL" {
        return None;
    }
    Some(Ay {
        file_version: d[8],
        player_version: d[9],
        songs: d[16] as usize + 1,
        first_song: d[17],
        author_ptr: u16be(d, 12),
        misc_ptr: u16be(d, 14),
        songs_ptr: u16be(d, 18),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"ZXAYEMUL\x03\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x14";
        let a = parse(f).unwrap();
        assert_eq!(a.file_version, 3);
        assert_eq!(a.player_version, 0);
        assert_eq!(a.songs, 3);
        assert_eq!(a.first_song, 0);
        assert_eq!(a.songs_ptr, 20);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"ZXAYEMU").is_none());
        assert!(parse(b"ZXAYEMUL").is_none()); // header truncated
        assert!(parse(b"PSIDXXXX\x03\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x14").is_none());
    }
}

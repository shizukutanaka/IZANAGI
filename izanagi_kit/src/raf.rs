//! Fujifilm RAF (Fuji RAW Format) header census.
//!
//! RAF starts with the 16-byte banner `FUJIFILMCCD-RAW `, then a fixed
//! header: 4-byte ASCII version at `0x54` (`"0201"`/`"0202"`), and a
//! big-endian pointer table — `u32be` JPEG offset/length at `0x5C`/`0x60`,
//! CFA (raw) offset/length at `0x64`/`0x68`.
//!
//! ```
//! let mut d = *b"FUJIFILMCCD-RAW \x00\x00\x00\x00";
//! let mut v = d.to_vec();
//! v.resize(0x54, 0);
//! v.extend_from_slice(b"0201");
//! v.resize(0x5C, 0);
//! v.extend_from_slice(&[0, 0, 0x10, 0, 0, 0, 1, 0, 0, 0, 0x80, 0, 0, 0, 0x40, 0]);
//! let r = izanagi_kit::raf::parse(&v).unwrap();
//! assert_eq!(&r.version, b"0201");
//! assert_eq!(r.jpeg_offset, 0x1000);
//! ```

/// The 16-byte banner.
pub const MAGIC: &[u8; 16] = b"FUJIFILMCCD-RAW ";

/// Census fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Raf {
    /// ASCII version bytes at `0x54` (`"0201"`/`"0202"`), zeroed when absent.
    pub version: [u8; 4],
    /// Camera/model identifier string capture (first up-to-32 printable bytes at `0x10`).
    pub camera: Option<usize>,
    /// `u32be` embedded JPEG offset.
    pub jpeg_offset: u32,
    /// `u32be` embedded JPEG length.
    pub jpeg_length: u32,
    /// `u32be` CFA raw block offset.
    pub cfa_offset: u32,
    /// `u32be` CFA raw block length.
    pub cfa_length: u32,
}

fn u32be(b: &[u8], i: usize) -> u32 {
    ((b[i] as u32) << 24) | ((b[i + 1] as u32) << 16) | ((b[i + 2] as u32) << 8) | (b[i + 3] as u32)
}

/// `true` on the `FUJIFILMCCD-RAW ` banner.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 16 && &b[..16] == MAGIC
}

/// Census; `None` on non-RAF or truncated header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Raf> {
    if !detect(b) || b.len() < 0x6C {
        return None;
    }
    let mut version = [0u8; 4];
    version.copy_from_slice(&b[0x54..0x58]);
    let camera = b[0x10..0x30]
        .iter()
        .position(|&c| c.is_ascii_graphic() || c == b' ')
        .map(|p| 0x10 + p);
    Some(Raf {
        version,
        camera,
        jpeg_offset: u32be(b, 0x5C),
        jpeg_length: u32be(b, 0x60),
        cfa_offset: u32be(b, 0x64),
        cfa_length: u32be(b, 0x68),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut v = MAGIC.to_vec();
        v.resize(0x54, 0);
        v.extend_from_slice(b"0201");
        v.resize(0x5C, 0);
        v.extend_from_slice(&[0, 0, 0x10, 0, 0, 0, 1, 0, 0, 0, 0x80, 0, 0, 0, 0x40, 0]);
        v
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"FUJIFILMCCD-RAX ..........."));
    }

    #[test]
    fn parses() {
        let r = parse(&fixture()).unwrap();
        assert_eq!(&r.version, b"0201");
        assert_eq!(r.jpeg_offset, 0x1000);
        assert_eq!(r.jpeg_length, 0x100);
        assert_eq!(r.cfa_offset, 0x8000);
        assert_eq!(r.cfa_length, 0x4000);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"FUJIFILMCCD-RAW ").is_none());
    }
}

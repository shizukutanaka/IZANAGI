//! PC Engine HES sound file parser.
//!
//! A `.hes` file begins with the `HESM` signature, a version byte,
//! a first-song index, the 16-bit little-endian init address, an
//! 8-bank map, and a 16-bit data-request word; data blocks tagged
//! `DATA` (load) / `ATAD` (verify tail) follow.
//!
//! ```
//! let mut f = b"HESM\x00\x00\x00\x20".to_vec();
//! f.extend_from_slice(&[0; 10]);
//! f.extend_from_slice(b"DATA\x10\x00\x00\x00");
//! let h = izanagi_kit::hes::parse(&f).unwrap();
//! assert_eq!(h.version, 0);
//! assert_eq!(h.init_addr, 0x2000);
//! assert_eq!(h.data_blocks, 1);
//! ```

/// Parsed HES header.
#[derive(Debug, Clone, PartialEq)]
pub struct Hes {
    /// Format version byte (usually 0).
    pub version: u8,
    /// Index of the song played on load.
    pub first_song: u8,
    /// 16-bit init routine address (little-endian).
    pub init_addr: u16,
    /// MPR bank map (8 bytes, one per MPR register).
    pub banks: [u8; 8],
    /// `DATA` chunk count after the header.
    pub data_blocks: usize,
    /// `ATAD` chunk count.
    pub atad_blocks: usize,
}

/// Parse an HES file; `None` without the `HESM` signature or when the
/// header is truncated (< 18 bytes).
pub fn parse(d: &[u8]) -> Option<Hes> {
    if d.len() < 18 || &d[..4] != b"HESM" {
        return None;
    }
    let mut banks = [0u8; 8];
    banks.copy_from_slice(&d[8..16]);
    let mut h = Hes {
        version: d[4],
        first_song: d[5],
        init_addr: d[6] as u16 | ((d[7] as u16) << 8),
        banks,
        data_blocks: 0,
        atad_blocks: 0,
    };
    let mut i = 18usize;
    while i + 4 <= d.len() {
        match &d[i..i + 4] {
            b"DATA" => {
                h.data_blocks += 1;
                if i + 12 <= d.len() {
                    let size = u32::from_le_bytes([d[i + 4], d[i + 5], d[i + 6], d[i + 7]]);
                    i = i + 12 + size as usize;
                    continue;
                }
            }
            b"ATAD" => {
                h.atad_blocks += 1;
                if i + 12 <= d.len() {
                    let size = u32::from_le_bytes([d[i + 4], d[i + 5], d[i + 6], d[i + 7]]);
                    i = i + 12 + size as usize;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut f = b"HESM\x00\x00\x00\x20".to_vec();
        f.extend_from_slice(&[0xE0, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7]);
        f.extend_from_slice(&[0, 0]);
        f.extend_from_slice(b"DATA\x10\x00\x00\x00");
        let h = parse(&f).unwrap();
        assert_eq!(h.init_addr, 0x2000);
        assert_eq!(h.banks[0], 0xE0);
        assert_eq!(h.data_blocks, 1);
    }

    #[test]
    fn sized_blocks_skip() {
        let mut f = b"HESM\x00\x00\x00\x20".to_vec();
        f.extend_from_slice(&[0; 10]);
        f.extend_from_slice(b"DATA\x02\x00\x00\x00\x00\x00\x00\x00"); // size 2 payload
        f.extend_from_slice(&[0xAA; 2]);
        f.extend_from_slice(b"ATAD\x00\x00\x00\x00");
        let h = parse(&f).unwrap();
        assert_eq!(h.data_blocks, 1);
        assert_eq!(h.atad_blocks, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"HES").is_none());
        assert!(parse(b"HESX\x00\x00\x00\x20\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00").is_none());
    }
}

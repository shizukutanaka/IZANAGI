//! Game Boy Advance ROM (`.gba`) cartridge header.
//!
//! The header occupies `0x00..0xC0`: a 4-byte ARM branch to the entry
//! point, the 156-byte compressed Nintendo logo, title, game/maker codes,
//! fixed `0x96`, unit/device bytes, version, and the `0xBD` checksum.
//!
//! ```
//! use izanagi_kit::gba::{parse, NINTENDO_LOGO, HEADER_LEN};
//!
//! let mut d = vec![0u8; HEADER_LEN + 16];
//! d[0..4].copy_from_slice(&[0x2E, 0x00, 0x00, 0xEA]); // b 0xC0
//! d[4..0xA0].copy_from_slice(&NINTENDO_LOGO);
//! d[0xA0..0xAC].copy_from_slice(b"HELLO\0\0\0\0\0\0\0");
//! d[0xAC..0xB0].copy_from_slice(b"ABCD"); // game code
//! d[0xB0..0xB2].copy_from_slice(b"01");   // maker code
//! d[0xB2] = 0x96;                         // fixed
//! d[0xBC] = 0;                            // version
//! let mut chk = 0u8;
//! for &b in &d[0xA0..=0xBC] { chk = chk.wrapping_sub(b); }
//! d[0xBD] = chk.wrapping_sub(0x19);
//! let g = parse(&d).unwrap();
//! assert!(g.logo_ok && g.checksum_ok && g.fixed_ok);
//! assert_eq!(g.game_code, "ABCD");
//! ```

use std::string::String;

/// Header size in bytes.
pub const HEADER_LEN: usize = 0xC0;
/// The fixed 156-byte compressed Nintendo logo at `0x04..0xA0`.
pub const NINTENDO_LOGO: [u8; 156] = [
    0x24, 0xFF, 0xAE, 0x51, 0x69, 0x9A, 0xA2, 0x21, 0x3D, 0x84, 0x82, 0x0A, 0x84, 0xE4, 0x09, 0xAD,
    0x11, 0x24, 0x8B, 0x98, 0xC0, 0x81, 0x7F, 0x21, 0xA3, 0x52, 0xBE, 0x19, 0x93, 0x09, 0xCE, 0x20,
    0x10, 0x46, 0x4A, 0x4A, 0xF8, 0x27, 0x31, 0xEC, 0x58, 0xC7, 0xE8, 0x33, 0x82, 0xE3, 0xCE, 0xBF,
    0x85, 0xF4, 0xDF, 0x94, 0xCE, 0x4B, 0x09, 0xC1, 0x94, 0x56, 0x8A, 0xC0, 0x13, 0x72, 0xA7, 0xFC,
    0x9F, 0x84, 0x4D, 0x73, 0xA3, 0xCA, 0x9A, 0x61, 0x58, 0x97, 0xA3, 0x27, 0xFC, 0x03, 0x98, 0x76,
    0x23, 0x1D, 0xC7, 0x61, 0x03, 0x04, 0xAE, 0x56, 0xBF, 0x38, 0x84, 0x00, 0x40, 0xA7, 0x0E, 0xFD,
    0xFF, 0x52, 0xFE, 0x03, 0x6F, 0x95, 0x30, 0xF1, 0x97, 0xFB, 0xC0, 0x85, 0x60, 0xD6, 0x80, 0x25,
    0xA9, 0x63, 0xBE, 0x03, 0x01, 0x4E, 0x38, 0xE2, 0xF9, 0xA2, 0x34, 0xFF, 0xBB, 0x3E, 0x03, 0x44,
    0x78, 0x00, 0x90, 0xCB, 0x88, 0x11, 0x3A, 0x94, 0x65, 0xC0, 0x7C, 0x63, 0x87, 0xF0, 0x3C, 0xAF,
    0xD6, 0x25, 0xE4, 0x8B, 0x38, 0x0A, 0xAC, 0x72, 0x21, 0xD4, 0xF8, 0x07,
];

fn txt(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).trim_end().to_string()
}

/// A decoded GBA cartridge header.
#[derive(Clone, Debug, PartialEq)]
pub struct Gba {
    /// Raw ARM branch instruction word (usually `b 0xC0`).
    pub entry: u32,
    /// Game title (up to 12 chars, trimmed).
    pub title: String,
    /// 4-char game code (`0xAC..0xB0`).
    pub game_code: String,
    /// 2-char maker code (`0xB0..0xB2`).
    pub maker_code: String,
    /// Whether byte `0xB2` is the required `0x96`.
    pub fixed_ok: bool,
    /// Unit code (0xB3; 0 = GBA).
    pub unit: u8,
    /// Device type (0xB4).
    pub device: u8,
    /// ROM version (0xBC).
    pub version: u8,
    /// Whether the compressed logo matched.
    pub logo_ok: bool,
    /// Whether the 0xBD header checksum matched.
    pub checksum_ok: bool,
    /// Whether the entry instruction is a forward ARM `b` (opcode 0xEA).
    pub entry_ok: bool,
}

/// Decode the 0xBD checksum field the spec mandates.
pub fn checksum(d: &[u8]) -> u8 {
    let mut chk = 0u8;
    for &b in &d[0xA0..=0xBC] {
        chk = chk.wrapping_sub(b);
    }
    chk.wrapping_sub(0x19)
}

/// Parse the GBA header. `None` when `d` cannot hold a header.
/// Sanity of each field is reported via the `*_ok` flags so homebrew
/// images (which often have an intact logo but custom codes) still parse.
pub fn parse(d: &[u8]) -> Option<Gba> {
    if d.len() < HEADER_LEN {
        return None;
    }
    let entry =
        u32::from(d[0]) | u32::from(d[1]) << 8 | u32::from(d[2]) << 16 | u32::from(d[3]) << 24;
    Some(Gba {
        entry,
        title: txt(&d[0xA0..0xAC]),
        game_code: String::from_utf8_lossy(&d[0xAC..0xB0]).into_owned(),
        maker_code: String::from_utf8_lossy(&d[0xB0..0xB2]).into_owned(),
        fixed_ok: d[0xB2] == 0x96,
        unit: d[0xB3],
        device: d[0xB4],
        version: d[0xBC],
        logo_ok: d[4..0xA0] == NINTENDO_LOGO[..],
        checksum_ok: checksum(d) == d[0xBD],
        entry_ok: (entry >> 24) == 0xEA,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn rom() -> Vec<u8> {
        let mut d = vec![0u8; 0x100];
        d[0..4].copy_from_slice(&[0x2E, 0x00, 0x00, 0xEA]);
        d[4..0xA0].copy_from_slice(&NINTENDO_LOGO);
        d[0xA0..0xAC].copy_from_slice(b"HELLO\0\0\0\0\0\0\0");
        d[0xAC..0xB0].copy_from_slice(b"ABCD");
        d[0xB0..0xB2].copy_from_slice(b"01");
        d[0xB2] = 0x96;
        d[0xB3] = 0x00;
        d[0xBC] = 0x00;
        let mut chk = 0u8;
        for &b in &d[0xA0..=0xBC] {
            chk = chk.wrapping_sub(b);
        }
        d[0xBD] = chk.wrapping_sub(0x19);
        d
    }

    #[test]
    fn fields_decode() {
        let g = parse(&rom()).unwrap();
        assert_eq!(g.entry, 0xEA00_002E);
        assert_eq!(g.title, "HELLO");
        assert_eq!(g.game_code, "ABCD");
        assert_eq!(g.maker_code, "01");
        assert!(g.fixed_ok && g.logo_ok && g.checksum_ok && g.entry_ok);
    }

    #[test]
    fn tamper_detection() {
        let mut d = rom();
        d[4] ^= 0xFF;
        assert!(!parse(&d).unwrap().logo_ok);
        let mut d = rom();
        d[0xA1] = b'X';
        let g = parse(&d).unwrap();
        assert!(!g.checksum_ok);
        let mut d = rom();
        d[0xB2] = 0x00;
        assert!(!parse(&d).unwrap().fixed_ok);
        let mut d = rom();
        d[3] = 0xEA;
        assert!(parse(&d).unwrap().entry_ok);
        let mut d = rom();
        d[3] = 0x00;
        assert!(!parse(&d).unwrap().entry_ok);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 0xBF]).is_none());
    }
}

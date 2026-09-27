//! N64 FlashRAM save (`.fla`) classification — 128 KiB raw dumps.
//!
//! FlashRAM saves are flat 128 KiB images with no header. Beyond a
//! fill statistic, the parser samples the first 8 KiB "system area"
//! for a game code: many tools write the cartridge's 4-byte game ID
//! (e.g. `NMOE` for Monster Truck Madness) at offset 0x08 — it is
//! reported when it is printable ASCII.
//!
//! ```
//! use izanagi_kit::fla;
//! let mut d = vec![0xFFu8; 131072];
//! d[8..12].copy_from_slice(b"NMQE");
//! let f = fla::parse(&d).unwrap();
//! assert_eq!(f.len, 131072);
//! assert_eq!(f.game_code.as_deref(), Some("NMQE"));
//! ```

use std::string::String;

/// FlashRAM image size.
pub const FLASH_LEN: usize = 131072;

/// A classified FlashRAM dump.
#[derive(Clone, Debug, PartialEq)]
pub struct Fla {
    /// Input length in bytes (always `131072`).
    pub len: usize,
    /// Bytes neither `0x00` nor `0xFF`, per mille.
    pub used_permille: u32,
    /// Optional 4-char game code found at offset 8.
    pub game_code: Option<String>,
}

/// Classifies a `.fla` dump: exact 128 KiB, fill statistic, and the
/// optional ASCII game code at offset 8.
pub fn parse(d: &[u8]) -> Option<Fla> {
    if d.len() != FLASH_LEN {
        return None;
    }
    let mut used = 0u64;
    for &b in d {
        if b != 0x00 && b != 0xFF {
            used += 1;
        }
    }
    let code = &d[8..12];
    let game_code = if code.iter().all(|&b| (0x20..=0x7E).contains(&b)) {
        Some(String::from_utf8_lossy(code).into_owned())
    } else {
        None
    };
    Some(Fla {
        len: d.len(),
        used_permille: ((used * 1024) / d.len() as u64) as u32,
        game_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn classifies() {
        let mut d = vec![0xFFu8; FLASH_LEN];
        d[8..12].copy_from_slice(b"NMWE");
        let f = parse(&d).unwrap();
        assert_eq!(f.game_code.as_deref(), Some("NMWE"));
        assert_eq!(f.used_permille, 0);
        // unprintable code region → None
        let d2 = vec![0x00u8; FLASH_LEN];
        assert!(parse(&d2).unwrap().game_code.is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(&vec![0u8; 65536]).is_none());
        assert!(parse(&vec![0u8; 131073]).is_none());
    }
}

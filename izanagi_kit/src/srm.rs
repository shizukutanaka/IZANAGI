//! Emulator battery-backed SRAM file (`.srm` / `.sav`) classification.
//!
//! These files carry no magic — they are raw SRAM/Flash dumps —
//! so classification is purely by size: 2 KiB (GB Camera pocket /
//! N64 Controller Pak quarter), 8 KiB, 32 KiB (SNES/GB SRAM),
//! 64 KiB, 128 KiB (GBA FlashRAM, N64 FlashRAM), 256 KiB,
//! 512 KiB and 1 MiB.
//!
//! ```
//! use izanagi_kit::srm;
//! let d = vec![0u8; 32768];
//! let s = srm::parse(&d).unwrap();
//! assert_eq!(s.kind, srm::Kind::Sram32k);
//! ```

/// Recognised SRAM size classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 2 KiB — small SRAM/EEPROM.
    Sram2k,
    /// 8 KiB — SRAM (GB MBC3 RTC carts etc.).
    Sram8k,
    /// 32 KiB — classic battery SRAM (SNES, GB/GBC, GG).
    Sram32k,
    /// 64 KiB — larger SRAM/EEPROM.
    Sram64k,
    /// 128 KiB — GBA/N64 FlashRAM-class save.
    Flash128k,
    /// 256 KiB — double FlashRAM.
    Flash256k,
    /// 512 KiB.
    Flash512k,
    /// 1 MiB — largest supported save RAM.
    Flash1m,
}

impl Kind {
    /// Byte size for a class.
    pub fn size(self) -> usize {
        match self {
            Kind::Sram2k => 2048,
            Kind::Sram8k => 8192,
            Kind::Sram32k => 32768,
            Kind::Sram64k => 65536,
            Kind::Flash128k => 131072,
            Kind::Flash256k => 262144,
            Kind::Flash512k => 524288,
            Kind::Flash1m => 1048576,
        }
    }

    /// Whether this is a FlashRAM-class size (≥128 KiB).
    pub fn is_flash(self) -> bool {
        (self.size()) >= Kind::Flash128k.size()
    }
}

/// A classified save dump.
#[derive(Clone, Debug, PartialEq)]
pub struct Srm {
    /// Size class.
    pub kind: Kind,
    /// Input length in bytes (== `kind.size()`).
    pub len: usize,
    /// Fraction of non-`0x00`/`0xFF` bytes, out of 1024
    /// (0 = empty/erased, ~1024 = fully used). Integer statistic.
    pub used_permille: u32,
}

/// Sizes accepted, smallest first.
pub const SIZES: &[usize] = &[2048, 8192, 32768, 65536, 131072, 262144, 524288, 1048576];

/// Classifies a raw save dump by length and computes a fill
/// statistic: bytes that are neither `0x00` nor `0xFF` (the two
/// erased patterns) per mille, sampled over the whole input.
pub fn parse(d: &[u8]) -> Option<Srm> {
    let kind = match d.len() {
        2048 => Kind::Sram2k,
        8192 => Kind::Sram8k,
        32768 => Kind::Sram32k,
        65536 => Kind::Sram64k,
        131072 => Kind::Flash128k,
        262144 => Kind::Flash256k,
        524288 => Kind::Flash512k,
        1048576 => Kind::Flash1m,
        _ => return None,
    };
    let mut used = 0u64;
    for &b in d {
        if b != 0x00 && b != 0xFF {
            used += 1;
        }
    }
    let used_permille = ((used * 1024) / d.len() as u64) as u32;
    Some(Srm {
        kind,
        len: d.len(),
        used_permille,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn classifies_sizes() {
        for &n in SIZES {
            let s = parse(&vec![0u8; n]).unwrap();
            assert_eq!(s.len, n);
            assert_eq!(s.kind.size(), n);
            assert_eq!(s.used_permille, 0);
        }
        assert_eq!(parse(&vec![0u8; 131072]).unwrap().kind, Kind::Flash128k);
        assert!(parse(&vec![0u8; 131072]).unwrap().kind.is_flash());
        assert!(!parse(&vec![0u8; 32768]).unwrap().kind.is_flash());
    }

    #[test]
    fn fill_statistic() {
        let mut d = vec![0xFFu8; 32768];
        for b in d.iter_mut().take(8192) {
            *b = 7; // 1/4 of the file is "used"
        }
        let s = parse(&d).unwrap();
        assert_eq!(s.used_permille, 256);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 1024]).is_none());
        assert!(parse(&[0u8; 32770]).is_none());
    }
}

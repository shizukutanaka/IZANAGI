//! N64 EEPROM save (`.eep`) classification — 4Kbit/16Kbit raw dumps.
//!
//! N64 EEPROM saves are raw serial-EEPROM images: 512 bytes for a
//! 4Kbit chip or 2048 bytes for a 16Kbit chip, delivered in
//! big-endian 8-byte pages. There is no header; classification is
//! by size plus a fill statistic over the two erased patterns
//! (`0x00`, `0xFF`).
//!
//! ```
//! use izanagi_kit::eep;
//! let d = vec![0xFFu8; 2048];
//! let e = eep::parse(&d).unwrap();
//! assert_eq!(e.kbit, 16);
//! ```

/// EEPROM capacity classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    /// 512 bytes — 4 Kbit (93C46).
    Eeprom4k,
    /// 2048 bytes — 16 Kbit (93C66).
    Eeprom16k,
}

/// A classified EEPROM dump.
#[derive(Clone, Debug, PartialEq)]
pub struct Eep {
    /// Chip class.
    pub chip: Chip,
    /// Capacity in kbit (4 or 16).
    pub kbit: u32,
    /// Input length in bytes.
    pub len: usize,
    /// Bytes that are neither `0x00` nor `0xFF`, per mille
    /// (0 = fully erased).
    pub used_permille: u32,
    /// Count of 8-byte pages where every byte is `0xFF` (erased).
    pub erased_pages: u32,
}

/// Page size the serial EEPROM protocol addresses.
pub const PAGE_LEN: usize = 8;

/// Classifies a `.eep` dump: 512- or 2048-byte length, plus fill and
/// erased-page statistics.
pub fn parse(d: &[u8]) -> Option<Eep> {
    let chip = match d.len() {
        512 => Chip::Eeprom4k,
        2048 => Chip::Eeprom16k,
        _ => return None,
    };
    let mut used = 0u64;
    for &b in d {
        if b != 0x00 && b != 0xFF {
            used += 1;
        }
    }
    let mut erased_pages = 0u32;
    for page in d.chunks(PAGE_LEN) {
        if page.iter().all(|&b| b == 0xFF) {
            erased_pages += 1;
        }
    }
    Some(Eep {
        chip,
        kbit: if chip == Chip::Eeprom4k { 4 } else { 16 },
        len: d.len(),
        used_permille: ((used * 1024) / d.len() as u64) as u32,
        erased_pages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn classifies() {
        let e = parse(&vec![0xFFu8; 512]).unwrap();
        assert_eq!(e.chip, Chip::Eeprom4k);
        assert_eq!(e.kbit, 4);
        assert_eq!(e.erased_pages, 64);
        let e = parse(&vec![0u8; 2048]).unwrap();
        assert_eq!(e.kbit, 16);
        assert_eq!(e.erased_pages, 0);
        assert_eq!(e.used_permille, 0);
    }

    #[test]
    fn fill_and_reject() {
        let mut d = vec![0u8; 2048];
        for b in d.iter_mut().take(1024) {
            *b = 0x12;
        }
        assert_eq!(parse(&d).unwrap().used_permille, 512);
        assert!(parse(&[0u8; 1000]).is_none());
        assert!(parse(&[0u8; 4096]).is_none());
    }
}

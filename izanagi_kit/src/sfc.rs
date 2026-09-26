//! Super Famicom / SNES ROM image (`.sfc` / `.smc`).
//!
//! `.smc` files prepend a 512-byte copier header (detected by file size);
//! the cartridge header sits at `0x7FC0` for LoROM or `0xFFC0` for HiROM
//! images. When the map-mode byte is ambiguous both locations are scored
//! (`checksum ^ complement == 0xFFFF`, sane map mode and size codes,
//! printable title) and the winner wins.
//!
//! ```
//! use izanagi_kit::sfc::{parse, MapMode};
//!
//! let mut d = vec![0u8; 0x10000]; // 64 KiB LoROM image
//! d[0x7FC0..0x7FD5].copy_from_slice(b"TEST GAME\0\0\0\0\0\0\0\0\0\0\0\0");
//! d[0x7FD5] = 0x20; // LoROM map mode
//! d[0x7FD6] = 0x00; // plain ROM
//! d[0x7FD7] = 0x07; // 2^7 = 128 KiB (file is shorter on purpose)
//! d[0x7FD8] = 0x03; // 8 KiB SRAM
//! d[0x7FD9] = 0x01; // USA
//! d[0x7FDB] = 0x00; // version 0
//! d[0x7FDC..0x7FDE].copy_from_slice(&0xAA55u16.to_le_bytes());
//! d[0x7FDE..0x7FE0].copy_from_slice(&0x55AAu16.to_le_bytes());
//! let s = parse(&d).unwrap();
//! assert_eq!(s.map_mode, MapMode::LoRom);
//! assert_eq!(s.header_at, 0x7FC0);
//! assert!(s.checksum_ok && !s.has_copier_header);
//! ```

use std::string::String;
use std::vec::Vec;

/// Copier (`.smc`) header size when present.
pub const COPIER: usize = 512;
/// LoROM header location inside the image (after any copier header).
pub const LOROM_HDR: usize = 0x7FC0;
/// HiROM header location.
pub const HIROM_HDR: usize = 0xFFC0;
/// Extended HiROM header location.
pub const EXHIROM_HDR: usize = 0x40_FFC0;

/// Map-mode byte interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapMode {
    /// LoROM ($8000-mapped banks).
    LoRom,
    /// HiROM (full 64 KiB banks).
    HiRom,
    /// SA-1 (LoROM addressing, co-processor).
    Sa1,
    /// ExHiROM.
    ExHiRom,
    /// LoROM + FastROM timing.
    LoRomFast,
    /// HiROM + FastROM timing.
    HiRomFast,
    /// Anything else.
    Other(u8),
}

fn map_mode(b: u8) -> MapMode {
    match b {
        0x20 => MapMode::LoRom,
        0x21 => MapMode::HiRom,
        0x23 => MapMode::Sa1,
        0x25 => MapMode::ExHiRom,
        0x30 => MapMode::LoRomFast,
        0x31 => MapMode::HiRomFast,
        0x35 => MapMode::ExHiRom,
        v => MapMode::Other(v),
    }
}

/// A decoded SNES cartridge header.
#[derive(Clone, Debug, PartialEq)]
pub struct Sfc {
    /// Offset in `d` where the header was found.
    pub header_at: usize,
    /// Whether a 512-byte copier header was detected and skipped.
    pub has_copier_header: bool,
    /// Game title (21 bytes, trimmed at NUL).
    pub title: String,
    /// Map-mode byte and class.
    pub map_mode_byte: u8,
    /// Map mode class.
    pub map_mode: MapMode,
    /// Cartridge type byte (co-processor flags).
    pub cart_type: u8,
    /// ROM size exponent (bytes = `1 KiB << n`).
    pub rom_size_exp: u8,
    /// SRAM size exponent (bytes = `1 KiB << n`).
    pub ram_size_exp: u8,
    /// Country code byte.
    pub country: u8,
    /// Licensee code byte.
    pub licensee: u8,
    /// Mask ROM version.
    pub version: u8,
    /// Whether `checksum ^ complement == 0xFFFF`.
    pub checksum_ok: bool,
    /// Byte size of the image minus any copier header.
    pub data_len: usize,
}

fn u16le(d: &[u8], at: usize) -> u16 {
    u16::from(d[at]) | u16::from(d[at + 1]) << 8
}

fn score(d: &[u8], at: usize) -> u32 {
    let mut s = 0u32;
    if u16le(d, at + 0x1C) ^ u16le(d, at + 0x1E) == 0xFFFF {
        s += 2;
    }
    match map_mode(d[at + 0x15]) {
        MapMode::Other(_) => {}
        _ => s += 1,
    }
    if d[at + 0x17] <= 0x0D {
        s += 1;
    }
    let printable = d[at..at + 21]
        .iter()
        .filter(|&&b| (0x20..=0x7E).contains(&b) || b == 0)
        .count();
    if printable >= 16 {
        s += 1;
    }
    s
}

fn read_header(d: &[u8], at: usize, has_copier: bool) -> Sfc {
    let h = &d[at..at + 0x40];
    let t_end = h[..21].iter().position(|&b| b == 0).unwrap_or(21);
    Sfc {
        header_at: at,
        has_copier_header: has_copier,
        title: String::from_utf8_lossy(&h[..t_end]).into_owned(),
        map_mode_byte: h[0x15],
        map_mode: map_mode(h[0x15]),
        cart_type: h[0x16],
        rom_size_exp: h[0x17],
        ram_size_exp: h[0x18],
        country: h[0x19],
        licensee: h[0x1A],
        version: h[0x1B],
        checksum_ok: u16le(h, 0x1C) ^ u16le(h, 0x1E) == 0xFFFF,
        data_len: d.len(),
    }
}

/// Parse the image. `None` when `d` cannot hold even a minimum LoROM
/// header after any copier header.
pub fn parse(d: &[u8]) -> Option<Sfc> {
    let base = if d.len() % 1024 == 512 { COPIER } else { 0 };
    let d = &d[base..];
    if d.len() < LOROM_HDR + 0x40 {
        return None;
    }
    let mut candidates: Vec<usize> = Vec::new();
    for at in [LOROM_HDR, HIROM_HDR, EXHIROM_HDR] {
        if at + 0x40 <= d.len() {
            candidates.push(at);
        }
    }
    let mut best = candidates[0];
    let mut best_score = 0;
    for &at in &candidates {
        let s = score(d, at);
        if s > best_score {
            best = at;
            best_score = s;
        }
    }
    Some(read_header(d, best, base == COPIER))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image(header: usize, len: usize, mode: u8) -> Vec<u8> {
        let mut d = vec![0u8; len];
        d[header..header + 21].copy_from_slice(b"TEST GAME\0\0\0\0\0\0\0\0\0\0\0\0");
        d[header + 0x15] = mode;
        d[header + 0x16] = 0x02;
        d[header + 0x17] = 0x07;
        d[header + 0x18] = 0x03;
        d[header + 0x19] = 0x01;
        d[header + 0x1A] = 0x33;
        d[header + 0x1B] = 0x00;
        d[header + 0x1C..header + 0x1E].copy_from_slice(&0xAA55u16.to_le_bytes());
        d[header + 0x1E..header + 0x20].copy_from_slice(&0x55AAu16.to_le_bytes());
        d
    }

    #[test]
    fn lorom_header() {
        let s = parse(&image(LOROM_HDR, 0x10000, 0x20)).unwrap();
        assert_eq!(s.header_at, LOROM_HDR);
        assert_eq!(s.title, "TEST GAME");
        assert_eq!(s.map_mode, MapMode::LoRom);
        assert!(s.checksum_ok);
        assert_eq!(s.rom_size_exp, 7);
        assert!(!s.has_copier_header);
    }

    #[test]
    fn hirom_header() {
        // 128 KiB image, header at 0xFFC0; LoROM location stays zero
        // (score 0 there — no complement pair).
        let s = parse(&image(HIROM_HDR, 0x20000, 0x21)).unwrap();
        assert_eq!(s.header_at, HIROM_HDR);
        assert_eq!(s.map_mode, MapMode::HiRom);
    }

    #[test]
    fn copier_header_skipped() {
        let mut d = vec![0u8; COPIER];
        d.extend_from_slice(&image(LOROM_HDR, 0x10000, 0x20));
        let s = parse(&d).unwrap();
        assert!(s.has_copier_header);
        assert_eq!(s.header_at, LOROM_HDR);
        assert_eq!(s.data_len, 0x10000);
    }

    #[test]
    fn scoring_picks_the_real_header() {
        // Ambiguous: garbage at LoROM position scores below the real
        // HiROM header.
        let mut d = image(HIROM_HDR, 0x20000, 0x21);
        for b in d[LOROM_HDR..LOROM_HDR + 0x40].iter_mut() {
            *b = 0x77;
        }
        let s = parse(&d).unwrap();
        assert_eq!(s.header_at, HIROM_HDR);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 0x7000]).is_none());
        assert!(parse(&[0u8; 512]).is_none());
    }
}

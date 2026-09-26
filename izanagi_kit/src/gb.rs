//! Game Boy ROM (`.gb` / `.gbc`) cartridge header.
//!
//! The header lives at `0x0100..=0x014F` of the ROM image: entry point,
//! the 48-byte Nintendo logo, title, CGB flag, licensee, cartridge type,
//! ROM/RAM size codes, destination, version, and two checksums.
//!
//! ```
//! use izanagi_kit::gb::{parse, NINTENDO_LOGO};
//!
//! let mut d = vec![0u8; 0x150];
//! d[0x100] = 0x00; d[0x101] = 0xC3; // nop; jp
//! d[0x102] = 0x50; d[0x103] = 0x01; //   -> 0x150
//! d[0x104..0x134].copy_from_slice(&NINTENDO_LOGO);
//! d[0x134..0x139].copy_from_slice(b"HELLO");
//! d[0x143] = 0x80; // CGB compatible
//! d[0x147] = 0x01; // MBC1
//! d[0x148] = 0x02; // 128 KiB ROM
//! d[0x149] = 0x03; // 32 KiB RAM
//! let mut chk = 0u8;
//! for &b in &d[0x134..=0x14C] { chk = chk.wrapping_sub(b).wrapping_sub(1); }
//! d[0x14D] = chk;
//! let g = parse(&d).unwrap();
//! assert!(g.logo_ok && g.header_ok);
//! assert_eq!(g.title, "HELLO");
//! assert_eq!(g.rom_bytes, 128 * 1024);
//! ```

use std::string::String;

/// Offset of the cartridge header.
pub const HEADER: usize = 0x100;
/// Header size (through the global checksum).
pub const HEADER_LEN: usize = 0x50;
/// The fixed 48-byte Nintendo logo at `0x104..0x134`.
pub const NINTENDO_LOGO: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

/// CGB support flag at 0x143.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cgb {
    /// DMG only.
    Dmg,
    /// Runs on DMG and CGB.
    Compatible,
    /// CGB only.
    Only,
    /// Anything else.
    Other(u8),
}

/// Destination byte at 0x14A.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dest {
    /// Japanese market.
    Japan,
    /// Overseas market.
    Overseas,
    /// Anything else.
    Other(u8),
}

/// Cartridge type at 0x147.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cart {
    /// Plain ROM.
    Rom,
    /// MBC1 family.
    Mbc1,
    /// MBC2.
    Mbc2,
    /// MBC3 (with or without RTC/RAM/battery).
    Mbc3,
    /// MBC5 (with or without rumble/RAM/battery).
    Mbc5,
    /// MBC6.
    Mbc6,
    /// MBC7 (tilt sensor).
    Mbc7,
    /// Hudson HuC1/HuC3.
    Huc,
    /// Unrecognised code.
    Other(u8),
}

/// A decoded cartridge header.
#[derive(Clone, Debug, PartialEq)]
pub struct Gb {
    /// Game title (NUL/space trimmed, up to 16 chars).
    pub title: String,
    /// CGB flag.
    pub cgb: Cgb,
    /// New licensee code (2 ASCII chars; valid when `old_license == 0x33`).
    pub licensee: String,
    /// Old licensee byte (0x14B).
    pub old_licensee: u8,
    /// SGB support flag (0x03 = supports SGB).
    pub sgb: bool,
    /// Cartridge type byte and class.
    pub cart_byte: u8,
    /// Cartridge family.
    pub cart: Cart,
    /// Whether the cart maps external RAM.
    pub has_ram: bool,
    /// Whether the cart has a battery.
    pub has_battery: bool,
    /// Whether the cart has an RTC (MBC3 only).
    pub has_timer: bool,
    /// Whether the cart has rumble (MBC5/MBC7).
    pub has_rumble: bool,
    /// ROM size code.
    pub rom_code: u8,
    /// Decoded ROM size in bytes (`32 KiB << code` for 0..=8).
    pub rom_bytes: usize,
    /// RAM size code.
    pub ram_code: u8,
    /// Decoded external RAM in bytes.
    pub ram_bytes: usize,
    /// Destination market.
    pub dest: Dest,
    /// Mask ROM version byte.
    pub version: u8,
    /// Whether the Nintendo logo matched exactly.
    pub logo_ok: bool,
    /// Whether the 0x14D header checksum matched.
    pub header_ok: bool,
    /// Whether the 0x14E global checksum matched (needs the whole file).
    pub global_ok: bool,
}

fn header_checksum(d: &[u8]) -> u8 {
    let mut x = 0u8;
    for &b in &d[0x134..=0x14C] {
        x = x.wrapping_sub(b).wrapping_sub(1);
    }
    x
}

fn ram_bytes(code: u8) -> usize {
    match code {
        0x02 => 8 * 1024,
        0x03 => 32 * 1024,
        0x04 => 128 * 1024,
        0x05 => 64 * 1024,
        _ => 0,
    }
}

/// Parse the cartridge header. `None` only when the image is too short
/// to hold the header; individual field sanity is reported via the
/// `*_ok` flags so homebrew ROMs still parse.
pub fn parse(d: &[u8]) -> Option<Gb> {
    if d.len() < HEADER + HEADER_LEN {
        return None;
    }
    let logo_ok = d[0x104..0x134] == NINTENDO_LOGO[..];
    let t_end = 0x134 + d[0x134..0x144].iter().position(|&b| b == 0).unwrap_or(16);
    let title = String::from_utf8_lossy(&d[0x134..t_end])
        .trim_end()
        .to_string();
    let cgb = match d[0x143] {
        0x80 => Cgb::Compatible,
        0xC0 => Cgb::Only,
        0x00 => Cgb::Dmg,
        v => Cgb::Other(v),
    };
    let licensee = String::from_utf8_lossy(&d[0x144..0x146]).into_owned();
    let cart_byte = d[0x147];
    let (cart, ram, bat, timer, rumble) = match cart_byte {
        0x00 | 0x08 | 0x09 => (
            Cart::Rom,
            cart_byte == 0x08 || cart_byte == 0x09,
            cart_byte == 0x09,
            false,
            false,
        ),
        0x01..=0x03 => (
            Cart::Mbc1,
            cart_byte >= 0x02,
            cart_byte == 0x03,
            false,
            false,
        ),
        0x05 | 0x06 => (Cart::Mbc2, true, cart_byte == 0x06, false, false),
        0x0F..=0x13 => (
            Cart::Mbc3,
            cart_byte >= 0x10,
            matches!(cart_byte, 0x0F | 0x10 | 0x13),
            cart_byte == 0x0F || cart_byte == 0x10,
            false,
        ),
        0x19..=0x1E => (
            Cart::Mbc5,
            cart_byte >= 0x1A,
            matches!(cart_byte, 0x1B | 0x1E),
            false,
            cart_byte >= 0x1C,
        ),
        0x20 => (Cart::Mbc6, true, true, false, false),
        0x22 => (Cart::Mbc7, true, true, false, true),
        0xFE | 0xFF => (Cart::Huc, true, true, false, false),
        v => (Cart::Other(v), false, false, false, false),
    };
    let rom_code = d[0x148];
    let rom_bytes = if rom_code <= 8 {
        (32 * 1024) << rom_code
    } else {
        0
    };
    let ram_code = d[0x149];
    let dest = match d[0x14A] {
        0 => Dest::Japan,
        1 => Dest::Overseas,
        v => Dest::Other(v),
    };
    let header_ok = header_checksum(d) == d[0x14D];
    let want = u16::from(d[0x14E]) << 8 | u16::from(d[0x14F]);
    let mut sum: u32 = 0;
    for (i, &b) in d.iter().enumerate() {
        if i != 0x14E && i != 0x14F {
            sum += u32::from(b);
        }
    }
    let global_ok = sum as u16 == want;
    Some(Gb {
        title,
        cgb,
        licensee,
        old_licensee: d[0x14B],
        sgb: d[0x146] == 0x03,
        cart_byte,
        cart,
        has_ram: ram,
        has_battery: bat,
        has_timer: timer,
        has_rumble: rumble,
        rom_code,
        rom_bytes,
        ram_code,
        ram_bytes: ram_bytes(ram_code),
        dest,
        version: d[0x14C],
        logo_ok,
        header_ok,
        global_ok,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn rom() -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[0x100] = 0x00;
        d[0x101] = 0xC3;
        d[0x102] = 0x50;
        d[0x103] = 0x01;
        d[0x104..0x134].copy_from_slice(&NINTENDO_LOGO);
        d[0x134..0x139].copy_from_slice(b"HELLO");
        d[0x143] = 0x80;
        d[0x144..0x146].copy_from_slice(b"AB");
        d[0x147] = 0x01;
        d[0x148] = 0x02;
        d[0x149] = 0x03;
        d[0x14A] = 0x01;
        d[0x14B] = 0x33;
        d[0x14C] = 0x00;
        let mut chk = 0u8;
        for &b in &d[0x134..=0x14C] {
            chk = chk.wrapping_sub(b).wrapping_sub(1);
        }
        d[0x14D] = chk;
        let mut sum = 0u32;
        for (i, &b) in d.iter().enumerate() {
            if i != 0x14E && i != 0x14F {
                sum += u32::from(b);
            }
        }
        d[0x14E] = (sum >> 8) as u8;
        d[0x14F] = sum as u8;
        d
    }

    #[test]
    fn fields_decode() {
        let g = parse(&rom()).unwrap();
        assert_eq!(g.title, "HELLO");
        assert_eq!(g.cgb, Cgb::Compatible);
        assert_eq!(g.licensee, "AB");
        assert_eq!(g.cart, Cart::Mbc1);
        assert!(!g.has_ram && !g.has_battery);
        assert_eq!(g.rom_bytes, 128 * 1024);
        assert_eq!(g.ram_bytes, 32 * 1024);
        assert_eq!(g.dest, Dest::Overseas);
        assert!(g.logo_ok && g.header_ok && g.global_ok);
    }

    #[test]
    fn cart_table() {
        for (byte, want) in [
            (0x00u8, Cart::Rom),
            (0x03, Cart::Mbc1),
            (0x06, Cart::Mbc2),
            (0x10, Cart::Mbc3),
            (0x1E, Cart::Mbc5),
            (0x20, Cart::Mbc6),
            (0x22, Cart::Mbc7),
            (0xFE, Cart::Huc),
            (0x42, Cart::Other(0x42)),
        ] {
            let mut d = rom();
            d[0x147] = byte;
            assert_eq!(parse(&d).unwrap().cart, want);
        }
        let mut d = rom();
        d[0x147] = 0x1B; // MBC5+RAM+BAT
        let g = parse(&d).unwrap();
        assert!(g.has_ram && g.has_battery && !g.has_rumble);
        let mut d = rom();
        d[0x147] = 0x22; // MBC7 sensor+rumble
        let g = parse(&d).unwrap();
        assert!(g.has_rumble);
    }

    #[test]
    fn checksums_detect_tampering() {
        let mut d = rom();
        d[0x104] ^= 0xFF; // break logo
        assert!(!parse(&d).unwrap().logo_ok);
        let mut d = rom();
        d[0x135] = b'X'; // break header checksum (title change)
        let g = parse(&d).unwrap();
        assert!(!g.header_ok);
        let mut d = rom();
        d[0x200 - 1] ^= 0xFF; // break global checksum only
        let g = parse(&d).unwrap();
        assert!(g.header_ok && !g.global_ok);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 0x14F]).is_none());
    }
}

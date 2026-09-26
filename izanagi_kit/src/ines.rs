//! iNES / NES 2.0 ROM images (`.nes`) — the emulator cartridge
//! format Marat Fayzullin created for the iNES emulator and the
//! community later extended. A file is a 16-byte header
//! (`NES`+`0x1A`, PRG/CHR unit counts, two flags bytes), an optional
//! 512-byte trainer, then PRG ROM (16 KiB units) and CHR ROM
//! (8 KiB units).
//!
//! [`parse`] reads the header and resolves both sizing schemes —
//! classic unit counts, and NES 2.0's MSB-nibble extension plus the
//! exponent-multiplier form (`flags9` nibble = `0xF`). Mapper numbers
//! reassemble across `flags6`/`flags7` (and `flags8` on NES 2.0).
//!
//! ```
//! use izanagi_kit::ines::{parse, Mirroring};
//!
//! let mut d = vec![0u8; 16];
//! d[..4].copy_from_slice(b"NES\x1A");
//! d[4] = 2; // 32 KiB PRG
//! d[5] = 1; // 8 KiB CHR
//! d[6] = 0x10; // mapper 1 (low nibble), vertical mirror
//! d[7] = 0x00;
//! d.extend_from_slice(&vec![0u8; 2 * 16384 + 8192]);
//! let n = parse(&d).unwrap();
//! assert_eq!(n.mapper, 1);
//! assert_eq!(n.mirroring, Mirroring::Vertical);
//! assert_eq!(n.prg_bytes, 32768);
//! assert!(n.fits(&d));
//! ```

/// Nametable layout declared by flags6 bit 0 + bit 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirroring {
    /// Vertical arrangement ("horizontal mirroring" in some docs).
    Vertical,
    /// Horizontal arrangement.
    Horizontal,
    /// Four-screen VRAM (bit 3 overrides bit 0).
    FourScreen,
}

/// Which video standard the cart targets (flags9 / NES 2.0 flags12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tv {
    /// NTSC (60 Hz).
    Ntsc,
    /// PAL (50 Hz).
    Pal,
    /// Multi-region or other.
    Other,
}

/// A parsed 16-byte `.nes` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Nes {
    /// PRG ROM size in bytes (16 KiB units × count, or NES 2.0 form).
    pub prg_bytes: u64,
    /// CHR ROM size in bytes; 0 means the board uses CHR RAM.
    pub chr_bytes: u64,
    /// Full mapper number (iNES: 8 bits; NES 2.0: 12).
    pub mapper: u16,
    /// NES 2.0 submapper (flags8 high nibble; 0 on iNES).
    pub submapper: u8,
    /// Nametable mirroring.
    pub mirroring: Mirroring,
    /// Cartridge has battery-backed PRG RAM (flags6 bit 1).
    pub battery: bool,
    /// A 512-byte trainer follows the header (flags6 bit 2).
    pub trainer: bool,
    /// `true` when flags7 bits 2–3 are `10`.
    pub nes2: bool,
    /// Console type (flags7 bits 0–1): 0 NES, 1 Vs., 2 PlayChoice, 3
    /// extended.
    pub console: u8,
    /// TV system hint.
    pub tv: Tv,
    /// Declared volatile PRG-RAM in bytes (iNES flags8 in 8 KiB
    /// units; NES 2.0 low nibble of flags10 as `64 << n`).
    pub prg_ram: u64,
    /// Declared non-volatile PRG-(NV)RAM/EEPROM in bytes (NES 2.0
    /// high nibble of flags10; 0 on iNES).
    pub prg_nvram: u64,
}

fn rom_size(lsb: u8, msb_nibble: u8, unit: u64) -> u64 {
    if msb_nibble == 0xF {
        // Exponent-multiplier form: size = 2^E × (MM×2 + 1).
        let e = (lsb >> 4) & 0xF;
        let m = u64::from(lsb & 0xF);
        (1u64 << e).saturating_mul(m.saturating_mul(2).saturating_add(1))
    } else {
        (u64::from(msb_nibble) << 8 | u64::from(lsb)) * unit
    }
}

/// NES 2.0 shift-encoded RAM size: shift count `0` → absent,
/// otherwise `64 << n` bytes (n = 7 → 8 KiB).
fn shift_size(nibble: u8) -> u64 {
    if nibble == 0 {
        0
    } else {
        64u64 << u32::from(nibble)
    }
}

/// Parse the 16-byte header. `None` only on short input or a bad
/// magic — the body may still be truncated; check with [`Nes::fits`].
pub fn parse(d: &[u8]) -> Option<Nes> {
    if d.len() < 16 || &d[..4] != b"NES\x1A" {
        return None;
    }
    let f6 = d[6];
    let f7 = d[7];
    let nes2 = f7 & 0x0C == 0x08;
    let mirroring = if f6 & 0x08 != 0 {
        Mirroring::FourScreen
    } else if f6 & 0x01 != 0 {
        Mirroring::Horizontal
    } else {
        Mirroring::Vertical
    };
    let mapper_lo = f6 >> 4;
    let mapper_hi = f7 & 0xF0;
    let mut mapper = u16::from(mapper_hi) | u16::from(mapper_lo);
    let (prg_bytes, chr_bytes, submapper, prg_ram, prg_nvram) = if nes2 {
        let f8 = d[8];
        let f9 = d[9];
        let f10 = d[10];
        mapper |= u16::from(f8 & 0x0F) << 8;
        (
            rom_size(d[4], f9 & 0x0F, 16384),
            rom_size(d[5], f9 >> 4, 8192),
            f8 >> 4,
            shift_size(f10 & 0x0F),
            shift_size(f10 >> 4),
        )
    } else {
        // iNES: flags8 is the (rarely-used) PRG-RAM size byte, in 8 KiB
        // units; the common extension takes 0 to mean 8 KiB.
        let prg_ram = u64::from(if d[8] == 0 { 1 } else { d[8] }) * 8192;
        (
            u64::from(d[4]) * 16384,
            u64::from(d[5]) * 8192,
            0,
            prg_ram,
            0,
        )
    };
    let tv = if nes2 {
        match d[12] & 0x03 {
            0 => Tv::Ntsc,
            1 => Tv::Pal,
            _ => Tv::Other,
        }
    } else {
        match d[9] & 0x01 {
            0 => Tv::Ntsc,
            _ => Tv::Pal,
        }
    };
    Some(Nes {
        prg_bytes,
        chr_bytes,
        mapper,
        submapper,
        mirroring,
        battery: f6 & 0x02 != 0,
        trainer: f6 & 0x04 != 0,
        nes2,
        console: f7 & 0x03,
        tv,
        prg_ram,
        prg_nvram,
    })
}

impl Nes {
    /// Byte offset where PRG ROM data starts (past the trainer when
    /// present).
    pub fn prg_at(&self) -> usize {
        16 + if self.trainer { 512 } else { 0 }
    }

    /// Byte offset where CHR ROM data starts.
    pub fn chr_at(&self) -> usize {
        self.prg_at() + self.prg_bytes as usize
    }

    /// Byte offset just past CHR ROM (i.e. total expected image size).
    pub fn end(&self) -> usize {
        self.chr_at() + self.chr_bytes as usize
    }

    /// The file contains the full declared PRG+CHR payload.
    pub fn fits(&self, d: &[u8]) -> bool {
        d.len() >= self.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom(prg_units: u8, chr_units: u8, f6: u8, f7: u8) -> Vec<u8> {
        let mut d = vec![0u8; 16];
        d[..4].copy_from_slice(b"NES\x1A");
        d[4] = prg_units;
        d[5] = chr_units;
        d[6] = f6;
        d[7] = f7;
        let n = parse(&d.clone()).unwrap();
        d.extend_from_slice(&vec![0xAB; n.end() - 16]);
        d
    }

    #[test]
    fn ines_header() {
        // mapper 69 (0x45): flags6 low nibble 5, flags7 high nibble 4;
        // f6 bits 0/1/2 set → horizontal mirror + battery + trainer.
        let d = rom(2, 1, 0x57, 0x40);
        let n = parse(&d).unwrap();
        assert!(!n.nes2);
        assert_eq!(n.mapper, 0x45);
        assert_eq!(n.mirroring, Mirroring::Horizontal); // f6 bit0 set
        assert!(n.battery);
        assert!(n.trainer);
        assert_eq!(n.prg_at(), 16 + 512);
        assert_eq!(n.prg_bytes, 32768);
        assert_eq!(n.chr_at(), 16 + 512 + 32768);
        assert!(n.fits(&d));
        assert_eq!(n.prg_ram, 8192);
    }

    #[test]
    fn nes2_extended() {
        let mut d = vec![0u8; 16];
        d[..4].copy_from_slice(b"NES\x1A");
        d[4] = 0x12; // PRG units
        d[5] = 0x34; // CHR units
        d[6] = 0x0A; // battery + four-screen, mapper lo 0
        d[7] = 0x08; // NES 2.0 marker
        d[8] = 0x31; // mapper msb nibble 1, submapper 3
        d[9] = 0x12; // prg msb nibble 2, chr msb 1
        d[10] = 0x44; // prg ram 64<<4, nvram 64<<4
        d[12] = 1; // PAL
        let n = parse(&d).unwrap();
        assert!(n.nes2);
        assert_eq!(n.mapper, 0x100);
        assert_eq!(n.submapper, 3);
        assert_eq!(n.prg_bytes, 0x212 * 16384);
        assert_eq!(n.chr_bytes, 0x134 * 8192);
        assert_eq!(n.mirroring, Mirroring::FourScreen);
        assert_eq!(n.tv, Tv::Pal);
        assert_eq!(n.prg_ram, 64 << 4);
        assert_eq!(n.prg_nvram, 64 << 4);
    }

    #[test]
    fn nes2_multiplier_form() {
        let mut d = vec![0u8; 16];
        d[..4].copy_from_slice(b"NES\x1A");
        d[7] = 0x08;
        d[9] = 0x0F; // exponent form for PRG
        d[4] = 0x45; // E=4, M=5 → 2^4 × 11 = 176 bytes
        let n = parse(&d).unwrap();
        assert_eq!(n.prg_bytes, 176);
    }

    #[test]
    fn chr_ram_and_console_types() {
        // chr=0 → CHR RAM; flags7 console bits.
        let mut d = vec![0u8; 16];
        d[..4].copy_from_slice(b"NES\x1A");
        d[4] = 1;
        d[7] = 0x02; // PlayChoice
        let n = parse(&d).unwrap();
        assert_eq!(n.chr_bytes, 0);
        assert_eq!(n.console, 2);
    }

    #[test]
    fn malformed() {
        assert!(parse(b"").is_none());
        assert!(parse(b"NES\x00................").is_none());
        assert!(parse(b"NES\x1A").is_none());
        // header fine but body short → fits() reports, parse() accepts
        let d = rom(2, 0, 0, 0);
        let n = parse(&d).unwrap();
        assert!(n.fits(&d));
        assert!(!n.fits(&d[..20]));
    }
}

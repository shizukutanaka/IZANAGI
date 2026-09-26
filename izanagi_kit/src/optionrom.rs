//! PCI option (expansion) ROM image.
//!
//! A ROM image starts with `0x55 0xAA`, a size byte counting
//! 512-byte units, then the init entry point. A `PCIR` data
//! structure (PCI firmware spec §6.3) is pointed to by a u16
//! pointer at offset 24: it carries the vendor and device ids,
//! code type (0 = x86 BIOS, 1 = Open Firmware, 3 = EFI), image
//! length in 512-byte units and a last-image indicator.
//!
//! ```
//! use izanagi_kit::optionrom::{parse, MAGIC};
//!
//! let mut d = vec![0u8; 1024];
//! d[0] = MAGIC[0]; d[1] = MAGIC[1];
//! d[2] = 2;                    // 2 * 512 bytes
//! d[24] = 0x40;                // PCIR at offset 0x40
//! let p = &mut d[0x40..];
//! p[..4].copy_from_slice(b"PCIR");
//! let put = |p: &mut [u8], at: usize, v: u16| {
//!     p[at] = v as u8; p[at + 1] = (v >> 8) as u8;
//! };
//! put(p, 4, 0x8086); put(p, 6, 0x1234); // vendor, device
//! put(p, 10, 24);                      // struct length
//! put(p, 18, 2);                       // image = 2 units
//! p[22] = 0;                           // x86 code
//! p[23] = 0x80;                        // last image
//! let r = parse(&d).unwrap();
//! assert_eq!(r.size_bytes(), 1024);
//! let pcir = r.pcir(&d).unwrap();
//! assert_eq!(pcir.vendor, 0x8086);
//! assert!(pcir.is_last());
//! ```

/// Two-byte ROM signature.
pub const MAGIC: [u8; 2] = [0x55, 0xaa];
/// Offset of the `PCIR` pointer.
pub const PCIR_PTR: usize = 24;

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

/// The `PCIR` data structure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pcir {
    /// File offset of the structure.
    pub at: usize,
    /// PCI vendor id.
    pub vendor: u16,
    /// PCI device id.
    pub device: u16,
    /// Device list pointer (Vital Product Data).
    pub vital_product: u16,
    /// Structure length (16 on PCI 2.x, 24 on 3.x).
    pub length: u16,
    /// Structure revision.
    pub revision: u8,
    /// 24-bit base-class/sub-class/interface code.
    pub class_code: u32,
    /// Image length in 512-byte units.
    pub image_units: u16,
    /// Code revision level.
    pub code_revision: u16,
    /// Code type (0 x86 BIOS, 1 Open Firmware, 3 EFI).
    pub code_type: u8,
    /// Indicator byte; bit 7 = last image.
    pub indicator: u8,
}

impl Pcir {
    /// True when this is the last image in the ROM.
    pub fn is_last(&self) -> bool {
        self.indicator & 0x80 != 0
    }
    /// Image length in bytes.
    pub fn image_bytes(&self) -> usize {
        usize::from(self.image_units) * 512
    }
}

/// A parsed option ROM header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionRom {
    /// Size byte: image length in 512-byte units.
    pub size_units: u8,
    /// Offset of the `PCIR` structure (0 when absent).
    pub pcir_at: usize,
}

impl OptionRom {
    /// Declared image size in bytes.
    pub fn size_bytes(&self) -> usize {
        usize::from(self.size_units) * 512
    }
    /// Parse the `PCIR` structure, `None` when the pointer is 0 or
    /// the signature is wrong.
    pub fn pcir(&self, d: &[u8]) -> Option<Pcir> {
        if self.pcir_at == 0 {
            return None;
        }
        let p = d.get(self.pcir_at..self.pcir_at.checked_add(24)?)?;
        if p.get(..4)? != b"PCIR" {
            return None;
        }
        Some(Pcir {
            at: self.pcir_at,
            vendor: le16(p, 4)?,
            device: le16(p, 6)?,
            vital_product: le16(p, 8)?,
            length: le16(p, 10)?,
            revision: p.get(12).copied()?,
            class_code: u32::from(p.get(13).copied()?)
                | u32::from(p.get(14).copied()?) << 8
                | u32::from(p.get(15).copied()?) << 16,
            image_units: le16(p, 18)?,
            code_revision: le16(p, 20)?,
            code_type: p.get(22).copied()?,
            indicator: p.get(23).copied()?,
        })
    }
}

/// Parse the ROM header. Returns `None` when the signature is
/// missing or the buffer is shorter than 26 bytes.
pub fn parse(d: &[u8]) -> Option<OptionRom> {
    if d.get(..2)? != MAGIC {
        return None;
    }
    Some(OptionRom {
        size_units: d.get(2).copied()?,
        pcir_at: usize::from(le16(d, PCIR_PTR)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 2048];
        d[0] = 0x55;
        d[1] = 0xaa;
        d[2] = 4; // 2048 bytes
        d[24] = 0x60;
        let w = |d: &mut [u8], o: usize, v: u16| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
        };
        d[0x60..0x64].copy_from_slice(b"PCIR");
        w(&mut d, 0x64, 0x8086);
        w(&mut d, 0x66, 0x10de);
        w(&mut d, 0x68, 0x100);
        w(&mut d, 0x6a, 24);
        d[0x6c] = 0;
        d[0x6f] = 3; // class code byte 2 = base class (display)
        w(&mut d, 0x72, 4);
        w(&mut d, 0x74, 0x1234);
        d[0x76] = 3; // EFI
        d[0x77] = 0x80;
        d
    }

    #[test]
    fn header_and_pcir() {
        let d = fixture();
        let r = parse(&d).unwrap();
        assert_eq!(r.size_units, 4);
        assert_eq!(r.size_bytes(), 2048);
        let p = r.pcir(&d).unwrap();
        assert_eq!(p.vendor, 0x8086);
        assert_eq!(p.device, 0x10de);
        assert_eq!(p.vital_product, 0x100);
        assert_eq!(p.length, 24);
        assert_eq!(p.class_code, 0x0003_0000);
        assert_eq!(p.image_bytes(), 2048);
        assert_eq!(p.code_type, 3);
        assert!(p.is_last());
    }

    #[test]
    fn missing_pcir_pointer() {
        let mut d = fixture();
        d[24] = 0;
        d[25] = 0;
        let r = parse(&d).unwrap();
        assert!(r.pcir(&d).is_none());
        d[24] = 0x30; // points at zeros
        assert!(r.pcir(&d).is_none());
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(parse(&[0u8; 2]).is_none());
        let mut d = fixture();
        d[1] = 0xab;
        assert!(parse(&d).is_none());
    }
}

//! AppImage: an ELF runtime with a SquashFS payload. Type-2 and
//! type-3 images stamp bytes 8..=10 with `AI\x02` / `AI\x03`.
//!
//! ```
//! let mut f = vec![0u8; 64];
//! f[..4].copy_from_slice(b"\x7fELF");
//! f[4] = 2; // ELFCLASS64
//! f[5] = 1; // little-endian
//! f[6] = 1; // EV_CURRENT
//! f[8..11].copy_from_slice(b"AI\x02");
//! f[16..18].copy_from_slice(&2u16.to_le_bytes()); // EXEC
//! f[18..20].copy_from_slice(&0x3eu16.to_le_bytes()); // x86-64
//! let a = izanagi_kit::appimage::parse(&f).unwrap();
//! assert_eq!(a.kind, izanagi_kit::appimage::Kind::Type2);
//! assert!(a.is64);
//! assert_eq!(a.machine, 0x3e);
//! ```

/// AppImage generation inferred from the magic byte at offset 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Type 2: ELF + SquashFS payload (`AI\x02`).
    Type2,
    /// Type 3: ELF + SquashFS payload, draft spec (`AI\x03`).
    Type3,
}

impl Kind {
    /// Classify the trailing magic byte (`2` or `3`).
    pub fn from_u8(b: u8) -> Option<Kind> {
        match b {
            2 => Some(Kind::Type2),
            3 => Some(Kind::Type3),
            _ => None,
        }
    }
}

/// A detected AppImage.
#[derive(Clone, Debug)]
pub struct AppImage {
    /// Type-2 or type-3 image.
    pub kind: Kind,
    /// ELF64 header (`EI_CLASS` = 2).
    pub is64: bool,
    /// `e_machine` (0x3e x86-64, 0xb7 AArch64, 0x28 ARM, 0xf3 RISC-V).
    pub machine: u16,
    /// ELF header size actually present in the input.
    pub header_bytes: usize,
}

/// Detect an AppImage: ELF magic, `EI_CLASS`/`EI_DATA` sane, and the
/// `AI\x02`/`AI\x03` signature at `e_ident[8..11]`. Returns `None`
/// for ordinary ELF binaries and type-1 images (which carry no
/// `AI` stamp).
pub fn parse(d: &[u8]) -> Option<AppImage> {
    if d.get(..4)? != b"\x7fELF" {
        return None;
    }
    if d.get(6)? != &1 {
        return None; // EV_CURRENT
    }
    let kind = if d.get(8)? == &b'A' && d.get(9)? == &b'I' {
        Kind::from_u8(*d.get(10)?)?
    } else {
        return None;
    };
    let class = *d.get(4)?;
    let data = *d.get(5)?;
    let header_bytes = match (class, data) {
        (1, 1) | (2, 1) | (1, 2) | (2, 2) => 52,
        _ => return None,
    };
    Some(AppImage {
        kind,
        is64: class == 2,
        machine: if data == 1 {
            u16::from_le_bytes(d.get(18..20)?.try_into().ok()?)
        } else {
            ((*d.get(18)? as u16) << 8) | *d.get(19)? as u16
        },
        header_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut f = vec![0u8; 64];
        f[..4].copy_from_slice(b"\x7fELF");
        f[4] = 2;
        f[5] = 1;
        f[6] = 1;
        f[8..11].copy_from_slice(b"AI\x02");
        f[16..18].copy_from_slice(&2u16.to_le_bytes());
        f[18..20].copy_from_slice(&0x3eu16.to_le_bytes());
        f
    }

    #[test]
    fn type2_x86_64() {
        let a = parse(&fixture()).unwrap();
        assert_eq!(a.kind, Kind::Type2);
        assert!(a.is64);
        assert_eq!(a.machine, 0x3e);
        assert_eq!(a.header_bytes, 52);
    }

    #[test]
    fn type3_and_big_endian() {
        let mut f = fixture();
        f[10] = 3;
        f[5] = 2;
        f[18] = 0;
        f[19] = 0x28;
        let a = parse(&f).unwrap();
        assert_eq!(a.kind, Kind::Type3);
        assert_eq!(a.machine, 0x28);
        assert_eq!(Kind::from_u8(9), None);
    }

    #[test]
    fn rejects_plain_elf() {
        let mut f = fixture();
        f[8] = 0;
        assert!(parse(&f).is_none());
        assert!(parse(b"\x7fELF").is_none());
        let mut g = fixture();
        g[6] = 0;
        assert!(parse(&g).is_none());
    }
}

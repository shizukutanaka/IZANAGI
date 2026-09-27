//! NXP i.MX boot image — IVT (Image Vector Table) parsing.
//!
//! The IVT starts with `{tag u8 = 0xD1, len u16 BE = 0x0020, version
//! u8 = 0x40}` (i.MX6/7 V1), then `entry`, `reserved1`, `dcd`,
//! `boot_data`, `self`, `csf`, `reserved2` (all u32 LE at @4..32).
//! `self` points back at the IVT's load address; `entry` is the first
//! instruction address; `dcd`/`csf` are 0 when absent. `boot_data`
//! (when non-zero) references `{start, length, plugin}`.
//!
//! ```
//! use izanagi_kit::imx;
//! let mut d = vec![0u8; 32];
//! d[0] = 0xD1;                 // IVT tag
//! d[1..3].copy_from_slice(&0x0020u16.to_be_bytes()); // len
//! d[3] = 0x40;                 // version
//! d[4..8].copy_from_slice(&0x87800000u32.to_le_bytes()); // entry
//! d[20..24].copy_from_slice(&0x877FF420u32.to_le_bytes()); // self
//! let i = imx::parse(&d).unwrap();
//! assert_eq!(i.entry, 0x87800000);
//! ```

/// IVT tag byte.
pub const IVT_TAG: u8 = 0xD1;

/// IVT header size (`len` field value).
pub const IVT_LEN: u16 = 0x0020;

/// A parsed IVT.
#[derive(Clone, Debug, PartialEq)]
pub struct Ivt {
    /// IVT version nibble value (0x40 = i.MX6 V1, 0x41 = V2/V3 family).
    pub version: u8,
    /// Absolute address of the image entry point.
    pub entry: u32,
    /// Address of the DCD block (0 = none).
    pub dcd: u32,
    /// Address of the boot-data structure (0 = none).
    pub boot_data: u32,
    /// Address the IVT itself was loaded at.
    pub self_addr: u32,
    /// Address of the CSF (secure-boot) block (0 = none).
    pub csf: u32,
}

/// Boot-data structure pointed to by [`Ivt::boot_data`].
#[derive(Clone, Debug, PartialEq)]
pub struct BootData {
    /// Load address of the image start (the IVT).
    pub start: u32,
    /// Image length in bytes.
    pub length: u32,
    /// Plugin flag (0 = normal boot image).
    pub plugin: u32,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses the 32-byte IVT header at offset 0.
pub fn parse(d: &[u8]) -> Option<Ivt> {
    if d.len() < 32 {
        return None;
    }
    if *d.first()? != IVT_TAG {
        return None;
    }
    let len = ((*d.get(1)? as u16) << 8) | *d.get(2)? as u16;
    if len != IVT_LEN {
        return None;
    }
    let version = *d.get(3)?;
    if version & 0xF0 != 0x40 {
        return None;
    }
    Some(Ivt {
        version,
        entry: u32le(d, 4)?,
        dcd: u32le(d, 12)?,
        boot_data: u32le(d, 16)?,
        self_addr: u32le(d, 20)?,
        csf: u32le(d, 24)?,
    })
}

/// Decodes the boot-data block when `ivt.boot_data` maps to a file
/// offset supplied as `at` (IVT fields are load addresses, so the
/// caller supplies the corresponding file offset).
pub fn boot_data(d: &[u8], at: usize) -> Option<BootData> {
    Some(BootData {
        start: u32le(d, at)?,
        length: u32le(d, at + 4)?,
        plugin: u32le(d, at + 8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 64];
        d[0] = IVT_TAG;
        d[1..3].copy_from_slice(&IVT_LEN.to_be_bytes());
        d[3] = 0x40;
        d[4..8].copy_from_slice(&0x8780_0000u32.to_le_bytes());
        d[12..16].copy_from_slice(&0x0091_7400u32.to_le_bytes()); // dcd
        d[16..20].copy_from_slice(&0x877F_F400u32.to_le_bytes()); // boot_data
        d[20..24].copy_from_slice(&0x877F_F420u32.to_le_bytes()); // self
        d[24..28].copy_from_slice(&0x8770_0000u32.to_le_bytes()); // csf
                                                                  // boot data at offset 32
        d[32..36].copy_from_slice(&0x877F_F400u32.to_le_bytes());
        d[36..40].copy_from_slice(&0x0004_0000u32.to_le_bytes());
        d[40..44].copy_from_slice(&0u32.to_le_bytes());
        d
    }

    #[test]
    fn parses_ivt() {
        let d = fixture();
        let i = parse(&d).unwrap();
        assert_eq!(i.entry, 0x8780_0000);
        assert_eq!(i.self_addr, 0x877F_F420);
        assert_eq!(i.csf, 0x8770_0000);
        let bd = boot_data(&d, 32).unwrap();
        assert_eq!(bd.length, 0x40000);
        assert_eq!(bd.plugin, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 16]).is_none());
        let mut d = fixture();
        d[0] = 0xD2;
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[3] = 0x50; // bad version nibble
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[1..3].copy_from_slice(&0x0024u16.to_be_bytes()); // bad len
        assert!(parse(&d).is_none());
    }
}

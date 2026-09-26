//! Microsoft VHD (Virtual Hard Disk) footer.
//!
//! A VHD file ends in a 512-byte footer starting with the `"conectix"`
//! cookie. All fields are big-endian: features, file-format version,
//! data offset, creation timestamp, creator application/version/OS,
//! original and current sizes, CHS geometry, disk type, a one's-
//! complement checksum, a unique GUID and a saved-state flag.
//!
//! ```
//! use izanagi_kit::vhd::{parse, Kind, FOOTER};
//!
//! let mut d = vec![0u8; FOOTER];
//! d[..8].copy_from_slice(b"conectix");
//! let put32 = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = (v >> 24) as u8; d[at + 1] = (v >> 16) as u8;
//!     d[at + 2] = (v >> 8) as u8; d[at + 3] = v as u8;
//! };
//! put32(&mut d, 12, 0x0001_0000);   // file version
//! put32(&mut d, 60, 2);             // fixed disk
//! let mut sum = 0u32;
//! for (i, &b) in d.iter().enumerate() {
//!     if !(64..68).contains(&i) { sum = sum.wrapping_add(u32::from(b)); }
//! }
//! put32(&mut d, 64, !sum);
//! let v = parse(&d).unwrap();
//! assert_eq!(v.kind, Kind::Fixed);
//! assert!(v.checksum_ok(&d));
//! ```

/// Footer size in bytes.
pub const FOOTER: usize = 512;
/// Magic cookie.
pub const COOKIE: &[u8; 8] = b"conectix";

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

fn be64(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from(be32(d, at)?) << 32 | u64::from(be32(d, at + 4)?))
}

/// Virtual disk type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `0` — no disk.
    None,
    /// `2` — fixed-size disk.
    Fixed,
    /// `3` — dynamically expanding disk.
    Dynamic,
    /// `4` — differencing disk.
    Differencing,
    /// Any other type id.
    Other(u32),
}

impl Kind {
    fn of(v: u32) -> Self {
        match v {
            0 => Kind::None,
            2 => Kind::Fixed,
            3 => Kind::Dynamic,
            4 => Kind::Differencing,
            v => Kind::Other(v),
        }
    }
}

/// A parsed VHD footer.
#[derive(Clone, Debug, PartialEq)]
pub struct Vhd {
    /// Feature bits (`0` none, `1` temporary, `2` reserved).
    pub features: u32,
    /// File format version (`0x00010000` for v1.0).
    pub version: u32,
    /// Byte offset of dynamic/differencing data (`u64::MAX` on fixed).
    pub data_offset: u64,
    /// Creation timestamp (seconds since 2000-01-01 UTC).
    pub timestamp: u32,
    /// Creator application tag (e.g. `"vpc "`, `"win "`).
    pub creator_app: u32,
    /// Creator version.
    pub creator_ver: u32,
    /// Creator host OS (`0` Windows, `1` Macintosh).
    pub creator_os: u32,
    /// Original disk size in bytes.
    pub original_size: u64,
    /// Current disk size in bytes.
    pub current_size: u64,
    /// CHS geometry: cylinders.
    pub cylinders: u16,
    /// CHS geometry: heads.
    pub heads: u8,
    /// CHS geometry: sectors per track.
    pub sectors_per_track: u8,
    /// Disk type.
    pub kind: Kind,
    /// Stored checksum field.
    pub checksum: u32,
    /// Unique disk GUID (16 raw bytes).
    pub guid: [u8; 16],
    /// Nonzero when the VM state is saved inside the image.
    pub saved_state: u8,
}

impl Vhd {
    /// Recompute the one's-complement checksum over the 512-byte
    /// footer (with the checksum field treated as zero) and compare
    /// against the stored value.
    pub fn checksum_ok(&self, d: &[u8]) -> bool {
        let mut sum = 0u32;
        for (i, &b) in d.iter().take(FOOTER).enumerate() {
            if !(64..68).contains(&i) {
                sum = sum.wrapping_add(u32::from(b));
            }
        }
        self.checksum == !sum
    }
}

/// Parse a VHD footer (the last 512 bytes of a `.vhd` file). Returns
/// `None` on a bad cookie or a buffer shorter than 512 bytes.
pub fn parse(d: &[u8]) -> Option<Vhd> {
    if d.get(..8)? != COOKIE {
        return None;
    }
    let mut guid = [0u8; 16];
    guid.copy_from_slice(d.get(68..84)?);
    Some(Vhd {
        features: be32(d, 8)?,
        version: be32(d, 12)?,
        data_offset: be64(d, 16)?,
        timestamp: be32(d, 24)?,
        creator_app: be32(d, 28)?,
        creator_ver: be32(d, 32)?,
        creator_os: be32(d, 36)?,
        original_size: be64(d, 40)?,
        current_size: be64(d, 48)?,
        cylinders: u16::from(*d.get(56)?) << 8 | u16::from(*d.get(57)?),
        heads: *d.get(58)?,
        sectors_per_track: *d.get(59)?,
        kind: Kind::of(be32(d, 60)?),
        checksum: be32(d, 64)?,
        guid,
        saved_state: *d.get(84)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn put32(d: &mut [u8], at: usize, v: u32) {
        d[at] = (v >> 24) as u8;
        d[at + 1] = (v >> 16) as u8;
        d[at + 2] = (v >> 8) as u8;
        d[at + 3] = v as u8;
    }

    fn footer() -> Vec<u8> {
        let mut d = vec![0u8; FOOTER];
        d[..8].copy_from_slice(COOKIE);
        put32(&mut d, 8, 2); // features
        put32(&mut d, 12, 0x0001_0000);
        put32(&mut d, 16, 0xFFFF_FFFF);
        put32(&mut d, 20, 0xFFFF_FFFF); // data_offset = -1 (fixed)
        put32(&mut d, 28, 0x7669_7274); // "virt"
        put32(&mut d, 36, 0); // creator OS = Windows
        put32(&mut d, 40, 0x0000_0000);
        put32(&mut d, 44, 0x0040_0000); // orig 4 MiB
        put32(&mut d, 48, 0x0000_0000);
        put32(&mut d, 52, 0x0040_0000); // cur 4 MiB
        d[56] = 0;
        d[57] = 8; // cyl 8
        d[58] = 16; // heads
        d[59] = 63; // spt
        put32(&mut d, 60, 2); // fixed
        for i in 0..16 {
            d[68 + i] = i as u8;
        }
        d[84] = 0;
        let mut sum = 0u32;
        for (i, &b) in d.iter().enumerate() {
            if !(64..68).contains(&i) {
                sum = sum.wrapping_add(u32::from(b));
            }
        }
        put32(&mut d, 64, !sum);
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let d = footer();
        let v = parse(&d).unwrap();
        assert_eq!(v.features, 2);
        assert_eq!(v.version, 0x0001_0000);
        assert_eq!(v.data_offset, u64::MAX);
        assert_eq!(v.creator_app, 0x7669_7274);
        assert_eq!(v.creator_os, 0);
        assert_eq!(v.original_size, 0x40_0000);
        assert_eq!(v.current_size, 0x40_0000);
        assert_eq!(v.cylinders, 8);
        assert_eq!(v.heads, 16);
        assert_eq!(v.sectors_per_track, 63);
        assert_eq!(v.kind, Kind::Fixed);
        assert_eq!(v.guid[15], 15);
        assert_eq!(v.saved_state, 0);
        assert!(v.checksum_ok(&d));
    }

    #[test]
    fn checksum_detects_corruption() {
        let mut d = footer();
        let v = parse(&d).unwrap();
        d[57] = 9;
        assert!(!v.checksum_ok(&d));
    }

    #[test]
    fn kind_ids() {
        assert_eq!(Kind::of(0), Kind::None);
        assert_eq!(Kind::of(3), Kind::Dynamic);
        assert_eq!(Kind::of(4), Kind::Differencing);
        assert_eq!(Kind::of(9), Kind::Other(9));
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0u8; 511]), None);
        let mut d = footer();
        d[0] = b'x';
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn constants() {
        assert_eq!(FOOTER, 512);
        assert_eq!(COOKIE, b"conectix");
    }
}

//! Intel Flash Descriptor (ICH/PCH IFD).
//!
//! The descriptor region lives in the first 4 KiB of the flash
//! part. Signature `0x0FF0A55A` at `0x10`, then the `FLMAP`
//! registers: `FLMAP0` packs the component and region base
//! addresses (in 16-byte units), `FLMAP1` the master base and PCH
//! soft-strap base. Each `FLREGx` entry in the region section is a
//! u32 whose low 15 bits give a 4 KiB-granularity base and bits
//! 30:16 the inclusive limit; a region is enabled when
//! `limit >= base`.
//!
//! ```
//! use izanagi_kit::ifd::{parse, SIGNATURE};
//!
//! let mut d = vec![0u8; 4096];
//! let put = |d: &mut [u8], at: usize, v: u32| {
//!     d[at] = v as u8; d[at + 1] = (v >> 8) as u8;
//!     d[at + 2] = (v >> 16) as u8; d[at + 3] = (v >> 24) as u8;
//! };
//! put(&mut d, 0x10, SIGNATURE);
//! put(&mut d, 0x14, 0x0004_0100); // frba=4 -> regions at 0x40
//! put(&mut d, 0x40, 0x01ff_0000); // region0: base 0, limit 0x1ff
//! let f = parse(&d).unwrap();
//! let r = f.regions[0].unwrap();
//! assert_eq!(r.base(), 0);
//! assert_eq!(r.limit(), 0x001f_ffff);
//! assert!(r.enabled());
//! ```

/// Descriptor signature (u32 LE at offset 0x10).
pub const SIGNATURE: u32 = 0x0ff0_a55a;
/// Number of region registers.
pub const REGIONS: usize = 16;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// One FLREG region entry (4 KiB granularity).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    /// Raw register value.
    pub raw: u32,
}

impl Region {
    /// Base in 4 KiB units.
    pub fn base4k(&self) -> u32 {
        self.raw & 0x7fff
    }
    /// Limit in 4 KiB units.
    pub fn limit4k(&self) -> u32 {
        (self.raw >> 16) & 0x7fff
    }
    /// True when the region is populated.
    pub fn enabled(&self) -> bool {
        self.limit4k() >= self.base4k()
    }
    /// First byte of the region.
    pub fn base(&self) -> u64 {
        u64::from(self.base4k()) * 4096
    }
    /// Last byte of the region (inclusive).
    pub fn limit(&self) -> u64 {
        u64::from(self.limit4k()) * 4096 + 4095
    }
}

/// A parsed flash descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ifd {
    /// Component section base (bytes; `FCBA * 16`).
    pub component_at: u32,
    /// Component count minus 1 (`NC`).
    pub nc: u8,
    /// Region section base (bytes; `FRBA * 16`).
    pub region_at: u32,
    /// Region count minus 1 (`NR`).
    pub nr: u8,
    /// Master section base (bytes; `FMBA * 16`).
    pub master_at: u32,
    /// Master count minus 1 (`NM`).
    pub nm: u8,
    /// PCH soft-strap base (bytes; `FPSBA * 16`).
    pub strap_at: u32,
    /// The 16 region registers; `None` entries are past the
    /// declared count or out of bounds.
    pub regions: [Option<Region>; REGIONS],
}

/// Well-known region indices.
pub const REGION_DESCRIPTOR: usize = 0;
/// BIOS region index.
pub const REGION_BIOS: usize = 1;
/// Intel ME region index.
pub const REGION_ME: usize = 2;
/// Gigabit Ethernet region index.
pub const REGION_GBE: usize = 3;
/// Platform data region index.
pub const REGION_PDR: usize = 4;

/// Parse a descriptor. Returns `None` when the signature is
/// missing or the region section falls outside the buffer.
pub fn parse(d: &[u8]) -> Option<Ifd> {
    if le32(d, 0x10)? != SIGNATURE {
        return None;
    }
    let flmap0 = le32(d, 0x14)?;
    let flmap1 = le32(d, 0x18)?;
    let region_at = ((flmap0 >> 16) & 0xff) * 16;
    let nr = ((flmap0 >> 24) & 0xff) as usize;
    let count = (nr + 1).min(REGIONS);
    let mut regions = [None; REGIONS];
    for (i, slot) in regions.iter_mut().enumerate().take(count) {
        let raw = le32(d, usize::try_from(region_at).ok()?.checked_add(i * 4)?)?;
        *slot = Some(Region { raw });
    }
    Some(Ifd {
        component_at: (flmap0 & 0xff) * 16,
        nc: ((flmap0 >> 8) & 0xff) as u8,
        region_at,
        nr: ((flmap0 >> 24) & 0xff) as u8,
        master_at: (flmap1 & 0xff) * 16,
        nm: ((flmap1 >> 8) & 0xff) as u8,
        strap_at: ((flmap1 >> 16) & 0xff) * 16,
        regions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 8192];
        let w = |d: &mut [u8], o: usize, v: u32| {
            d[o] = v as u8;
            d[o + 1] = (v >> 8) as u8;
            d[o + 2] = (v >> 16) as u8;
            d[o + 3] = (v >> 24) as u8;
        };
        w(&mut d, 0x10, SIGNATURE);
        w(&mut d, 0x14, 0x0204_0100); // fcba=0x00, nc=1, frba=4, nr=2
        w(&mut d, 0x18, 0x0030_0201); // fmba=1, nm=2, fpsba=0x30
                                      // regions at 0x40
        w(&mut d, 0x40, 0x07ff_0000); // r0: base0 lim0x7ff enabled
        w(&mut d, 0x44, 0x0000_0fff); // r1: base0xfff lim0 disabled
        w(&mut d, 0x48, 0x0fff_0008); // r2: base8 lim0xfff enabled
        d
    }

    #[test]
    fn fields_and_regions() {
        let d = fixture();
        let f = parse(&d).unwrap();
        assert_eq!(f.component_at, 0);
        assert_eq!(f.nc, 1);
        assert_eq!(f.region_at, 0x40);
        assert_eq!(f.nr, 2);
        assert_eq!(f.master_at, 16);
        assert_eq!(f.nm, 2);
        assert_eq!(f.strap_at, 0x300);
        let r0 = f.regions[0].unwrap();
        assert_eq!(r0.base4k(), 0);
        assert_eq!(r0.limit4k(), 0x7ff);
        assert!(r0.enabled());
        assert_eq!(r0.base(), 0);
        assert_eq!(r0.limit(), 0x007f_ffff);
        let r1 = f.regions[1].unwrap();
        assert!(!r1.enabled());
        let r2 = f.regions[2].unwrap();
        assert_eq!(r2.base(), 8 * 4096);
        assert_eq!(f.regions[3], None); // nr+1 = 3 entries
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 32]).is_none());
        let mut d = fixture();
        d[0x10] = 0;
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        // frba=0xff -> region section at 0xff0; truncate so reads run out
        d2[0x16] = 0xff;
        d2.truncate(4082);
        assert!(parse(&d2).is_none());
    }
}

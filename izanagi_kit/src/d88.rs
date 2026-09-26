//! NEC PC-88/PC-98 disk image (`.d88`).
//!
//! Header: `name[16] | reserved[9] | protect u8 | disk_type u8 |
//! image_size u32 | track_offsets u32 × 164`. Each non-zero track
//! offset points at a sequence of sector headers
//! (`C H R N | count u16 | density | deleted | status | reserved[5] |
//! data_size u16`, then `data_size` bytes). `N` is the log2 density
//! class (`0`=128 B … `3`=1024 B).
//!
//! ```
//! use izanagi_kit::d88::{parse, SECTOR_256};
//!
//! // Header + one track with a single 256-byte sector.
//! let mut d = vec![0u8; 688 + 16 + 256];
//! d[0..3].copy_from_slice(b"DSK");
//! d[26] = 0x20;                            // 2HD
//! let n = d.len() as u32;
//! d[28..32].copy_from_slice(&n.to_le_bytes());
//! d[32..36].copy_from_slice(&688u32.to_le_bytes()); // track 0 ptr
//! let s = 688;
//! d[s] = 0; d[s + 1] = 0; d[s + 2] = 1; d[s + 3] = 1; // C H R N
//! d[s + 4..s + 6].copy_from_slice(&1u16.to_le_bytes()); // count
//! d[s + 14..s + 16].copy_from_slice(&256u16.to_le_bytes());
//! let f = parse(&d).unwrap();
//! assert_eq!(f.name, "DSK");
//! let sec = f.sectors(&d, 0).next().unwrap();
//! assert_eq!(sec.data_len, SECTOR_256);
//! ```

use std::string::String;

/// Track pointer table size: 164 entries (80 cyl × 2 + spares).
pub const TRACKS: usize = 164;
/// Header size: name(16) + reserved(9) + protect + type + size(4)
/// + 164 × u32 pointers.
pub const HEADER: usize = 688;
/// Sector bytes for `N == 0`.
pub const SECTOR_128: usize = 128;
/// Sector bytes for `N == 1`.
pub const SECTOR_256: usize = 256;

/// One sector's parsed header plus its data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sector {
    /// Cylinder / head / record / size-class bytes.
    pub c: u8,
    /// Head.
    pub h: u8,
    /// Record (sector number).
    pub r: u8,
    /// Size class N (`128 << n` bytes nominal).
    pub n: u8,
    /// Sector count on this track (same for every header in it).
    pub count: u16,
    /// Density byte (0x00 double, 0x40 single, 0x01 high).
    pub density: u8,
    /// Deleted-data flag.
    pub deleted: u8,
    /// FDC status byte (0 = normal).
    pub status: u8,
    /// The sector payload (`data_size` bytes).
    pub data_at: usize,
    /// Payload length.
    pub data_len: usize,
}

/// A parsed D88 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D88 {
    /// Image name (trimmed).
    pub name: String,
    /// Write-protect flag.
    pub protect: u8,
    /// Disk type byte (0x00 2D, 0x10 2DD, 0x20 2HD, 0x30 1D, 0x40 1DD, 0x50 2HS).
    pub disk_type: u8,
    /// Declared image size.
    pub image_size: u32,
    /// Track offsets (0 = absent track).
    pub tracks: [u32; TRACKS],
}

impl D88 {
    /// Iterate the sector headers stored at track `i`.
    pub fn sectors<'a>(&self, d: &'a [u8], i: usize) -> Sectors<'a> {
        let at = self.tracks.get(i).copied().unwrap_or(0) as usize;
        Sectors {
            d,
            at,
            left: 0,
            started: false,
        }
    }
}

/// Iterator over one track's sectors.
pub struct Sectors<'a> {
    d: &'a [u8],
    at: usize,
    left: u16,
    started: bool,
}

impl<'a> Sectors<'a> {
    /// Slice of the last sector's data, or a specific sector's data.
    pub fn data(&self, s: &Sector) -> &'a [u8] {
        &self.d[s.data_at..s.data_at + s.data_len]
    }
}

impl<'a> Iterator for Sectors<'a> {
    type Item = Sector;
    fn next(&mut self) -> Option<Sector> {
        if self.at == 0 {
            return None;
        }
        if self.left == 0 {
            if self.started {
                return None;
            }
            let h = self.d.get(self.at..self.at + 16)?;
            self.left = u16::from(h[4]) | u16::from(h[5]) << 8;
            self.started = true;
            if self.left == 0 {
                return None;
            }
        }
        let h = self.d.get(self.at..self.at + 16)?;
        let len = usize::from(u16::from(h[14]) | u16::from(h[15]) << 8);
        let data_at = self.at + 16;
        let data_len = if len == 0 {
            SECTOR_128 << usize::from(h[3])
        } else {
            len
        };
        self.d.get(data_at..data_at + data_len)?;
        let s = Sector {
            c: h[0],
            h: h[1],
            r: h[2],
            n: h[3],
            count: self.left,
            density: h[6],
            deleted: h[7],
            status: h[8],
            data_at,
            data_len,
        };
        self.at = data_at + data_len;
        self.left -= 1;
        Some(s)
    }
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// Parse a D88 header, or `None` when the file is too short, the
/// declared size disagrees, or a track pointer is out of range.
pub fn parse(d: &[u8]) -> Option<D88> {
    if d.len() < HEADER {
        return None;
    }
    let mut name = [0u8; 16];
    name.copy_from_slice(&d[0..16]);
    let end = name.iter().position(|&b| b == 0).unwrap_or(16);
    let name = String::from_utf8_lossy(&name[..end]).trim_end().to_string();
    let image_size = u32le(d, 28)?;
    let mut tracks = [0u32; TRACKS];
    for (i, t) in tracks.iter_mut().enumerate() {
        let v = u32le(d, 32 + i * 4)?;
        if v != 0 && usize::try_from(v).is_ok_and(|n| n >= d.len()) {
            return None;
        }
        *t = v;
    }
    Some(D88 {
        name,
        protect: d[25],
        disk_type: d[26],
        image_size,
        tracks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; HEADER];
        d[0..4].copy_from_slice(b"TEST");
        d[26] = 0x20;
        // track 0: two 128B sectors
        let s = HEADER;
        d.resize(HEADER + 2 * (16 + 128), 0);
        let n = d.len() as u32;
        d[28..32].copy_from_slice(&n.to_le_bytes());
        d[32..36].copy_from_slice(&(HEADER as u32).to_le_bytes());
        for i in 0..2usize {
            let h = s + i * (16 + 128);
            d[h] = 0;
            d[h + 1] = 0;
            d[h + 2] = (i + 1) as u8;
            d[h + 3] = 0;
            d[h + 4..h + 6].copy_from_slice(&2u16.to_le_bytes());
            d[h + 14..h + 16].copy_from_slice(&128u16.to_le_bytes());
            d[h + 16] = 0x60 + i as u8;
        }
        d
    }

    #[test]
    fn fields() {
        let d = image();
        let f = parse(&d).unwrap();
        assert_eq!(f.name, "TEST");
        assert_eq!(f.disk_type, 0x20);
        assert_eq!(f.image_size as usize, d.len());
        assert_eq!(f.tracks[0] as usize, HEADER);
        assert_eq!(f.tracks[1], 0);
    }

    #[test]
    fn sector_walk() {
        let d = image();
        let f = parse(&d).unwrap();
        let secs: Vec<_> = f.sectors(&d, 0).collect();
        assert_eq!(secs.len(), 2);
        assert_eq!(secs[0].r, 1);
        assert_eq!(secs[1].r, 2);
        assert_eq!(secs[0].data_len, SECTOR_128);
        let s = f.sectors(&d, 0);
        let first: Vec<_> = s.map(|x| (x.r, x.status)).collect();
        assert_eq!(first, [(1, 0), (2, 0)]);
        // absent track yields empty
        assert!(f.sectors(&d, 1).next().is_none());
    }

    #[test]
    fn sector_data_slice() {
        let d = image();
        let f = parse(&d).unwrap();
        let mut it = f.sectors(&d, 0);
        let sec = it.next().unwrap();
        assert_eq!(it.data(&sec)[0], 0x60);
        let sec = it.next().unwrap();
        assert_eq!(it.data(&sec)[0], 0x61);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&vec![0u8; HEADER - 1]), None);
        let mut d = image();
        // track pointer past end
        d[32..36].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse(&d), None);
    }
}

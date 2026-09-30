//! SCP (SuperCard Pro flux image): `SCP` magic +
//! version, disk type, revolution count, track range,
//! flags, bit-cell width, heads, resolution, `u32le`
//! data checksum, then a 168-entry `u32le` track
//! offset table.
//!
//! ```
//! let mut d = b"SCP\x25\x80".to_vec(); // v2.5, C64 disk
//! d.extend_from_slice(&[2, 0, 39]); // revs, start, end
//! d.extend_from_slice(&[0, 0, 0, 0]); // flags, cell, heads, res
//! d.extend_from_slice(&[0; 4]); // checksum
//! d.extend_from_slice(&[0; 168 * 4]); // offset table
//! let p = izanagi_kit::scp::parse(&d).unwrap();
//! assert_eq!(p.version, 0x25);
//! assert_eq!(p.end_track, 39);
//! assert!(izanagi_kit::scp::detect(&d));
//! ```

/// Census of a SuperCard Pro image.
#[derive(Debug, Clone, PartialEq)]
pub struct Scp {
    /// File version byte (BCD-ish, e.g. `0x25` = v2.5).
    pub version: u8,
    /// Disk type byte (upper nibble manufacturer).
    pub disk_type: u8,
    /// Number of revolutions captured per track.
    pub revolutions: u8,
    /// First track index.
    pub start_track: u8,
    /// Last track index.
    pub end_track: u8,
    /// Flags byte.
    pub flags: u8,
    /// Bit cell size byte.
    pub cell_size: u8,
    /// Heads byte (0 = both, 1 = bottom, 2 = top).
    pub heads: u8,
    /// Resolution byte (25 ns units).
    pub resolution: u8,
    /// `u32le` checksum covering the data area.
    pub checksum: u32,
    /// Track offset table entries that point at data.
    pub track_offsets: u32,
    /// Sum of `u32le` offset-table payload span after
    /// the 16-byte header + 672-byte table region.
    pub data_len: u32,
    /// Offset table ran past the end of the buffer.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on `SCP` with a plausible header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 16
        && b[0] == b'S'
        && b[1] == b'C'
        && b[2] == b'P'
        && b[3] != 0
        && b[5] >= 1
        && b[5] <= 5
        && b[6] <= b[7]
        && b[7] <= 167
        && b[10] <= 2
}

/// Census; `None` without the SCP magic.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Scp> {
    if !detect(b) {
        return None;
    }
    let mut s = Scp {
        version: b[3],
        disk_type: b[4],
        revolutions: b[5],
        start_track: b[6],
        end_track: b[7],
        flags: b[8],
        cell_size: b[9],
        heads: b[10],
        resolution: b[11],
        checksum: le32(b, 12),
        track_offsets: 0,
        data_len: 0,
        truncated: false,
    };
    let table = 16 + 168 * 4;
    if b.len() < table {
        s.truncated = true;
        let mut i = 16;
        while i + 4 <= b.len() {
            if le32(b, i) != 0 {
                s.track_offsets += 1;
            }
            i += 4;
        }
    } else {
        for t in 0..168usize {
            if le32(b, 16 + t * 4) != 0 {
                s.track_offsets += 1;
            }
        }
    }
    s.data_len = b.len().saturating_sub(table) as u32;
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"SCP\x09\x00".to_vec(); // v0.9, C64
        d.extend_from_slice(&[3, 0, 83]); // 3 revs, tracks 0..83
        d.extend_from_slice(&[1, 0, 0, 0]); // flags, cell, heads, res
        d.extend_from_slice(&[0x34, 0x12, 0, 0]); // checksum
        let mut offs = vec![0u8; 168 * 4];
        offs[0] = 0xB8;
        offs[1] = 0x02;
        offs[4] = 0xB8;
        offs[5] = 0x05;
        d.extend_from_slice(&offs);
        d.extend_from_slice(&[0; 32]); // track data area
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"scp file"));
        assert!(!detect(b"SCP\x00"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, 0x09);
        assert_eq!(p.revolutions, 3);
        assert_eq!(p.start_track, 0);
        assert_eq!(p.end_track, 83);
        assert_eq!(p.checksum, 0x1234);
        assert_eq!(p.track_offsets, 2);
        assert_eq!(p.data_len, 32);
        assert!(!p.truncated);
    }

    #[test]
    fn short_table() {
        let mut d = b"SCP\x09\x00".to_vec();
        d.extend_from_slice(&[1, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0]);
        let p = parse(&d).unwrap();
        assert!(p.truncated);
    }
}

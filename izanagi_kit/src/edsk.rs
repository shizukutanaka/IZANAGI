//! EDSK/DSK (Amstrad CPC disk image): `MV - CPCEMU
//! Disk-File\r\n` (standard) or `EXTENDED CPC DSK
//! File\r\n` magic, 256-byte header (creator, tracks,
//! sides, track size / per-track size table), then
//! `Track-Info\r\n` blocks with an 8-byte-per-sector
//! information list.
//!
//! ```
//! let mut d = b"MV - CPCEMU Disk-File\r\n".to_vec();
//! d.resize(34, 0); // pad sig area
//! d.extend_from_slice(b"IZANAGI       "); // creator 14B
//! d.extend_from_slice(&[1, 1]); // tracks, sides
//! d.extend_from_slice(&[0x00, 0x13]); // track size 0x1300
//! d.resize(256, 0);
//! d.extend_from_slice(b"Track-Info\r\n");
//! d.extend_from_slice(&[0; 4]);
//! d.extend_from_slice(&[0, 0, 0, 0]); // track, side
//! d.extend_from_slice(&[2, 1, 0x4D, 0xE5]); // bps, nsect, gap, fill
//! d.extend_from_slice(&[0, 0, 0xC1, 2, 0, 0, 0, 0]); // sector info
//! let p = izanagi_kit::edsk::parse(&d).unwrap();
//! assert!(!p.extended);
//! assert_eq!(p.track_blocks, 1);
//! assert_eq!(p.sectors, 1);
//! assert!(izanagi_kit::edsk::detect(&d));
//! ```

/// Census of a CPC DSK/EDSK image.
#[derive(Debug, Clone, PartialEq)]
pub struct Edsk {
    /// `EXTENDED` vs plain `MV - CPCEMU` signature.
    pub extended: bool,
    /// Track count byte at offset 48.
    pub tracks: u8,
    /// Side count byte at offset 49.
    pub sides: u8,
    /// Standard-format uniform track size (`u16le`).
    pub track_size: u16,
    /// Track-size-table bytes used (extended only).
    pub size_table_len: u32,
    /// `Track-Info` blocks found past the header.
    pub track_blocks: u32,
    /// Sector information entries across blocks.
    pub sectors: u32,
    /// Largest sector size code (`128 << n`).
    pub max_sector_size: u32,
    /// A `Track-Info` block ran past the buffer.
    pub truncated: bool,
    /// Bytes after the 256-byte header.
    pub payload_len: u32,
}

const STD: &[u8] = b"MV - CPCEMU Disk-File";
const EXT: &[u8] = b"EXTENDED CPC DSK File";
const TIB: &[u8] = b"Track-Info\r\n";

/// `true` on a CPC signature with sane geometry.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 50
        && (b.starts_with(STD) || b.starts_with(EXT))
        && b[48] >= 1
        && b[48] <= 99
        && (b[49] == 1 || b[49] == 2)
}

/// Census; `None` without the DSK signature.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Edsk> {
    if !detect(b) {
        return None;
    }
    let extended = b.starts_with(EXT);
    let mut e = Edsk {
        extended,
        tracks: b[48],
        sides: b[49],
        track_size: b[50] as u16 | ((b[51] as u16) << 8),
        size_table_len: 0,
        track_blocks: 0,
        sectors: 0,
        max_sector_size: 0,
        truncated: false,
        payload_len: b.len().saturating_sub(256) as u32,
    };
    if extended {
        e.size_table_len = e.tracks as u32 * e.sides as u32;
    }
    // Walk Track-Info blocks: each has a 24-byte header
    // (12-byte signature + fields) followed by nsect * 8
    // bytes of sector information.
    let mut i = 256usize;
    while i + TIB.len() <= b.len() {
        match b[i..].windows(TIB.len()).position(|w| w == TIB) {
            Some(o) => {
                let t = i + o;
                if t + 24 > b.len() {
                    e.truncated = true;
                    break;
                }
                e.track_blocks += 1;
                let nsect = b[t + 21] as usize;
                if t + 24 + nsect * 8 > b.len() {
                    e.truncated = true;
                    break;
                }
                e.sectors += nsect as u32;
                for s in 0..nsect {
                    let code = b[t + 24 + s * 8 + 3] & 7;
                    let sz = 128u32 << code;
                    if sz > e.max_sector_size {
                        e.max_sector_size = sz;
                    }
                }
                i = t + 12;
            }
            None => break,
        }
    }
    Some(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"EXTENDED CPC DSK File\r\n".to_vec();
        d.resize(34, 0);
        d.extend_from_slice(b"CREATOR       ");
        d.extend_from_slice(&[2, 2]); // 2 tracks, 2 sides
        d.extend_from_slice(&[0, 0]);
        // extended size table: 4 tracks * 0x13 * 256
        d.extend_from_slice(&[0x13, 0x13, 0x13, 0x13]);
        d.resize(256, 0);
        d.extend_from_slice(b"Track-Info\r\n");
        d.extend_from_slice(&[0; 4]);
        d.extend_from_slice(&[0, 0, 0, 0]);
        d.extend_from_slice(&[2, 2, 0x4D, 0xE5]);
        d.extend_from_slice(&[0, 0, 1, 2, 0, 0, 0, 0]);
        d.extend_from_slice(&[0, 0, 2, 2, 0, 0, 0, 0]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"MV - OTHER Disk-File\r\n"));
        assert!(!detect(&fixture()[..40]));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert!(p.extended);
        assert_eq!(p.tracks, 2);
        assert_eq!(p.sides, 2);
        assert_eq!(p.size_table_len, 4);
        assert_eq!(p.track_blocks, 1);
        assert_eq!(p.sectors, 2);
        assert_eq!(p.max_sector_size, 512);
        assert!(!p.truncated);
    }

    #[test]
    fn tib_past_eof() {
        let mut d = b"MV - CPCEMU Disk-File\r\n".to_vec();
        d.resize(34, 0);
        d.extend_from_slice(b"X             ");
        d.extend_from_slice(&[1, 1, 0, 0]);
        d.resize(256, 0);
        d.extend_from_slice(b"Track-Info\r\n");
        let p = parse(&d).unwrap();
        assert!(p.truncated);
        assert_eq!(p.track_blocks, 0);
    }
}

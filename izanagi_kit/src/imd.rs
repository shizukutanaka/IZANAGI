//! IMD (ImageDisk floppy image): ASCII comment header
//! `IMD ` terminated by `0x1A`, then track records —
//! mode `0..=5`, cylinder, head byte (bit6 = sector
//! cylinder map, bit5 = sector head map), sector
//! count, size code (`128 << n`), sector maps, and
//! per-sector data records (`0` = unavailable, `1..=8`
//! = data follows).
//!
//! ```
//! let mut d = b"IMD comment".to_vec();
//! d.push(0x1A);
//! d.extend_from_slice(&[0, 0, 0, 1, 0]); // mode, cyl, head, nsec, 128B
//! d.extend_from_slice(&[1]); // sector numbering map
//! d.extend_from_slice(&[1]); // sector type 1 = data follows
//! d.extend_from_slice(&[0xAA; 128]);
//! let p = izanagi_kit::imd::parse(&d).unwrap();
//! assert_eq!(p.tracks, 1);
//! assert_eq!(p.sectors_total, 1);
//! assert!(izanagi_kit::imd::detect(&d));
//! ```

/// Census of an ImageDisk image.
#[derive(Debug, Clone, PartialEq)]
pub struct Imd {
    /// Header bytes up to and including the `0x1A`
    /// terminator.
    pub header_len: u32,
    /// Track records walked.
    pub tracks: u32,
    /// Largest cylinder index seen.
    pub cylinders_max: u32,
    /// Largest head value seen (`head & 0x01`).
    pub heads_max: u32,
    /// Sector records across all tracks.
    pub sectors_total: u32,
    /// Sector data payloads with a nonzero type.
    pub data_sectors: u32,
    /// Distinct transfer modes seen (max mode + 1).
    pub modes_max: u32,
    /// Tracks that had a cylinder map.
    pub cyl_maps: u32,
    /// Tracks that had a head map.
    pub head_maps: u32,
    /// A record overran the end of the buffer.
    pub truncated: bool,
}

/// `true` on `IMD ` with a `0x1A`-terminated comment
/// and room for one track header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 10 || b[..4] != *b"IMD " {
        return false;
    }
    match b[4..].iter().position(|&c| c == 0x1A) {
        Some(i) => {
            let t = 4 + i + 1;
            // Track header needs 5 bytes; empty payload allowed.
            t <= b.len()
        }
        None => false,
    }
}

/// Census; `None` without the `IMD ` header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Imd> {
    if !detect(b) {
        return None;
    }
    let h = 4 + b[4..].iter().position(|&c| c == 0x1A)? + 1;
    let mut m = Imd {
        header_len: h as u32,
        tracks: 0,
        cylinders_max: 0,
        heads_max: 0,
        sectors_total: 0,
        data_sectors: 0,
        modes_max: 0,
        cyl_maps: 0,
        head_maps: 0,
        truncated: false,
    };
    let mut i = h;
    while i + 5 <= b.len() {
        let mode = b[i];
        if mode > 5 {
            break; // padding / tail
        }
        let cyl = b[i + 1] as u32;
        let hb = b[i + 2];
        let head = (hb & 0x01) as u32;
        let nsec = b[i + 3] as usize;
        let ssize = b[i + 4] & 0x07;
        if nsec == 0 {
            break;
        }
        i += 5;
        // Sector numbering map always present.
        if i + nsec > b.len() {
            m.truncated = true;
            break;
        }
        i += nsec;
        if hb & 0x40 != 0 {
            if i + nsec > b.len() {
                m.truncated = true;
                break;
            }
            i += nsec;
            m.cyl_maps += 1;
        }
        if hb & 0x20 != 0 {
            if i + nsec > b.len() {
                m.truncated = true;
                break;
            }
            i += nsec;
            m.head_maps += 1;
        }
        // Per-sector data records.
        let slen = 128usize << ssize;
        let mut ok = true;
        for _ in 0..nsec {
            if i >= b.len() {
                m.truncated = true;
                ok = false;
                break;
            }
            let ty = b[i];
            i += 1;
            if ty == 0 {
                continue;
            }
            if ty > 8 || i + slen > b.len() {
                m.truncated = true;
                ok = false;
                break;
            }
            i += slen;
            m.data_sectors += 1;
        }
        if !ok {
            break;
        }
        m.tracks += 1;
        if cyl > m.cylinders_max {
            m.cylinders_max = cyl;
        }
        if head > m.heads_max {
            m.heads_max = head;
        }
        if mode as u32 > m.modes_max {
            m.modes_max = mode as u32;
        }
        m.sectors_total += nsec as u32;
    }
    if i < b.len() && !m.truncated {
        // Trailing bytes that are not a full track header.
        m.truncated = m.tracks > 0 && b.len() - i >= 5;
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"IMD comment text".to_vec();
        d.push(0x1A);
        // track 0: mode 0, cyl 0, head 0, 1 sector of 128B
        d.extend_from_slice(&[0, 0, 0, 1, 0, 1, 1]);
        d.extend_from_slice(&[0x55; 128]);
        // track 1: mode 0, cyl 1, head 1|0x60 maps, 2 sectors
        d.extend_from_slice(&[0, 1, 0x61, 2, 1, 1, 2]);
        d.extend_from_slice(&[0, 0]); // cyl map
        d.extend_from_slice(&[1, 1]); // head map
        d.extend_from_slice(&[1]);
        d.extend_from_slice(&[0x11; 256]);
        d.extend_from_slice(&[1]);
        d.extend_from_slice(&[0x22; 256]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"IMD no terminator"));
        assert!(!detect(b"XMD \x1A"));
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.tracks, 2);
        assert_eq!(p.cylinders_max, 1);
        assert_eq!(p.heads_max, 1);
        assert_eq!(p.sectors_total, 3);
        assert_eq!(p.data_sectors, 3);
        assert_eq!(p.cyl_maps, 1);
        assert_eq!(p.head_maps, 1);
        assert!(!p.truncated);
    }

    #[test]
    fn truncated_sector() {
        let mut d = b"IMD c".to_vec();
        d.push(0x1A);
        d.extend_from_slice(&[0, 0, 0, 1, 2, 1, 1]);
        d.extend_from_slice(&[0; 10]); // only 10 of 512 bytes
        let p = parse(&d).unwrap();
        assert!(p.truncated);
        assert_eq!(p.tracks, 0);
    }
}

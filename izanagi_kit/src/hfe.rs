//! HFE (HxC Floppy Emulator image): `HXCPICFE` magic +
//! revision `0`, track/side counts, encoding, bitrate,
//! rpm, interface mode, and a 512-byte-unit track
//! lookup table (offset + length `u16le` pairs).
//!
//! ```
//! let mut d = b"HXCPICFE".to_vec();
//! d.extend_from_slice(&[0, 2, 2, 0]); // rev, tracks, sides, enc
//! d.extend_from_slice(&[0xC0, 0x01]); // bitrate 448
//! d.extend_from_slice(&[0x2C, 0x01]); // rpm 300
//! d.extend_from_slice(&[7, 0]); // interface, dnu
//! d.extend_from_slice(&[1, 0]); // lut at 512
//! d.extend_from_slice(&[1, 1]); // write allowed, single step
//! d.resize(512, 0);
//! d.extend_from_slice(&[2, 0, 0x10, 0]); // track0: off 1024, len 0x10
//! d.extend_from_slice(&[0; 4]); // track1 empty pair
//! let p = izanagi_kit::hfe::parse(&d).unwrap();
//! assert_eq!(p.tracks, 2);
//! assert_eq!(p.bitrate, 448);
//! assert_eq!(p.lut_entries, 1);
//! assert!(izanagi_kit::hfe::detect(&d));
//! ```

/// Census of an HFE image.
#[derive(Debug, Clone, PartialEq)]
pub struct Hfe {
    /// Number of tracks.
    pub tracks: u8,
    /// Number of sides (1 or 2).
    pub sides: u8,
    /// Track encoding byte.
    pub encoding: u8,
    /// Bitrate in kbit/s (`u16le`).
    pub bitrate: u16,
    /// Floppy rpm (`u16le`).
    pub rpm: u16,
    /// Interface mode byte.
    pub interface_mode: u8,
    /// Track lookup table offset in 512-byte units.
    pub track_list_offset: u16,
    /// Write-allowed flag.
    pub write_allowed: u8,
    /// Single-step flag.
    pub single_step: u8,
    /// Lookup-table pairs with a nonzero offset.
    pub lut_entries: u32,
    /// Largest track-data byte offset (`offset * 512`).
    pub max_track_offset: u32,
    /// Bytes past the 512-byte header block (track data).
    pub payload_len: u32,
    /// Lookup table ran past the end of the buffer.
    pub truncated: bool,
}

fn le16(b: &[u8], i: usize) -> u16 {
    b[i] as u16 | ((b[i + 1] as u16) << 8)
}

/// `true` on `HXCPICFE` revision 0 with sane geometry.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 20 && b[..8] == *b"HXCPICFE" && b[8] == 0 && b[9] > 0 && (b[10] == 1 || b[10] == 2)
}

/// Census; `None` without the HFE magic.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Hfe> {
    if !detect(b) {
        return None;
    }
    let lo = le16(b, 18);
    let mut h = Hfe {
        tracks: b[9],
        sides: b[10],
        encoding: b[11],
        bitrate: le16(b, 12),
        rpm: le16(b, 14),
        interface_mode: b[16],
        track_list_offset: lo,
        write_allowed: if b.len() > 20 { b[20] } else { 0 },
        single_step: if b.len() > 21 { b[21] } else { 0 },
        lut_entries: 0,
        max_track_offset: 0,
        payload_len: b.len().saturating_sub(512) as u32,
        truncated: false,
    };
    let base = lo as usize * 512;
    for t in 0..h.tracks as usize {
        let at = base + t * 4;
        if at + 4 > b.len() {
            h.truncated = true;
            break;
        }
        let off = le16(b, at);
        if off > 0 {
            h.lut_entries += 1;
            let byte_off = off as u32 * 512;
            if byte_off > h.max_track_offset {
                h.max_track_offset = byte_off;
            }
        }
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"HXCPICFE".to_vec();
        d.extend_from_slice(&[0, 3, 1, 0]);
        d.extend_from_slice(&[0xF4, 0x01]); // bitrate 500
        d.extend_from_slice(&[0x2C, 0x01]); // rpm 300
        d.extend_from_slice(&[0, 0]);
        d.extend_from_slice(&[1, 0]);
        d.extend_from_slice(&[0, 0]);
        d.resize(512, 0);
        d.extend_from_slice(&[4, 0, 0x20, 0]);
        d.extend_from_slice(&[6, 0, 0x20, 0]);
        d.extend_from_slice(&[0, 0, 0, 0]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"HXCPICFF"));
        let mut d = b"HXCPICFE".to_vec();
        d.extend_from_slice(&[1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!detect(&d)); // revision 1
    }

    #[test]
    fn parses() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.tracks, 3);
        assert_eq!(p.sides, 1);
        assert_eq!(p.bitrate, 500);
        assert_eq!(p.rpm, 300);
        assert_eq!(p.track_list_offset, 1);
        assert_eq!(p.lut_entries, 2);
        assert_eq!(p.max_track_offset, 6 * 512);
        assert!(!p.truncated);
    }

    #[test]
    fn lut_past_eof() {
        let mut d = b"HXCPICFE".to_vec();
        d.extend_from_slice(&[0, 80, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0]);
        let p = parse(&d).unwrap();
        assert!(p.truncated);
    }
}

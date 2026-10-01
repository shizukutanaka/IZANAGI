//! C3D — biomechanics motion-capture file (AMTI/ADTech standard).
//!
//! Byte 0 = parameter-block sector number (×512 → offset), byte 1 = 0x50
//! (Intel byte order). Header then holds LE u16 point/analog/frame fields,
//! an f32 scale factor (raw bits kept; negative ⇒ integer data), and the
//! data-start block number.
//!
//! ```
//! let mut d = vec![0u8; 40];
//! d[0] = 2; // parameter block at offset 1024
//! d[1] = 0x50;
//! d[2..4].copy_from_slice(&21u16.to_le_bytes()); // points
//! d[4..6].copy_from_slice(&0u16.to_le_bytes()); // analog
//! d[6..8].copy_from_slice(&1u16.to_le_bytes()); // first frame
//! d[8..10].copy_from_slice(&450u16.to_le_bytes()); // last frame
//! d[12..16].copy_from_slice(&0xC1200000u32.to_le_bytes()); // scale -10 (integer data)
//! d[16..18].copy_from_slice(&3u16.to_le_bytes()); // data block
//! let c = izanagi_kit::c3d::parse(&d).unwrap();
//! assert_eq!(c.points, 21);
//! assert_eq!(c.frames, 450);
//! assert!(c.integer_data);
//! ```

/// Parsed C3D header (Intel byte order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C3d {
    /// Offset of the parameter block (`byte0 * 512`).
    pub param_offset: usize,
    /// Number of 3D points per frame.
    pub points: u16,
    /// Analog measurements per frame.
    pub analog: u16,
    /// First frame number (1-based).
    pub first_frame: u16,
    /// Last frame number.
    pub last_frame: u16,
    /// Total frames (`last - first + 1`).
    pub frames: u32,
    /// Raw bits of the f32 scale factor.
    pub scale_bits: u32,
    /// True when `scale_bits` is negative — point data stored as integers.
    pub integer_data: bool,
    /// Offset of the point data (`data_block * 512`).
    pub data_offset: usize,
}

fn le16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

/// Parse a C3D header; `None` on short input, wrong marker, or bad frame range.
pub fn parse(d: &[u8]) -> Option<C3d> {
    if d.len() < 20 || d[1] != 0x50 || d[0] == 0 {
        return None;
    }
    let param_offset = d[0] as usize * 512;
    let points = le16(d, 2)?;
    let analog = le16(d, 4)?;
    let first_frame = le16(d, 6)?;
    let last_frame = le16(d, 8)?;
    if first_frame == 0 || last_frame < first_frame {
        return None;
    }
    let scale_bits = u32::from_le_bytes(d.get(12..16)?.try_into().ok()?);
    let data_offset = le16(d, 16)? as usize * 512;
    if points == 0 {
        return None;
    }
    Some(C3d {
        param_offset,
        points,
        analog,
        first_frame,
        last_frame,
        frames: last_frame as u32 - first_frame as u32 + 1,
        scale_bits,
        integer_data: (scale_bits as i32) < 0,
        data_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(points: u16, analog: u16, first: u16, last: u16, scale: u32, dblk: u16) -> Vec<u8> {
        let mut d = vec![0u8; 512];
        d[0] = 2;
        d[1] = 0x50;
        d[2..4].copy_from_slice(&points.to_le_bytes());
        d[4..6].copy_from_slice(&analog.to_le_bytes());
        d[6..8].copy_from_slice(&first.to_le_bytes());
        d[8..10].copy_from_slice(&last.to_le_bytes());
        d[12..16].copy_from_slice(&scale.to_le_bytes());
        d[16..18].copy_from_slice(&dblk.to_le_bytes());
        d
    }

    #[test]
    fn basic() {
        let c = parse(&hdr(10, 8, 1, 100, 0x3F800000, 4)).unwrap();
        assert_eq!(c.param_offset, 1024);
        assert_eq!((c.points, c.analog), (10, 8));
        assert_eq!(c.frames, 100);
        assert_eq!(c.scale_bits, 0x3F800000); // +1\x2e0
        assert!(!c.integer_data);
        assert_eq!(c.data_offset, 2048);
    }

    #[test]
    fn integer_scale() {
        let c = parse(&hdr(5, 0, 1, 10, 0xBF000000, 3)).unwrap();
        assert!(c.integer_data); // negative scale → integer point data
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 40]).is_none()); // no 0x50
        assert!(parse(&hdr(10, 0, 0, 5, 0, 3)).is_none()); // first frame 0
        assert!(parse(&hdr(10, 0, 50, 5, 0, 3)).is_none()); // last < first
        assert!(parse(&hdr(0, 0, 1, 5, 0, 3)).is_none()); // no points
    }
}

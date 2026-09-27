//! RAD Game Tools Bink video header parsing.
//!
//! Magic `BIK` + version letter (`b`, `f`, `i`, ...), then u32LE:
//! `file_size_minus_8`, `num_frames`, `largest_frame`, `width`,
//! `height`, `fps_num`, `fps_denom`, `video_flags`, `audio_tracks`.
//! Per-track `{sample_rate u16, flags u16}` pairs follow at @48.
//!
//! ```
//! use izanagi_kit::bik;
//! let mut d = b"BIKi".to_vec();
//! d.extend_from_slice(&120u32.to_le_bytes()); // file_size - 8
//! d.extend_from_slice(&60u32.to_le_bytes()); // frames
//! d.extend_from_slice(&4096u32.to_le_bytes()); // largest frame
//! d.extend_from_slice(&640u32.to_le_bytes()); // width
//! d.extend_from_slice(&360u32.to_le_bytes()); // height
//! d.extend_from_slice(&30000u32.to_le_bytes()); // fps num
//! d.extend_from_slice(&1001u32.to_le_bytes()); // fps denom
//! d.extend_from_slice(&0u32.to_le_bytes()); // video flags
//! d.extend_from_slice(&0u32.to_le_bytes()); // audio tracks
//! d.extend_from_slice(&[0u8; 92]); // pad to declared size
//! let b = bik::parse(&d).unwrap();
//! assert_eq!(b.num_frames, 60);
//! ```

/// Bink signature `BIK`.
pub const MAGIC: &[u8; 3] = b"BIK";

/// Minimum header length (magic + version + 9 u32).
pub const HEADER_LEN: usize = 4 + 36;

/// A parsed Bink header.
#[derive(Clone, Debug, PartialEq)]
pub struct Bik {
    /// Version letter (`b`/`f`/`i`/`k`...).
    pub version: u8,
    /// Declared `file_size - 8`.
    pub file_size: u32,
    /// `num_frames`.
    pub num_frames: u32,
    /// `largest_frame` byte size.
    pub largest_frame: u32,
    /// `width`.
    pub width: u32,
    /// `height`.
    pub height: u32,
    /// Frame-rate numerator.
    pub fps_num: u32,
    /// Frame-rate denominator.
    pub fps_denom: u32,
    /// `video_flags`.
    pub video_flags: u32,
    /// Number of audio tracks.
    pub audio_tracks: u32,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a Bink header: `BIK` + version letter + u32LE fields;
/// `file_size` must cover the input when non-zero.
pub fn parse(d: &[u8]) -> Option<Bik> {
    if d.len() < HEADER_LEN {
        return None;
    }
    if d.get(..3)? != MAGIC {
        return None;
    }
    let version = *d.get(3)?;
    if !version.is_ascii_alphabetic() {
        return None;
    }
    let file_size = u32le(d, 4)?;
    // file_size counts bytes after the 8-byte magic+size field.
    if file_size != 0 && file_size as usize + 8 > d.len() {
        return None;
    }
    Some(Bik {
        version,
        file_size,
        num_frames: u32le(d, 8)?,
        largest_frame: u32le(d, 12)?,
        width: u32le(d, 16)?,
        height: u32le(d, 20)?,
        fps_num: u32le(d, 24)?,
        fps_denom: u32le(d, 28)?,
        video_flags: u32le(d, 32)?,
        audio_tracks: u32le(d, 36)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.push(b'f');
        for v in [92u32, 300, 2048, 1280, 720, 60, 1, 0, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&[0u8; 92]);
        d
    }

    #[test]
    fn parses_header() {
        let b = parse(&fixture()).unwrap();
        assert_eq!(b.version, b'f');
        assert_eq!(b.num_frames, 300);
        assert_eq!(b.fps_num, 60);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[3] = b'9'; // non-alpha version
        assert!(parse(&d).is_none());
    }
}

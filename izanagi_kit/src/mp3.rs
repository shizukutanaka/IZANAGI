//! MP3 frame-header parsing (MPEG-1/2/2.5 Layer III, no ID3 — see `id3`).
//!
//! Header (4 B, BE): 11-bit sync `0xFFE`, version, layer(=1), protection,
//! bitrate index, sample-rate index, padding, private, channel mode...
//!
//! ```
//! use izanagi_kit::mp3::{parse, Version, Layer};
//!
//! // MPEG-1 L3, 128kbps, 44100Hz, stereo, no CRC -> 0xFF FB 90 00
//! let f = [0xFFu8, 0xFB, 0x90, 0x00, 0, 0, 0, 0];
//! let m = parse(&f).unwrap();
//! assert_eq!(m.version, Version::Mpeg1);
//! assert_eq!(m.layer, Layer::L3);
//! assert_eq!(m.bitrate_kbps, 128);
//! assert_eq!(m.sample_rate, 44100);
//! assert_eq!(m.frame_len, 417);
//! ```

/// MPEG audio version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// MPEG-1.
    Mpeg1,
    /// MPEG-2.
    Mpeg2,
    /// MPEG-2.5.
    Mpeg25,
}

/// MPEG audio layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Layer I.
    L1,
    /// Layer II.
    L2,
    /// Layer III.
    L3,
}

/// Parsed MP3 frame header.
#[derive(Debug)]
pub struct Mp3 {
    /// MPEG version.
    pub version: Version,
    /// Layer.
    pub layer: Layer,
    /// Whether a CRC-16 follows the header (protection bit clear).
    pub has_crc: bool,
    /// Bitrate in kbps.
    pub bitrate_kbps: u32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Padding bit.
    pub padded: bool,
    /// Channel mode raw bits (0=stereo,1=joint,2=dual,3=mono).
    pub channel_mode: u8,
    /// Total frame length in bytes including header.
    pub frame_len: usize,
}

const RATES: [u32; 3] = [44100, 48000, 32000];
const BITRATES_V1L3: [u32; 15] = [
    0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];
const BITRATES_V1L2: [u32; 15] = [
    0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
];
const BITRATES_V1L1: [u32; 15] = [
    0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
];
const BITRATES_V2: [u32; 15] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

/// Parses one 4-byte MP3 frame header (sync + fields). Returns `None` on
/// bad sync, reserved indices, or free/unused bitrate index 0 or 15.
pub fn parse(d: &[u8]) -> Option<Mp3> {
    if d.len() < 4 || d[0] != 0xFF || d[1] & 0xE0 != 0xE0 {
        return None;
    }
    let version = match (d[1] >> 3) & 3 {
        0 => Version::Mpeg25,
        2 => Version::Mpeg2,
        3 => Version::Mpeg1,
        _ => return None,
    };
    let layer = match (d[1] >> 1) & 3 {
        1 => Layer::L3,
        2 => Layer::L2,
        3 => Layer::L1,
        _ => return None,
    };
    let has_crc = d[1] & 1 == 0;
    let bi = (d[2] >> 4) & 0x0F;
    let si = (d[2] >> 2) & 3;
    if si == 3 || bi == 0 || bi == 15 {
        return None;
    }
    let v1 = version == Version::Mpeg1;
    let table = match (v1, layer) {
        (true, Layer::L1) => &BITRATES_V1L1,
        (true, Layer::L2) => &BITRATES_V1L2,
        (true, Layer::L3) => &BITRATES_V1L3,
        (false, _) => &BITRATES_V2,
    };
    let bitrate = table[bi as usize];
    let base_rate = RATES[si as usize];
    let sample_rate = match version {
        Version::Mpeg1 => base_rate,
        Version::Mpeg2 => base_rate / 2,
        Version::Mpeg25 => base_rate / 4,
    };
    let padded = d[2] & 2 != 0;
    let pad = u32::from(padded);
    let frame_len = match layer {
        Layer::L1 => ((12 * bitrate * 1000 / sample_rate) as usize + pad as usize) * 4,
        Layer::L3 => {
            let coef = if v1 { 144 } else { 72 };
            (coef * bitrate * 1000 / sample_rate) as usize + pad as usize
        }
        Layer::L2 => (144 * bitrate * 1000 / sample_rate) as usize + pad as usize,
    };
    Some(Mp3 {
        version,
        layer,
        has_crc,
        bitrate_kbps: bitrate,
        sample_rate,
        padded,
        channel_mode: (d[3] >> 6) & 3,
        frame_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_l3() {
        let m = parse(&[0xFF, 0xFB, 0x90, 0x00]).unwrap();
        assert_eq!(m.bitrate_kbps, 128);
        assert_eq!(m.sample_rate, 44100);
        assert_eq!(m.frame_len, 417);
        assert_eq!(m.channel_mode, 0);
        assert!(!m.has_crc); // protection bit clear => CRC present? bit=1 means NO crc
    }

    #[test]
    fn v25_l3() {
        // 0xFF 0xE3: v=00 MPEG-2.5, L3; bitrate idx 4 -> 32kbps; sr idx 0
        let m = parse(&[0xFF, 0xE3, 0x40, 0x00]).unwrap();
        assert_eq!(m.version, Version::Mpeg25);
        assert_eq!(m.layer, Layer::L3);
        assert_eq!(m.bitrate_kbps, 32);
        assert_eq!(m.sample_rate, 11025);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0xFF, 0x00, 0, 0]).is_none()); // bad sync
        assert!(parse(&[0xFF, 0xFB, 0x0C, 0]).is_none()); // bitrate idx 0
        assert!(parse(&[0xFF, 0xFB, 0xF4, 0]).is_none()); // bitrate idx 15
        assert!(parse(&[0xFF, 0xFB, 0x9C, 0]).is_none()); // sr idx 3
        assert!(parse(&[0xFF]).is_none());
    }
}

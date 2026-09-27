//! Adobe Photoshop PSD/PSB header parsing.
//!
//! Header (26 bytes): `8BPS` magic, version (1 = PSD, 2 = PSB),
//! 6 reserved zero bytes, channel count u16BE, height u32BE,
//! width u32BE, depth u16BE (bits per channel), color mode u16BE.
//! A color-mode data section length u32BE follows.
//!
//! ```
//! use izanagi_kit::psd;
//! let mut d = vec![0u8; 30];
//! d[..4].copy_from_slice(b"8BPS");
//! d[4..6].copy_from_slice(&1u16.to_be_bytes());
//! d[12..14].copy_from_slice(&3u16.to_be_bytes()); // channels
//! d[14..18].copy_from_slice(&64u32.to_be_bytes()); // height
//! d[18..22].copy_from_slice(&64u32.to_be_bytes()); // width
//! d[22..24].copy_from_slice(&8u16.to_be_bytes()); // depth
//! d[24..26].copy_from_slice(&3u16.to_be_bytes()); // RGB
//! let p = psd::parse(&d).unwrap();
//! assert_eq!(p.color_mode, psd::ColorMode::Rgb);
//! ```

/// Color mode byte at offset 25.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorMode {
    /// 0 — bitmap.
    Bitmap,
    /// 1 — grayscale.
    Grayscale,
    /// 2 — indexed.
    Indexed,
    /// 3 — RGB.
    Rgb,
    /// 4 — CMYK.
    Cmyk,
    /// 7 — multichannel.
    Multichannel,
    /// 8 — duotone.
    Duotone,
    /// 9 — Lab.
    Lab,
    /// Any other value.
    Other(u16),
}

/// A parsed PSD header.
#[derive(Clone, Debug, PartialEq)]
pub struct Psd {
    /// 1 = PSD, 2 = PSB.
    pub version: u16,
    /// Channel count (1–56).
    pub channels: u16,
    /// Height in pixels.
    pub height: u32,
    /// Width in pixels.
    pub width: u32,
    /// Bits per channel.
    pub depth: u16,
    /// Color mode.
    pub color_mode: ColorMode,
    /// Byte offset of the color-mode section length field.
    pub section_len_at: usize,
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) << 8 | s[1] as u16)
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

/// Parses a PSD/PSB header: `8BPS`, version 1–2, six reserved
/// zeros, sane channel/depth/mode ranges.
pub fn parse(d: &[u8]) -> Option<Psd> {
    if d.len() < 26 {
        return None;
    }
    if d[..4] != *b"8BPS" {
        return None;
    }
    let version = u16be(d, 4)?;
    if !(version == 1 || version == 2) {
        return None;
    }
    if d[6..12].iter().any(|&b| b != 0) {
        return None;
    }
    let channels = u16be(d, 12)?;
    if channels == 0 || channels > 56 {
        return None;
    }
    let height = u32be(d, 14)?;
    let width = u32be(d, 18)?;
    if height == 0 || width == 0 {
        return None;
    }
    let depth = u16be(d, 22)?;
    if !(depth == 1 || depth == 8 || depth == 16 || depth == 32) {
        return None;
    }
    let color_mode = match u16be(d, 24)? {
        0 => ColorMode::Bitmap,
        1 => ColorMode::Grayscale,
        2 => ColorMode::Indexed,
        3 => ColorMode::Rgb,
        4 => ColorMode::Cmyk,
        7 => ColorMode::Multichannel,
        8 => ColorMode::Duotone,
        9 => ColorMode::Lab,
        v => ColorMode::Other(v),
    };
    Some(Psd {
        version,
        channels,
        height,
        width,
        depth,
        color_mode,
        section_len_at: 26,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 30];
        d[..4].copy_from_slice(b"8BPS");
        d[4..6].copy_from_slice(&1u16.to_be_bytes());
        d[12..14].copy_from_slice(&3u16.to_be_bytes());
        d[14..18].copy_from_slice(&64u32.to_be_bytes());
        d[18..22].copy_from_slice(&128u32.to_be_bytes());
        d[22..24].copy_from_slice(&8u16.to_be_bytes());
        d[24..26].copy_from_slice(&3u16.to_be_bytes());
        d
    }

    #[test]
    fn parses_header() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.channels, 3);
        assert_eq!((p.height, p.width), (64, 128));
        assert_eq!(p.depth, 8);
        assert_eq!(p.color_mode, ColorMode::Rgb);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[3] = b'X';
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[5] = 3; // version 3
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[13] = 60; // 60 channels
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[8] = 1; // reserved nonzero
        assert!(parse(&d).is_none());
    }
}

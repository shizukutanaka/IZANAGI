//! Radiance HDR (`.hdr`, RGBE) file header parsing.
//!
//! Text header: `#?RADIANCE` (or `#?RGBE`) first line, `FORMAT=
//! 32-bit_rle_rgbe` line, a blank line, then the resolution
//! directive `-Y <h> +X <w>` (standard orientation).
//!
//! ```
//! use izanagi_kit::hdr;
//! let d = b"#?RADIANCE\n# comment\nFORMAT=32-bit_rle_rgbe\n\n-Y 4 +X 8\n";
//! let h = hdr::parse(d).unwrap();
//! assert_eq!((h.width, h.height), (8, 4));
//! ```

use std::string::{String, ToString};

/// A parsed HDR header.
#[derive(Clone, Debug, PartialEq)]
pub struct Hdr {
    /// True for `#?RGBE` (older magic).
    pub rgbe_magic: bool,
    /// `FORMAT` line value (e.g. `32-bit_rle_rgbe`).
    pub format: String,
    /// Declared scanline width.
    pub width: u32,
    /// Declared scanline height.
    pub height: u32,
    /// Raw resolution line (e.g. `-Y 4 +X 8`).
    pub res_line: String,
    /// Byte offset of the pixel data.
    pub data_at: usize,
}

/// Parses a Radiance HDR header: magic line, `FORMAT=` must be a
/// `32-bit_rle_rgbe`-class value, blank line, `-Y <h> +X <w>`
/// resolution.
pub fn parse(d: &[u8]) -> Option<Hdr> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split('\n');
    let first = lines.next()?;
    let rgbe = match first.trim_end_matches('\r') {
        "#?RADIANCE" => false,
        "#?RGBE" => true,
        _ => return None,
    };
    let mut format = None;
    let mut at = first.len() + 1;
    for line in &mut lines {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            at += line.len() + 1;
            // next line must be the resolution directive
            let res = lines.next()?;
            let res = res.trim_end_matches('\r');
            let (w, h) = parse_res(res)?;
            return Some(Hdr {
                rgbe_magic: rgbe,
                format: format?,
                width: w,
                height: h,
                res_line: res.to_string(),
                data_at: at + res.len() + 1,
            });
        }
        if let Some(v) = line.strip_prefix("FORMAT=") {
            format = Some(v.to_string());
        }
        at += line.len() + 1;
        if at > 1 << 20 {
            return None;
        }
    }
    None
}

fn parse_res(line: &str) -> Option<(u32, u32)> {
    // standard: "-Y <h> +X <w>"; also accept "+Y/-X" variants
    let mut parts = line.split(' ');
    let a = parts.next()?;
    let n1: u32 = parts.next()?.parse().ok()?;
    let b = parts.next()?;
    let n2: u32 = parts.next()?.parse().ok()?;
    let (w, h) = match (a, b) {
        ("-Y", "+X") | ("+Y", "+X") => (n2, n1),
        ("-Y", "-X") | ("+Y", "-X") => (n2, n1),
        _ => return None,
    };
    if w == 0 || h == 0 {
        return None;
    }
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    const DOC: &[u8] =
        b"#?RADIANCE\n# created by test\nEXPOSURE=1.000\nFORMAT=32-bit_rle_rgbe\n\n-Y 480 +X 640\n";

    #[test]
    fn parses() {
        let h = parse(DOC).unwrap();
        assert!(!h.rgbe_magic);
        assert_eq!(h.format, "32-bit_rle_rgbe");
        assert_eq!((h.width, h.height), (640, 480));
        assert_eq!(h.res_line, "-Y 480 +X 640");
    }

    #[test]
    fn rgbe_and_rejects() {
        let d = b"#?RGBE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 1\n";
        assert!(parse(d).unwrap().rgbe_magic);
        assert!(parse(b"#?OTHER\n\n-Y 1 +X 1\n").is_none());
        assert!(parse(b"#?RADIANCE\n\n-Y 0 +X 1\n").is_none());
        assert!(parse(DOC.split_at(10).0).is_none());
    }
}

//! X11 XBM bitmap — a C-source text format.
//!
//! XBM files are literally C code: `#define <name>_width N`,
//! `#define <name>_height N`, then `static unsigned char
//! <name>_bits[] = { 0x.., ... };`. Bits are packed least-significant
//! bit first, so column `x` of row `y` is bit `x % 8` of byte
//! `y * ceil(w/8) + x/8`.
//!
//! ```
//! use izanagi_kit::xbm::parse;
//!
//! let s = "#define a_width 8\n#define a_height 2\n\
//!          static unsigned char a_bits[] = { 0x01, 0x55 };\n";
//! let x = parse(s).unwrap();
//! assert_eq!((x.width, x.height), (8, 2));
//! assert_eq!(x.data, vec![0x01, 0x55]);
//! assert!(x.pixel(0, 0));      // 0x01 bit 0
//! assert!(!x.pixel(1, 0));
//! ```

use std::vec::Vec;

/// A parsed XBM bitmap.
#[derive(Clone, Debug, PartialEq)]
pub struct Xbm {
    /// Width in pixels (from `#define *_width`).
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Packed bits, LSB-first per byte.
    pub data: Vec<u8>,
}

impl Xbm {
    /// Bytes per row (`ceil(width / 8)`).
    pub fn stride(&self) -> usize {
        usize::try_from(self.width.saturating_add(7)).unwrap_or(u32::MAX as usize) / 8
    }

    /// Total bytes the bitmap needs (`stride * height`).
    pub fn data_len(&self) -> usize {
        self.stride()
            .saturating_mul(usize::try_from(self.height).unwrap_or(u32::MAX as usize))
    }

    /// Pixel at column `x`, row `y` (LSB-first packing). `false` for
    /// out-of-bounds coordinates or missing data bytes.
    pub fn pixel(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let at = usize::try_from(y)
            .ok()
            .and_then(|y| y.checked_mul(self.stride()))
            .and_then(|b| b.checked_add(usize::try_from(x / 8).ok()?));
        match at.and_then(|at| self.data.get(at)) {
            Some(b) => b & (1 << (x % 8)) != 0,
            None => false,
        }
    }
}

fn u32_dec(t: &str, key: &str) -> Option<u32> {
    let i = t.find(key)? + key.len();
    let rest = t.get(i..)?.trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    rest.get(..end)?.parse().ok()
}

/// Parse XBM source text. Returns `None` when the width/height defines
/// or a `{ ... }` byte list are missing, or a byte literal is not a
/// `0x..`/decimal token inside the array.
pub fn parse(s: &str) -> Option<Xbm> {
    let width = u32_dec(s, "_width")?;
    let height = u32_dec(s, "_height")?;
    let open = s.find('{')? + 1;
    let close = s.get(open..)?.find('}')? + open;
    let mut data = Vec::new();
    for tok in s.get(open..close)?.split(',') {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        let v = if let Some(h) = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")) {
            u8::from_str_radix(h, 16).ok()?
        } else {
            tok.parse().ok()?
        };
        data.push(v);
    }
    let x = Xbm {
        width,
        height,
        data,
    };
    if x.data_len() > x.data.len() {
        return None;
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "#define face_width 16\n#define face_height 4\n\
        static unsigned char face_bits[] = {\n  0xFF, 0x80, 0x01, 0xFF,\n  0x00, 0x42, 0x24, 0x00 };\n";

    #[test]
    fn parses_defines_and_bits() {
        let x = parse(SRC).unwrap();
        assert_eq!((x.width, x.height), (16, 4));
        assert_eq!(x.data, vec![0xFF, 0x80, 0x01, 0xFF, 0, 0x42, 0x24, 0]);
        assert_eq!(x.stride(), 2);
        assert_eq!(x.data_len(), 8);
    }

    #[test]
    fn pixel_lsb_first() {
        let x = parse(SRC).unwrap();
        assert!(x.pixel(0, 0));
        assert!(x.pixel(7, 0)); // 0x80 bit 7 -> x 7
        assert!(!x.pixel(8, 0));
        assert!(x.pixel(15, 0)); // 0x80 at byte 1 -> x 15
        assert!(x.pixel(0, 1)); // byte 2 = 0x01
        assert!(!x.pixel(1, 2)); // byte 4 = 0x00
        assert!(x.pixel(9, 2)); // byte 5 = 0x42: bit 1 -> col 9
        assert!(x.pixel(14, 2)); // byte 5 = 0x42: bit 6 -> col 14
        assert!(!x.pixel(16, 0));
        assert!(!x.pixel(0, 4));
    }

    #[test]
    fn decimal_literals_and_spaces() {
        let s = "#define b_width 16\n#define b_height 1\nstatic char b_bits[]={ 255, 0 };\n";
        let x = parse(s).unwrap();
        assert_eq!(x.data, vec![255, 0]);
    }

    #[test]
    fn rejects_missing_and_short() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("#define a_width 8\n"), None);
        assert_eq!(parse("#define a_width 8\n#define a_height 1\n"), None);
        // needs 1 byte but supplies 0
        assert_eq!(
            parse("#define a_width 8\n#define a_height 1\nstatic char b[]={ };\n"),
            None
        );
        // bad literal
        assert_eq!(
            parse("#define a_width 8\n#define a_height 1\nstatic char b[]={ 0xZZ };\n"),
            None
        );
    }
}

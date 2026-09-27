//! FrSky SBUS — 25-byte RC frame: `0x0F` start, 16 channels × 11 bits
//! packed little-endian, a flag byte, `0x00` end.
//!
//! Flags: bit0=ch17 digital, bit1=ch18 digital, bit2=frame_lost,
//! bit3=failsafe.
//!
//! ```
//! use izanagi_kit::sbus::parse;
//!
//! let mut f = [0u8; 25];
//! f[0] = 0x0F;
//! f[23] = 0b0000_0100; // frame_lost
//! let s = parse(&f).unwrap();
//! assert_eq!(s.channels[0], 0);
//! assert!(s.frame_lost);
//! ```

/// A decoded SBUS frame.
#[derive(Clone, Debug)]
pub struct Sbus {
    /// 16 channels, each 0..2047 (11 bits).
    pub channels: [u16; 16],
    /// Digital channel 17.
    pub ch17: bool,
    /// Digital channel 18.
    pub ch18: bool,
    /// Frame-lost flag.
    pub frame_lost: bool,
    /// Failsafe flag.
    pub failsafe: bool,
}

/// Decode a 25-byte SBUS frame. `None` on wrong length or start byte.
pub fn parse(d: &[u8]) -> Option<Sbus> {
    if d.len() != 25 || d[0] != 0x0F {
        return None;
    }
    let b = &d[1..23];
    let mut channels = [0u16; 16];
    for (i, ch) in channels.iter_mut().enumerate() {
        // 11 bits per channel, little-endian bit order
        let bit = i * 11;
        let byte = bit / 8;
        let shift = bit % 8;
        let lo = b[byte] as u32;
        let hi = if byte + 1 < 22 { b[byte + 1] as u32 } else { 0 };
        let hi2 = if byte + 2 < 22 { b[byte + 2] as u32 } else { 0 };
        *ch = (((lo | (hi << 8) | (hi2 << 16)) >> shift) & 0x7FF) as u16;
    }
    let flags = d[23];
    Some(Sbus {
        channels,
        ch17: flags & 1 != 0,
        ch18: flags & 2 != 0,
        frame_lost: flags & 4 != 0,
        failsafe: flags & 8 != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut f = [0u8; 25];
        f[0] = 0x0F;
        // channel 0 = 0x7FF (all bits set in first 11 bits)
        f[1] = 0xFF;
        f[2] = 0x07;
        let s = parse(&f).unwrap();
        assert_eq!(s.channels[0], 0x7FF);
        assert_eq!(s.channels[1], 0);
        // mid frame values
        let mut f2 = [0u8; 25];
        f2[0] = 0x0F;
        f2[23] = 0b1111;
        let s2 = parse(&f2).unwrap();
        assert!(s2.ch17 && s2.ch18 && s2.frame_lost && s2.failsafe);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 24]).is_none());
        let mut f = [0u8; 25];
        f[0] = 0x0E;
        assert!(parse(&f).is_none());
    }
}

//! Brotli — RFC 7932's bitstream head. Unlike gzip/zstd there is no
//! magic: the stream opens with `WBITS`, the sliding-window size,
//! coded LSB-first in the first byte(s): a `0` bit means 16,
//! otherwise a 3-bit value `n` — `n > 0` gives `17 + n` (18..=24),
//! `n == 0` selects the extended form: one more bit `0` means 17,
//! `1` then 3 bits `m` gives either 10 (`m == 0`... large-window
//! encodings use `m`) — see `window_bits` for the exact ladder.
//!
//! ```
//! use izanagi_kit::brotli::window_bits;
//! assert_eq!(window_bits(&[0x00]), Some(16)); // first bit 0
//! assert_eq!(window_bits(&[0b0011]), Some(18)); // 1 then n=1
//! assert_eq!(window_bits(&[]), None);
//! ```

/// Read `WBITS` from the start of a brotli bitstream.
///
/// Bits are consumed LSB-first. The ladder (RFC 7932 §9.1):
///
/// | bits seen           | window |
/// |---------------------|--------|
/// | `0`                 | 16     |
/// | `1`, `n!=0`         | 17+n   |
/// | `1`,`000`,`m!=0`    | 17+m   |
/// | `1`,`000`,`000`,`0` | 17     |
/// | `1`,`000`,`000`,`1`,`0` | 10 |
/// | `1`,`000`,`000`,`1`,`1` | 17 |
///
/// `None` when the prefix is truncated.
pub fn window_bits(d: &[u8]) -> Option<u16> {
    let mut bits = Bits::new(d);
    if bits.take(1)? == 0 {
        return Some(16);
    }
    let n = bits.take(3)?;
    if n != 0 {
        return Some(17 + u16::from(n as u8));
    }
    let m = bits.take(3)?;
    if m != 0 {
        // extended / large-window: 17 + m gives 18..24 again on some
        // encoders; RFC's large-window range is 17+m
        return Some(17 + u16::from(m as u8));
    }
    // n == 0, m == 0: one more bit
    if bits.take(1)? == 0 {
        return Some(17);
    }
    // '1 000 0001 x': WBITS = 10? — RFC 7932: last resort codes
    let last = bits.take(1)?;
    Some(if last == 0 { 10 } else { 17 })
}

/// LSB-first bit reader over a byte slice.
pub struct Bits<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> Bits<'a> {
    /// Wrap a slice.
    pub fn new(d: &'a [u8]) -> Self {
        Bits { d, at: 0 }
    }

    /// Take `n` bits (0..=8), LSB-first.
    pub fn take(&mut self, n: u8) -> Option<u16> {
        if n == 0 {
            return Some(0);
        }
        let mut v = 0u16;
        for i in 0..n {
            let byte = *self.d.get(self.at / 8)?;
            v |= u16::from((byte >> (self.at % 8)) & 1) << i;
            self.at += 1;
        }
        Some(v)
    }

    /// Bits consumed so far.
    pub fn taken(&self) -> usize {
        self.at
    }
}

/// The documented-but-extended large-window codes are folded into the
/// 17+m ladder above; callers needing RFC 7749 large-window brotli
/// should treat `wbits > 24` as extended.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wbits_ladder() {
        // bit0=0 → 16
        assert_eq!(window_bits(&[0x00]), Some(16));
        // 1 then n=1..7 (LSB-first): byte = 1 | (n<<1)
        for n in 1u8..8 {
            assert_eq!(
                window_bits(&[1 | (n << 1)]),
                Some(17 + u16::from(n)),
                "n={n}"
            );
        }
        // n=0, m=0, bit7=0 → 17
        assert_eq!(window_bits(&[0b0000_0001, 0]), Some(17));
        // n=0, m=1 → 17+1=18
        assert_eq!(window_bits(&[0b0001_0001]), Some(18));
        // n=0, m=0, bit7=1, bit8=0 → 10
        assert_eq!(window_bits(&[0b1000_0001, 0]), Some(10));
        assert_eq!(window_bits(&[]), None);
        // deepest ladder needs a 9th bit: `1`,`000`,`000`,`1` then EOF
        assert_eq!(window_bits(&[0x81]), None);
    }

    #[test]
    fn bits_reader() {
        let mut b = Bits::new(&[0b1010_0011, 0xFF]);
        assert_eq!(b.take(4), Some(0b0011));
        assert_eq!(b.take(4), Some(0b1010));
        assert_eq!(b.take(8), Some(0xFF));
        assert_eq!(b.take(1), None);
        assert_eq!(b.taken(), 16);
        assert_eq!(Bits::new(&[7]).take(0), Some(0));
    }
}

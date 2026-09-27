//! Minimal reader for SRTM `.hgt` elevation tiles: a square grid of
//! big-endian i16 samples. File size alone determines the resolution —
//! SRTM-1 is 3601×3601, SRTM-3 is 1201×1201; other square grid sizes are
//! accepted.
//!
//! ```
//! use izanagi_kit::hgt::parse;
//!
//! // 2×2 custom grid: four BE i16 samples
//! let h = parse(&[0, 10, 0, 20, 0, 30, 0, 40]).unwrap();
//! assert_eq!(h.side, 2);
//! assert_eq!(h.at(0, 0), Some(10));
//! assert_eq!(h.at(1, 0), Some(20));
//! assert_eq!(h.at(0, 1), Some(30));
//! assert_eq!(h.at(1, 1), Some(40));
//! ```

/// A decoded `.hgt` grid.
#[derive(Debug)]
pub struct Hgt {
    /// Samples per side (`1201`, `3601`, or any other square root).
    pub side: usize,
    /// `true` for 3601 (SRTM-1 arc-second resolution).
    pub srtm1: bool,
    /// Samples in row-major order (north-to-south, west-to-east).
    pub values: Vec<i16>,
}

impl Hgt {
    /// Sample at `(x, y)` — `x` east, `y` south of the NW corner.
    pub fn at(&self, x: usize, y: usize) -> Option<i16> {
        if x >= self.side || y >= self.side {
            return None;
        }
        self.values.get(y * self.side + x).copied()
    }
}

/// Integer square root: largest `s` with `s*s <= n`.
fn isqrt(n: usize) -> usize {
    let mut s = 0usize;
    while (s + 1) * (s + 1) <= n {
        s += 1;
    }
    s
}

/// Parse `.hgt` bytes into a sample grid. `None` on empty input, odd
/// length, or a non-square sample count.
pub fn parse(d: &[u8]) -> Option<Hgt> {
    if d.is_empty() || d.len() % 2 != 0 {
        return None;
    }
    let n = d.len() / 2;
    let side = isqrt(n);
    if side == 0 || side * side != n {
        return None;
    }
    let mut values = Vec::with_capacity(n);
    for pair in d.chunks_exact(2) {
        let hi = pair[0] as i16;
        let lo = pair[1] as u16;
        values.push((hi << 8) | lo as i16);
    }
    Some(Hgt {
        side,
        srtm1: side == 3601,
        values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let h = parse(&[0, 10, 0, 20, 0, 30, 0, 40]).unwrap();
        assert_eq!(h.side, 2);
        assert!(!h.srtm1);
        assert_eq!(h.values.len(), 4);
        assert_eq!(h.at(1, 1), Some(40));
        assert_eq!(h.at(2, 0), None);
        // void marker -32768 decodes as i16
        let h = parse(&[0x80, 0x00]).unwrap();
        assert_eq!(h.at(0, 0), Some(-32768));
        assert_eq!(h.values[0], i16::MIN);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0, 1, 0]).is_none()); // odd length
        assert!(parse(&[0; 6]).is_none()); // 3 samples — not square
        assert!(parse(&[0; 10]).is_none()); // 5 samples — not square
    }

    #[test]
    fn isqrt_correct() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(8), 2);
        assert_eq!(isqrt(1_442_401), 1201); // SRTM-3 sample count
        assert_eq!(isqrt(12_967_201), 3601); // SRTM-1
    }
}

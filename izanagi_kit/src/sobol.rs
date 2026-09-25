//! Sobol' low-discrepancy sequence — 2-D quasi-random points with
//! provably better coverage than `SplitMix64` draws, from direction
//! numbers in the Joe–Kuo tradition (the canonical `s=2, a=1` dim-2
//! table with initial `m = 1, 3`: primitive polynomial `x² + x + 1`,
//! recurrence `m_i = 2m_{i−1} ⊕ 4m_{i−2} ⊕ m_{i−2}`).
//!
//! Points live in `[0,1)²` as `u32` fractions (`x / 2³²`) or `Fixed`
//! via the top 16 bits. Stratification is the honest claim: within
//! every aligned block of 4 consecutive indices the four quadrants
//! are each hit exactly once — a property test, not a promise.
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::sobol::sobol2;
//!
//! // Canonical opening: 1/2 then the 3/4-1/4 anti-diagonal pair.
//! assert_eq!(sobol2(1), (0x8000_0000, 0x8000_0000));
//! assert_eq!(sobol2(2), (0x4000_0000, 0xC000_0000));
//! assert_eq!(sobol2(3), (0xC000_0000, 0x4000_0000));
//! let (_, fy) = sobol2(4);
//! assert_eq!(fy, 0x6000_0000);
//! let one = Fixed::from_raw(0x8000);
//! assert_eq!(one, Fixed::from_ratio(1, 2));
//! ```

use crate::fixed::Fixed;

/// Direction numbers `v_i = m_i ≪ (32 − i)` for both dimensions,
/// `i = 1..=32`. Dim 1 is the `x`-axis sequence (`m_i = 1`); dim 2
/// uses `x²+x+1` with Joe–Kuo initials `m₁ = 1, m₂ = 3`.
fn direction_numbers() -> ([u32; 32], [u32; 32]) {
    let mut v1 = [0u32; 32];
    let mut v2 = [0u32; 32];
    for (i, v) in v1.iter_mut().enumerate() {
        *v = 1u32 << (31 - i);
    }
    let mut m = [0u32; 32];
    m[0] = 1;
    m[1] = 3; // Joe–Kuo dim-2 initials: 1, 3 — not 1, 1
    for i in 2..32 {
        m[i] = (2 * m[i - 1]) ^ (4 * m[i - 2]) ^ m[i - 2];
    }
    for i in 0..32 {
        v2[i] = m[i] << (31 - i);
    }
    (v1, v2)
}

/// The `n`-th Sobol' point (1-indexed; `n = 0` yields `(0, 0)`):
/// `x_d(n) = ⊕_{bit i of n} v_i` per dimension — the direct-index
/// textbook form, so `sobol2(n)` is a pure function of `n`.
pub fn sobol2(n: u32) -> (u32, u32) {
    let (v1, v2) = direction_numbers();
    let (mut x, mut y) = (0u32, 0u32);
    for i in 0..32 {
        if n & (1 << i) != 0 {
            x ^= v1[i];
            y ^= v2[i];
        }
    }
    (x, y)
}

/// Same point as `Fixed` values in `[0,1)` — the top 16 bits of the
/// `u32` fraction, so `Fixed::from_raw(x >> 16)` quantizes at `1/65536`.
pub fn sobol2_fixed(n: u32) -> (Fixed, Fixed) {
    let (x, y) = sobol2(n);
    (
        Fixed::from_raw((x >> 16) as i32),
        Fixed::from_raw((y >> 16) as i32),
    )
}

/// Sequential 2-D Sobol' iterator — `next()` walks `n = 1, 2, …`.
/// Stateless beyond the counter, so a `Sobol2` is its own replay
/// checkpoint.
#[derive(Clone, Copy, Debug)]
pub struct Sobol2 {
    n: u32,
}

impl Sobol2 {
    /// Start at `n = 1` (the first non-zero point).
    pub const fn new() -> Self {
        Self { n: 0 }
    }

    /// Next point as `(Fixed, Fixed)` in `[0,1)²`.
    pub fn next_fixed(&mut self) -> (Fixed, Fixed) {
        self.n = self.n.wrapping_add(1);
        sobol2_fixed(self.n)
    }

    /// Next point as raw `u32` fractions.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> (u32, u32) {
        self.n = self.n.wrapping_add(1);
        sobol2(self.n)
    }
}

impl Default for Sobol2 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_first_points() {
        // Textbook opening (Joe–Kuo dim-2, s=2, a=1, m=(1,3)):
        // x: 1/2, 1/4, 3/4, 1/8, 5/8, 3/8, 7/8
        // y: 1/2, 3/4, 1/4, 3/8, 7/8, 5/8, 1/8
        let want_x = [
            0x8000_0000u32,
            0x4000_0000,
            0xC000_0000,
            0x2000_0000,
            0xA000_0000,
            0x6000_0000,
            0xE000_0000,
        ];
        let want_y = [
            0x8000_0000u32,
            0xC000_0000,
            0x4000_0000,
            0x6000_0000,
            0xE000_0000,
            0xA000_0000,
            0x2000_0000,
        ];
        for n in 1..=7u32 {
            let (x, y) = sobol2(n);
            assert_eq!(x, want_x[(n - 1) as usize], "x at n={n}");
            assert_eq!(y, want_y[(n - 1) as usize], "y at n={n}");
        }
    }

    #[test]
    fn blocks_of_four_hit_every_quadrant() {
        // Every aligned 4-block is a perfect 2×2 stratification:
        // the two top bits of x and y enumerate all four quadrants.
        for base in (0u32..512).step_by(4) {
            let mut seen = [false; 4];
            for n in base..base + 4 {
                let (x, y) = sobol2(n);
                let q = ((x >> 31) | ((y >> 31) << 1)) as usize;
                assert!(!seen[q], "quadrant {q} twice in block {base}");
                seen[q] = true;
            }
            assert!(seen.iter().all(|&s| s), "block {base} misses a quadrant");
        }
    }

    #[test]
    fn blocks_of_sixteen_stratify_4x4() {
        // Aligned 16-blocks tile the 4×4 grid exactly once each.
        for base in (0u32..=1024).step_by(16) {
            let mut seen = [false; 16];
            for n in base..base + 16 {
                let (x, y) = sobol2(n);
                let q = ((x >> 30) | ((y >> 30) << 2)) as usize;
                seen[q] = true;
            }
            assert!(seen.iter().all(|&s| s), "block {base} not stratified");
        }
    }

    #[test]
    fn iterator_matches_direct_index() {
        let mut s = Sobol2::new();
        for n in 1..=64u32 {
            assert_eq!(s.next(), sobol2(n));
        }
        let mut s = Sobol2::new();
        for n in 1..=64u32 {
            assert_eq!(s.next_fixed(), sobol2_fixed(n));
        }
    }

    #[test]
    fn deterministic_and_distinct() {
        let mut points = std::collections::BTreeSet::new();
        for n in 1..=4096u32 {
            assert!(points.insert(sobol2(n)), "collision at {n}");
        }
        assert_eq!(sobol2(777), sobol2(777));
        assert_eq!(sobol2(0), (0, 0));
    }
}

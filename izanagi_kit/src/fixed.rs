//! Q16.16 fixed-point scalar.
//!
//! Gameplay/simulation math that must replay bit-identically across CPUs,
//! compilers and OSes cannot use `f32`/`f64`: IEEE results are deterministic
//! per-binary but not reproducible across targets (arxiv/Gaffer/lockstep
//! literature). Fixed-point integer math sidesteps this entirely. Use this for
//! positions, velocities, and anything that feeds the world hash / replay.
//!
//! Range: ±32767.99998, resolution 1/65536. Arithmetic uses i64 intermediates
//! to avoid overflow on multiply/divide.

const FRAC_BITS: u32 = 16;
const ONE: i32 = 1 << FRAC_BITS;

// --- Transcendental constants (Q16.16), all integer literals so the bit pattern
// is identical on every target. Deriving them from `f64` at runtime would
// reintroduce the cross-platform variance this module exists to avoid.
/// π in Q16.16 (3.14159265 · 2¹⁶ ≈ 205887).
const PI: i32 = 205887;
/// 2π in Q16.16.
const TWO_PI: i32 = 411774;
/// π/2 in Q16.16.
const HALF_PI: i32 = 102944;
/// The CORDIC scaling constant K = 1/A ≈ 0.60725293, where A ≈ 1.64676 is the
/// pseudo-rotation gain (A = Π√(1+2⁻²ⁱ)). Seeding the rotation-mode x with K
/// cancels the gain so the final (x, y) read out as (cos, sin) directly.
const CORDIC_K: i32 = 39797;
/// atan(2⁻ⁱ) in Q16.16 for i = 0..16. Σ ≈ 114246 (≈1.7433 rad) bounds the
/// rotation/vectoring convergence range, which comfortably covers ±π/2.
const CORDIC_ATAN: [i32; 16] = [
    51472, 30386, 16055, 8149, 4091, 2047, 1024, 512, 256, 128, 64, 32, 16, 8, 4, 2,
];

/// Floor of the integer square root of `n`, computed bit-by-bit with only
/// add/shift/compare. (`u64::isqrt` would be cleaner but stabilised in 1.84,
/// past this crate's 1.75 MSRV.) Deterministic on every target.
fn isqrt_u64(n: u64) -> u64 {
    let mut rem = n;
    let mut root: u64 = 0;
    // Largest power of four not exceeding `n`.
    let mut bit: u64 = 1 << 62;
    while bit > rem {
        bit >>= 2;
    }
    while bit != 0 {
        if rem >= root + bit {
            rem -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// Fixed-point number stored as a scaled `i32`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Fixed(i32);

impl Fixed {
    pub const ZERO: Fixed = Fixed(0);
    pub const ONE: Fixed = Fixed(ONE);
    pub const MAX: Fixed = Fixed(i32::MAX);
    pub const MIN: Fixed = Fixed(i32::MIN);

    /// Saturates rather than wrapping: `from_int(32768)` exceeds the Q16.16
    /// integer range and clamps to the maximum instead of silently flipping
    /// sign (the bare `value << 16` did the latter, breaking the module's
    /// "never wrap to the opposite extreme" invariant at construction time).
    #[inline]
    pub fn from_int(value: i32) -> Self {
        Fixed(value.saturating_mul(ONE))
    }

    /// Constructs from a ratio `num/den` without touching floats. A zero
    /// denominator saturates toward `num`'s sign (consistent with [`Fixed::div`])
    /// instead of panicking, and an out-of-range quotient clamps via
    /// [`Fixed::from_wide`] instead of a wrapping `as i32` cast.
    #[inline]
    pub fn from_ratio(num: i32, den: i32) -> Self {
        if den == 0 {
            return if num >= 0 {
                Fixed(i32::MAX)
            } else {
                Fixed(i32::MIN)
            };
        }
        Fixed::from_wide(((num as i64) << FRAC_BITS) / den as i64)
    }

    #[inline]
    pub fn raw(self) -> i32 {
        self.0
    }

    #[inline]
    pub fn to_int_trunc(self) -> i32 {
        self.0 >> FRAC_BITS
    }

    #[inline]
    pub fn saturating_add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(rhs.0))
    }

    /// Clamps an i64 intermediate into the i32 fixed range instead of a silent
    /// truncating `as i32` (which can flip sign on overflow).
    #[inline]
    fn from_wide(wide: i64) -> Fixed {
        if wide > i32::MAX as i64 {
            Fixed(i32::MAX)
        } else if wide < i32::MIN as i64 {
            Fixed(i32::MIN)
        } else {
            Fixed(wide as i32)
        }
    }

    // Named methods (not the `Mul`/`Div` operators) on purpose: these have
    // saturating, divide-by-zero-safe Q16.16 semantics that differ from the
    // plain integer operators, so we keep them explicit rather than overloading
    // `*` / `/` with surprising behaviour.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn mul(self, rhs: Fixed) -> Fixed {
        Fixed::from_wide(((self.0 as i64) * rhs.0 as i64) >> FRAC_BITS)
    }

    /// Division. A zero divisor saturates toward the dividend's sign rather than
    /// panicking, so a stray 0 in content never crashes the sim.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn div(self, rhs: Fixed) -> Fixed {
        if rhs.0 == 0 {
            return if self.0 >= 0 {
                Fixed(i32::MAX)
            } else {
                Fixed(i32::MIN)
            };
        }
        Fixed::from_wide(((self.0 as i64) << FRAC_BITS) / rhs.0 as i64)
    }

    /// Square root. Negative inputs saturate to zero (√ of a negative is
    /// undefined; clamping is the safe deterministic choice rather than panicking
    /// or producing garbage). Computed with integer-only [`isqrt_u64`] so the
    /// result is bit-identical across targets and feeds the world hash safely.
    ///
    /// Identity: `√(raw/2¹⁶)·2¹⁶ = √(raw·2¹⁶)`, so we isqrt `raw << 16`.
    #[inline]
    pub fn sqrt(self) -> Fixed {
        if self.0 <= 0 {
            return Fixed::ZERO;
        }
        let widened = (self.0 as u64) << FRAC_BITS;
        Fixed(isqrt_u64(widened) as i32)
    }

    /// Sine and cosine of an angle in radians, via rotation-mode CORDIC (only
    /// shifts/adds — no float, no lookup of `f32::sin`). 16 iterations give
    /// ~Q16.16 precision. Returns `(sin, cos)`; [`Fixed::sin`]/[`Fixed::cos`]
    /// wrap this.
    pub fn sin_cos(self) -> (Fixed, Fixed) {
        // Range-reduce into [-π/2, π/2], the CORDIC convergence window, tracking
        // a sign flip for the second/third quadrants (sin/cos negate across ±π).
        let mut angle = self.0 as i64 % TWO_PI as i64;
        if angle > PI as i64 {
            angle -= TWO_PI as i64;
        } else if angle < -(PI as i64) {
            angle += TWO_PI as i64;
        }
        let mut negate = false;
        if angle > HALF_PI as i64 {
            angle -= PI as i64;
            negate = true;
        } else if angle < -(HALF_PI as i64) {
            angle += PI as i64;
            negate = true;
        }

        let mut x: i32 = CORDIC_K;
        let mut y: i32 = 0;
        let mut z: i32 = angle as i32;
        for (i, &atan) in CORDIC_ATAN.iter().enumerate() {
            let xs = x >> i;
            let ys = y >> i;
            if z >= 0 {
                x -= ys;
                y += xs;
                z -= atan;
            } else {
                x += ys;
                y -= xs;
                z += atan;
            }
        }
        if negate {
            x = -x;
            y = -y;
        }
        (Fixed(y), Fixed(x))
    }

    /// Sine of an angle in radians. See [`Fixed::sin_cos`].
    #[inline]
    pub fn sin(self) -> Fixed {
        self.sin_cos().0
    }

    /// Cosine of an angle in radians. See [`Fixed::sin_cos`].
    #[inline]
    pub fn cos(self) -> Fixed {
        self.sin_cos().1
    }

    /// Four-quadrant arctangent of `y/x` in radians, via vectoring-mode CORDIC.
    /// Result is in (-π, π]. The rotation runs in `i64` because the CORDIC gain
    /// grows the working vector ~1.6× and `y`/`x` may already be near the `i32`
    /// extremes. The angle accumulator is gain-independent, so no `1/K` seed is
    /// needed here.
    pub fn atan2(y: Fixed, x: Fixed) -> Fixed {
        let (xr, yr) = (x.0, y.0);
        if xr == 0 {
            return if yr > 0 {
                Fixed(HALF_PI)
            } else if yr < 0 {
                Fixed(-HALF_PI)
            } else {
                Fixed::ZERO
            };
        }
        // CORDIC vectoring converges for x > 0; for x < 0 rotate the input by π
        // (negate both components) and add ±π back afterwards.
        let negate = xr < 0;
        let mut vx = xr as i64;
        let mut vy = yr as i64;
        if negate {
            vx = -vx;
            vy = -vy;
        }
        let mut z: i64 = 0;
        for (i, &atan) in CORDIC_ATAN.iter().enumerate() {
            let xs = vx >> i;
            let ys = vy >> i;
            if vy < 0 {
                vx -= ys;
                vy += xs;
                z -= atan as i64;
            } else {
                vx += ys;
                vy -= xs;
                z += atan as i64;
            }
        }
        if negate {
            if yr >= 0 {
                z += PI as i64;
            } else {
                z -= PI as i64;
            }
        }
        Fixed::from_wide(z)
    }
}

// Add/Sub saturate (not wrap). Per the SMT fixed-point literature, saturation
// keeps an overflowed result at the nearest representable extreme; wrap-around
// would flip a position to the opposite extreme — a silent gameplay bug. For a
// deterministic engine, saturation is both safer and still bit-exact.
impl core::ops::Add for Fixed {
    type Output = Fixed;
    #[inline]
    fn add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(rhs.0))
    }
}

impl core::ops::Sub for Fixed {
    type Output = Fixed;
    #[inline]
    fn sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_sub(rhs.0))
    }
}

impl crate::world_hash::DetHash for Fixed {
    #[inline]
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        hasher.write_i32(self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_int_roundtrips_through_trunc() {
        assert_eq!(Fixed::from_int(7).to_int_trunc(), 7);
        assert_eq!(Fixed::from_int(-3).to_int_trunc(), -3);
    }

    #[test]
    fn test_mul_half_by_half_is_quarter() {
        let half = Fixed::from_ratio(1, 2);
        let quarter = half.mul(half);
        assert_eq!(quarter, Fixed::from_ratio(1, 4));
    }

    #[test]
    fn test_div_inverse_of_mul() {
        let a = Fixed::from_int(6);
        let b = Fixed::from_int(3);
        assert_eq!(a.div(b), Fixed::from_int(2));
    }

    #[test]
    fn test_raw_bits_are_stable_for_known_value() {
        // 1.5 == 0x00018000 — a fixed bit pattern, identical on every target.
        assert_eq!(Fixed::from_ratio(3, 2).raw(), 0x0001_8000);
    }

    #[test]
    fn test_add_saturates_instead_of_wrapping() {
        let big = Fixed(i32::MAX);
        // Adding one unit must NOT flip to a large negative (the old wrap bug).
        let r = big + Fixed::from_int(1);
        assert_eq!(r.raw(), i32::MAX);
        assert!(r.raw() > 0, "must not wrap to negative");
    }

    #[test]
    fn test_sub_saturates_at_min() {
        let small = Fixed(i32::MIN);
        let r = small - Fixed::from_int(1);
        assert_eq!(r.raw(), i32::MIN);
    }

    #[test]
    fn test_mul_overflow_saturates() {
        let big = Fixed::from_int(30000);
        // 30000 * 30000 far exceeds the Q16.16 integer range.
        let r = big.mul(big);
        assert_eq!(
            r.raw(),
            i32::MAX,
            "overflowing mul must saturate, not flip sign"
        );
    }

    #[test]
    fn test_div_by_zero_saturates_by_sign() {
        assert_eq!(Fixed::from_int(5).div(Fixed::ZERO).raw(), i32::MAX);
        assert_eq!(Fixed::from_int(-5).div(Fixed::ZERO).raw(), i32::MIN);
    }

    #[test]
    fn test_from_int_saturates_out_of_range() {
        // Above the Q16.16 integer ceiling: must clamp positive, never flip sign.
        assert_eq!(Fixed::from_int(32768).raw(), i32::MAX);
        assert!(
            Fixed::from_int(40_000).raw() > 0,
            "must not wrap to negative"
        );
        // Below the floor clamps to the minimum.
        assert_eq!(Fixed::from_int(-40_000).raw(), i32::MIN);
        // In-range values are still exact (regression guard for the common path).
        assert_eq!(Fixed::from_int(1000).to_int_trunc(), 1000);
    }

    #[test]
    fn test_from_ratio_div_by_zero_saturates_by_sign() {
        // Zero denominator must saturate, not panic (matches Fixed::div).
        assert_eq!(Fixed::from_ratio(1, 0).raw(), i32::MAX);
        assert_eq!(Fixed::from_ratio(-1, 0).raw(), i32::MIN);
        // An out-of-range quotient clamps instead of wrapping via `as i32`.
        assert_eq!(Fixed::from_ratio(i32::MAX, 1).raw(), i32::MAX);
    }

    /// Asserts `a` and `b` are within `tol` raw units (Q16.16 LSBs).
    fn approx(a: Fixed, b: Fixed, tol: i32) {
        let diff = (a.raw() - b.raw()).abs();
        assert!(
            diff <= tol,
            "expected {:?} ≈ {:?} (within {} raw), diff = {}",
            a,
            b,
            tol,
            diff
        );
    }

    #[test]
    fn test_sqrt_perfect_squares_are_exact() {
        assert_eq!(Fixed::from_int(4).sqrt(), Fixed::from_int(2));
        assert_eq!(Fixed::from_int(9).sqrt(), Fixed::from_int(3));
        assert_eq!(Fixed::from_int(144).sqrt(), Fixed::from_int(12));
        assert_eq!(Fixed::ZERO.sqrt(), Fixed::ZERO);
    }

    #[test]
    fn test_sqrt_of_two_is_close() {
        // √2 ≈ 1.41421; integer-isqrt floors, so allow a couple of LSBs.
        let root2 = Fixed::from_int(2).sqrt();
        approx(root2, Fixed::from_ratio(141_421, 100_000), 4);
    }

    #[test]
    fn test_sqrt_negative_saturates_to_zero() {
        // √ of a negative is undefined: clamp to zero, never panic or wrap.
        assert_eq!(Fixed::from_int(-4).sqrt(), Fixed::ZERO);
        assert_eq!(Fixed(i32::MIN).sqrt(), Fixed::ZERO);
    }

    #[test]
    fn test_sqrt_is_inverse_of_squaring() {
        for n in [1, 5, 7, 16, 100, 1000, 10_000] {
            let v = Fixed::from_int(n);
            let r = v.sqrt();
            // r*r should recover v within rounding (floor isqrt undershoots).
            approx(r.mul(r), v, 1 << 9);
        }
    }

    #[test]
    fn test_sin_cos_cardinal_angles() {
        let tol = 64; // ≈0.001 in Q16.16
                      // sin(0)=0, cos(0)=1
        approx(Fixed::ZERO.sin(), Fixed::ZERO, tol);
        approx(Fixed::ZERO.cos(), Fixed::ONE, tol);
        // sin(π/2)=1, cos(π/2)=0
        let half_pi = Fixed(HALF_PI);
        approx(half_pi.sin(), Fixed::ONE, tol);
        approx(half_pi.cos(), Fixed::ZERO, tol);
        // sin(π)=0, cos(π)=-1
        let pi = Fixed(PI);
        approx(pi.sin(), Fixed::ZERO, tol);
        approx(pi.cos(), Fixed::ZERO - Fixed::ONE, tol);
        // sin(-π/2)=-1, cos(-π/2)=0
        let neg_half_pi = Fixed(-HALF_PI);
        approx(neg_half_pi.sin(), Fixed::ZERO - Fixed::ONE, tol);
        approx(neg_half_pi.cos(), Fixed::ZERO, tol);
    }

    #[test]
    fn test_sin_cos_pythagorean_identity() {
        // sin²θ + cos²θ = 1 for a spread of angles, incl. out-of-range ones that
        // exercise range reduction.
        for raw in [-500_000i32, -123_456, -1, 0, 12_345, 205_887, 800_000] {
            let (s, c) = Fixed(raw).sin_cos();
            let sum = s.mul(s) + c.mul(c);
            approx(sum, Fixed::ONE, 96);
        }
    }

    #[test]
    fn test_atan2_cardinals_and_quadrants() {
        let tol = 64;
        let one = Fixed::ONE;
        let neg_one = Fixed::ZERO - Fixed::ONE;
        // Axes.
        approx(Fixed::atan2(Fixed::ZERO, one), Fixed::ZERO, tol); // +x
        approx(Fixed::atan2(one, Fixed::ZERO), Fixed(HALF_PI), tol); // +y
        approx(Fixed::atan2(Fixed::ZERO, neg_one), Fixed(PI), tol); // -x
        approx(Fixed::atan2(neg_one, Fixed::ZERO), Fixed(-HALF_PI), tol); // -y
        approx(Fixed::atan2(Fixed::ZERO, Fixed::ZERO), Fixed::ZERO, tol); // origin
                                                                          // Diagonals: ±π/4, ±3π/4.
        let quarter_pi = Fixed(HALF_PI / 2);
        approx(Fixed::atan2(one, one), quarter_pi, tol);
        approx(Fixed::atan2(neg_one, one), Fixed::ZERO - quarter_pi, tol);
        approx(Fixed::atan2(one, neg_one), Fixed(PI) - quarter_pi, tol);
    }

    #[test]
    fn test_atan2_inverts_sin_cos() {
        // For an angle in (-π, π], atan2(sin, cos) recovers it.
        for raw in [-200_000i32, -100_000, -1000, 0, 50_000, 150_000, 205_000] {
            let a = Fixed(raw);
            let (s, c) = a.sin_cos();
            approx(Fixed::atan2(s, c), a, 128);
        }
    }

    #[test]
    fn test_trig_is_deterministic() {
        // Same input → byte-identical output (no float, no global state).
        let a = Fixed(123_456);
        assert_eq!(a.sin_cos(), a.sin_cos());
        assert_eq!(
            Fixed::atan2(a, Fixed::from_int(2)),
            Fixed::atan2(a, Fixed::from_int(2))
        );
        assert_eq!(Fixed::from_int(50).sqrt(), Fixed::from_int(50).sqrt());
    }
}

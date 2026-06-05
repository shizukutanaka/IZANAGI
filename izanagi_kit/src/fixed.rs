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

/// Fixed-point number stored as a scaled `i32`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Fixed(i32);

impl Fixed {
    pub const ZERO: Fixed = Fixed(0);
    pub const ONE: Fixed = Fixed(ONE);

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
}

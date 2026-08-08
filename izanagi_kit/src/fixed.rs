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
//!
//! # Why `i64` intermediates are exactly enough
//!
//! A wider intermediate (`i128`) is sometimes proposed for fixed-point
//! multiply. Here it would be pure overhead, because the value is a Q16.16
//! stored in an `i32` and the bound is provable by counting bits:
//!
//! - **Multiply** widens both raw operands to `i64` before multiplying. With
//!   `|raw| ≤ 2³¹`, the exact product satisfies `|a·b| ≤ 2⁶²`, which is inside
//!   `i64::MAX = 2⁶³ − 1`. The extreme case `i32::MIN · i32::MIN = 2⁶²` is the
//!   worst input that exists, and it still fits with a bit to spare.
//! - **Divide** shifts the widened numerator left by 16 first: `|raw << 16| ≤
//!   2⁴⁷`, sixteen orders of magnitude below the `i64` ceiling. The one way an
//!   `i64` division can overflow — `i64::MIN / -1` — is unreachable, since the
//!   numerator never approaches `i64::MIN`.
//!
//! So the `i64` intermediate cannot overflow for **any** pair of `Fixed`
//! values, and saturation is decided solely by the internal `from_wide` clamp
//! applied to the *result*. Widening to `i128` would change no output and
//! only cost speed. `test_mul_i64_intermediate_cannot_overflow` and
//! `test_div_i64_intermediate_cannot_overflow` machine-check the bound with
//! `checked_mul`/`checked_shl` at the sign extremes.
//!
//! # Rounding
//!
//! There is no configurable rounding mode; each operation has one fixed,
//! deterministic behaviour, documented here so callers can reason about the
//! last bit. The behaviours differ between multiply and divide because each
//! uses the cheapest correct integer primitive, and the two primitives round
//! negatives differently:
//!
//! - [`mul`](Fixed::mul) shifts the 64-bit product right by 16
//!   (`product >> 16`). An **arithmetic** right shift **floors toward −∞**, so
//!   a fractional negative product rounds *down* (e.g. raw `1 × -1` yields raw
//!   `-1`, not `0`).
//! - [`div`](Fixed::div) and [`from_ratio`](Fixed::from_ratio) use integer
//!   division (`(a << 16) / b`), which in Rust **truncates toward zero**, so a
//!   fractional negative quotient rounds *up* toward zero (e.g. `-1 / 3`
//!   yields raw `-21845`, i.e. `-0.33332`, not the floored `-21846`).
//! - [`to_int_trunc`](Fixed::to_int_trunc) truncates toward −∞ (it is a bare
//!   `>> 16`); [`round`](Fixed::round) rounds to nearest with ties going toward
//!   +∞ (`0.5 → 1`, `-0.5 → 0`); [`floor`](Fixed::floor) and
//!   [`mid`](Fixed::mid) also floor toward −∞.
//!
//! For non-negative operands every operation above coincides (floor,
//! truncate-toward-zero, and round-down are all the same), so the distinction
//! only matters for negative values. The exact behaviours are pinned by the
//! `test_mul_rounds_toward_negative_infinity`, `test_div_truncates_toward_zero`,
//! and `test_from_ratio_truncates_toward_zero_like_div` regression tests below.

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
    /// The value `0`.
    pub const ZERO: Fixed = Fixed(0);
    /// The value `1`.
    pub const ONE: Fixed = Fixed(ONE);
    /// The largest representable value, `raw() == i32::MAX`.
    pub const MAX: Fixed = Fixed(i32::MAX);
    /// The smallest (most negative) representable value, `raw() == i32::MIN`.
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
    /// `from_wide` (a private helper) instead of a wrapping `as i32` cast.
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

    /// The raw Q16.16 integer representation (value × 65536, truncated toward
    /// −∞ at construction time by the operation that produced it).
    #[inline]
    pub fn raw(self) -> i32 {
        self.0
    }

    /// Convert to the integer part via a bare arithmetic `>> 16`.
    ///
    /// **Name caveat**: this floors toward −∞, it does *not* truncate toward
    /// zero. For non-negative values the two are identical (the common case,
    /// hence the historical name), but for negatives it rounds *down*:
    /// `to_int_trunc(-1.5) == -2`, not `-1`. If you need round-half-to-nearest
    /// use [`round`](Self::round) / [`to_int_round`](Self::to_int_round); the
    /// behaviour here is deliberately kept (renaming or changing it would be a
    /// breaking API/semantics change touching the pinned-hash simulation) and
    /// is pinned by `test_to_int_trunc_floors_negatives`.
    #[inline]
    pub fn to_int_trunc(self) -> i32 {
        self.0 >> FRAC_BITS
    }

    /// Add, saturating at [`Fixed::MAX`]/[`Fixed::MIN`] instead of wrapping on overflow.
    #[inline]
    pub fn saturating_add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_add(rhs.0))
    }

    /// Named-method mirror of the [`Sub`](core::ops::Sub) operator (which
    /// already saturates), added for API-matrix completeness and for generic
    /// code that can't spell `-` — exactly like [`Fixed::saturating_add`]
    /// mirrors [`Add`](core::ops::Add).
    #[inline]
    pub fn saturating_sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.saturating_sub(rhs.0))
    }

    /// Checked addition on the raw `i32`. Returns `None` iff the addition
    /// would overflow `i32`, instead of the saturating behaviour of `+`.
    #[inline]
    pub fn checked_add(self, rhs: Fixed) -> Option<Fixed> {
        self.0.checked_add(rhs.0).map(Fixed)
    }

    /// Checked subtraction on the raw `i32`. Returns `None` iff the
    /// subtraction would overflow `i32`, instead of the saturating behaviour
    /// of `-`.
    #[inline]
    pub fn checked_sub(self, rhs: Fixed) -> Option<Fixed> {
        self.0.checked_sub(rhs.0).map(Fixed)
    }

    /// Wrapping addition on the raw `i32`: overflow silently wraps around
    /// instead of saturating or panicking. Prefer `+` or
    /// [`Fixed::saturating_add`] unless wraparound is explicitly wanted.
    #[inline]
    pub fn wrapping_add(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.wrapping_add(rhs.0))
    }

    /// Wrapping subtraction on the raw `i32`: overflow silently wraps around
    /// instead of saturating or panicking. Prefer `-` or
    /// [`Fixed::saturating_sub`] unless wraparound is explicitly wanted.
    #[inline]
    pub fn wrapping_sub(self, rhs: Fixed) -> Fixed {
        Fixed(self.0.wrapping_sub(rhs.0))
    }

    /// Overflowing addition on the raw `i32`. Returns the wrapped result
    /// alongside a `bool` that is `true` iff overflow occurred.
    #[inline]
    pub fn overflowing_add(self, rhs: Fixed) -> (Fixed, bool) {
        let (v, o) = self.0.overflowing_add(rhs.0);
        (Fixed(v), o)
    }

    /// Overflowing subtraction on the raw `i32`. Returns the wrapped result
    /// alongside a `bool` that is `true` iff overflow occurred.
    #[inline]
    pub fn overflowing_sub(self, rhs: Fixed) -> (Fixed, bool) {
        let (v, o) = self.0.overflowing_sub(rhs.0);
        (Fixed(v), o)
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
    /// Multiply. See the module-level "Rounding" section: floors toward −∞ on
    /// a fractional negative product.
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn mul(self, rhs: Fixed) -> Fixed {
        Fixed::from_wide(((self.0 as i64) * rhs.0 as i64) >> FRAC_BITS)
    }

    /// Checked multiplication. Same rounding as [`Fixed::mul`] (the i64
    /// intermediate is shifted right, i.e. floors toward -infinity), but
    /// returns `None` instead of saturating when the exact product falls
    /// outside the representable `i32` range.
    #[inline]
    pub fn checked_mul(self, rhs: Fixed) -> Option<Fixed> {
        let wide = ((self.0 as i64) * (rhs.0 as i64)) >> FRAC_BITS;
        if wide > i32::MAX as i64 || wide < i32::MIN as i64 {
            None
        } else {
            Some(Fixed(wide as i32))
        }
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

    /// Checked division. Same rounding as [`Fixed::div`] (the i64
    /// intermediate is truncated toward zero by integer division), but
    /// returns `None` instead of saturating: on a zero divisor, and on a
    /// quotient whose exact value falls outside the representable `i32`
    /// range.
    #[inline]
    pub fn checked_div(self, rhs: Fixed) -> Option<Fixed> {
        if rhs.0 == 0 {
            return None;
        }
        let wide = ((self.0 as i64) << FRAC_BITS) / rhs.0 as i64;
        if wide > i32::MAX as i64 || wide < i32::MIN as i64 {
            None
        } else {
            Some(Fixed(wide as i32))
        }
    }

    /// Absolute value. `Fixed::MIN.abs()` saturates to `Fixed::MAX` (no panic).
    #[inline]
    pub fn abs(self) -> Fixed {
        if self.0 == i32::MIN {
            Fixed::MAX
        } else {
            Fixed(self.0.abs())
        }
    }

    /// Absolute difference `|self − other|`. Saturating — never wraps.
    ///
    /// Equivalent to `(self - other).abs()` but without the intermediate
    /// saturated subtraction losing sign information. The correct primitive
    /// for "how far apart are two fixed-point values?" range checks.
    #[inline]
    pub fn abs_diff(self, other: Fixed) -> Fixed {
        if self.0 >= other.0 {
            Fixed(self.0.saturating_sub(other.0))
        } else {
            Fixed(other.0.saturating_sub(self.0))
        }
    }

    /// Square: equivalent to `self.mul(self)`. Saturates at `Fixed::MAX`/`MIN`
    /// like `mul`. Useful for squared-distance checks and quadratic formulas
    /// without writing `x.mul(x)` at every call site.
    #[inline]
    pub fn pow2(self) -> Fixed {
        self.mul(self)
    }

    /// Sign: returns `Fixed::ONE` for positive, `Fixed::ZERO` for zero, and
    /// `Fixed::ONE.neg()` (i.e. `-1`) for negative. Deterministic.
    #[inline]
    pub fn sign(self) -> Fixed {
        match self.0.cmp(&0) {
            core::cmp::Ordering::Greater => Fixed::ONE,
            core::cmp::Ordering::Less => Fixed(-(ONE)),
            core::cmp::Ordering::Equal => Fixed::ZERO,
        }
    }

    /// Clamp to `[lo, hi]`. If `lo > hi` the result is `lo` (no panic).
    #[inline]
    pub fn clamp(self, lo: Fixed, hi: Fixed) -> Fixed {
        if self.0 < lo.0 {
            lo
        } else if self.0 > hi.0 {
            hi
        } else {
            self
        }
    }

    /// Clamp to `[0, 1]` — shorthand for `clamp(Fixed::ZERO, Fixed::ONE)`.
    /// Used so frequently in easing and animation code that the common case
    /// deserves a dedicated name.
    #[inline]
    pub fn clamp01(self) -> Fixed {
        self.clamp(Fixed::ZERO, Fixed::ONE)
    }

    /// Advance `self` toward `target` by at most `|step|`. If already at
    /// `target` (or `step == 0`) returns `self` unchanged. Saturates, never
    /// overshoots. Useful for velocity ramps, health-bar drains, and AI
    /// approach logic without manual clamp branches at call sites.
    #[inline]
    pub fn step_toward(self, target: Fixed, step: Fixed) -> Fixed {
        let step = step.abs();
        if self.0 < target.0 {
            Fixed(self.0.saturating_add(step.0).min(target.0))
        } else if self.0 > target.0 {
            Fixed(self.0.saturating_sub(step.0).max(target.0))
        } else {
            self
        }
    }

    /// True when `self` lies in the closed interval `[lo, hi]`. Equivalent to
    /// `self >= lo && self <= hi` but reads more clearly at complex call sites.
    #[inline]
    pub fn in_range(self, lo: Fixed, hi: Fixed) -> bool {
        self.0 >= lo.0 && self.0 <= hi.0
    }

    /// Midpoint of `self` and `other`: `(self + other) / 2`, **rounding toward
    /// negative infinity** (floor) on an odd raw sum — consistent with
    /// [`floor`](Self::floor) and [`fract`](Self::fract), not C-style
    /// truncation. For example, the midpoint of raw `-1` and `0` is raw `-1`,
    /// not `0`. Avoids the manual `(a + b) / 2` overflow hazard — intermediate
    /// addition uses `i64`. Useful for bisection and centering.
    #[inline]
    pub fn mid(self, other: Fixed) -> Fixed {
        Fixed(((self.0 as i64 + other.0 as i64) >> 1) as i32)
    }

    /// Linear interpolation: `a + (b - a) * t`, where `t` is in `[0, 1]`.
    /// `t` outside `[0, 1]` extrapolates (no clamping). Uses saturating
    /// arithmetic so overflow is pinned rather than wrapping.
    #[inline]
    pub fn lerp(a: Fixed, b: Fixed, t: Fixed) -> Fixed {
        a + (b - a).mul(t)
    }

    /// Square root. Negative inputs saturate to zero (√ of a negative is
    /// undefined; clamping is the safe deterministic choice rather than panicking
    /// or producing garbage). Computed with an integer-only `isqrt_u64` helper so the
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

    /// Round toward negative infinity (largest integer ≤ self).
    ///
    /// Implemented as arithmetic-shift-right then shift-left, which propagates
    /// the sign bit and never overflows for any in-range value.
    #[inline]
    pub fn floor(self) -> Fixed {
        Fixed((self.0 >> FRAC_BITS) << FRAC_BITS)
    }

    /// Round toward positive infinity (smallest integer ≥ self).
    #[inline]
    pub fn ceil(self) -> Fixed {
        let frac = self.0 & (ONE - 1);
        if frac == 0 {
            self
        } else {
            Fixed(self.floor().0.saturating_add(ONE))
        }
    }

    /// Round to the nearest integer. Ties round away from negative infinity
    /// (i.e. `round(0.5) == 1`, `round(-0.5) == 0`).
    #[inline]
    pub fn round(self) -> Fixed {
        Fixed(self.0.saturating_add(ONE / 2)).floor()
    }

    /// Fractional part: `self - self.floor()`. Always in `[0, 1)`.
    ///
    /// Note: this follows the floor convention, not IEEE 754 (where `fract` of a
    /// negative number is negative). `fract(-1.5)` returns `0.5`, not `-0.5`.
    #[inline]
    pub fn fract(self) -> Fixed {
        Fixed(self.0.wrapping_sub(self.floor().0))
    }

    /// Round to the nearest integer and return it as `i32`.
    /// Ties round away from negative infinity (`0.5 → 1`, `-0.5 → 0`), matching
    /// `self.round().to_int_trunc()`. More ergonomic than chaining both calls when
    /// only the integer result is needed (e.g., UI pixel coordinates, grid snapping).
    #[inline]
    pub fn to_int_round(self) -> i32 {
        self.round().to_int_trunc()
    }

    /// Component-wise minimum: `self` if `self <= other`, otherwise `other`.
    /// Saturates rather than panicking on extremes. Useful for bounding
    /// fixed-point positions to map extents without branching at call sites.
    #[inline]
    pub fn min(self, other: Fixed) -> Fixed {
        if self.0 <= other.0 {
            self
        } else {
            other
        }
    }

    /// Component-wise maximum: `self` if `self >= other`, otherwise `other`.
    #[inline]
    pub fn max(self, other: Fixed) -> Fixed {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }

    /// Reciprocal: `1 / self`. Saturates to `Fixed::MAX` for `self = 0`.
    /// Equivalent to `Fixed::ONE.div(self)` — provided as a named method for
    /// readability at call sites where division by a variable is the intent
    /// (e.g., normalising a vector component: `v.mul(magnitude.recip())`).
    #[inline]
    pub fn recip(self) -> Fixed {
        Fixed::ONE.div(self)
    }

    /// Returns `true` when this value is exactly zero.
    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` when this value is strictly greater than zero.
    #[inline]
    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Returns `true` when this value is strictly less than zero.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Returns `true` when the fractional part is exactly zero (i.e. the value
    /// represents a whole number). Equivalent to `self.fract().is_zero()`.
    /// Useful for grid-snapping guards ("only move if the position is on a cell
    /// boundary") and animation completion checks.
    #[inline]
    pub fn is_integer(self) -> bool {
        self.fract().is_zero()
    }

    /// Integer exponentiation: `self` raised to `exp`, via repeated saturating
    /// multiplication. `pow(0)` is `Fixed::ONE` for any base; `pow(1)` is `self`
    /// exactly. Each multiply saturates (overflow pins to `Fixed::MAX`/`MIN`
    /// rather than wrapping). Cost is `exp` multiplies — keep `exp` small.
    #[inline]
    pub fn pow(self, exp: u32) -> Fixed {
        let mut result = Fixed::ONE;
        for _ in 0..exp {
            result = result.mul(self);
        }
        result
    }

    /// Euclidean distance `sqrt(a² + b²)`, rounded down. Useful for 2-D range
    /// checks and normalisation without a separate manual `sqrt`. Saturates for
    /// very large inputs the same way `mul` and `sqrt` do — safe for typical
    /// roguelike coordinate ranges.
    #[inline]
    pub fn hypot(a: Fixed, b: Fixed) -> Fixed {
        a.mul(a).saturating_add(b.mul(b)).sqrt()
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

    /// Sign of `self`: returns `1` if positive, `-1` if negative, `0` if zero.
    /// Deterministic integer-only signum (no branching on sign bits, all paths
    /// identical cost).
    #[inline]
    pub fn signum(self) -> i32 {
        if self.0 > 0 {
            1
        } else if self.0 < 0 {
            -1
        } else {
            0
        }
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

impl core::ops::Neg for Fixed {
    type Output = Fixed;
    /// Saturating negation: `Fixed::MIN.neg()` returns `Fixed::MAX` (no panic).
    #[inline]
    fn neg(self) -> Fixed {
        if self.0 == i32::MIN {
            Fixed::MAX
        } else {
            Fixed(-self.0)
        }
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

    #[test]
    fn test_abs_positive_unchanged() {
        assert_eq!(Fixed::from_int(3).abs(), Fixed::from_int(3));
    }

    #[test]
    fn test_abs_negative_negated() {
        assert_eq!(Fixed::from_int(-5).abs(), Fixed::from_int(5));
    }

    #[test]
    fn test_abs_min_saturates_not_wraps() {
        assert_eq!(Fixed::MIN.abs(), Fixed::MAX);
    }

    #[test]
    fn test_sign_positive_negative_zero() {
        let neg_one = Fixed::ZERO - Fixed::ONE;
        assert_eq!(Fixed::from_int(7).sign(), Fixed::ONE);
        assert_eq!(Fixed::ZERO.sign(), Fixed::ZERO);
        assert_eq!(Fixed::from_int(-3).sign(), neg_one);
    }

    #[test]
    fn test_clamp_within_range_unchanged() {
        let v = Fixed::from_int(5);
        assert_eq!(v.clamp(Fixed::from_int(1), Fixed::from_int(10)), v);
    }

    #[test]
    fn test_clamp_below_lo_returns_lo() {
        let lo = Fixed::from_int(2);
        assert_eq!(Fixed::from_int(-1).clamp(lo, Fixed::from_int(10)), lo);
    }

    #[test]
    fn test_clamp_above_hi_returns_hi() {
        let hi = Fixed::from_int(8);
        assert_eq!(Fixed::from_int(100).clamp(Fixed::from_int(1), hi), hi);
    }

    #[test]
    fn test_neg_positive() {
        assert_eq!((-Fixed::from_int(3)).to_int_trunc(), -3);
    }

    #[test]
    fn test_neg_negative() {
        assert_eq!((-Fixed::from_int(-5)).to_int_trunc(), 5);
    }

    #[test]
    fn test_neg_zero() {
        assert_eq!(-Fixed::ZERO, Fixed::ZERO);
    }

    #[test]
    fn test_neg_min_saturates_to_max() {
        assert_eq!(-Fixed::MIN, Fixed::MAX);
    }

    #[test]
    fn test_neg_double_is_identity() {
        let v = Fixed::from_int(7);
        assert_eq!(-(-v), v);
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = Fixed::from_int(0);
        let b = Fixed::from_int(10);
        assert_eq!(Fixed::lerp(a, b, Fixed::ZERO), a);
        assert_eq!(Fixed::lerp(a, b, Fixed::ONE), b);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = Fixed::from_int(0);
        let b = Fixed::from_int(10);
        let half = Fixed::from_ratio(1, 2);
        approx(Fixed::lerp(a, b, half), Fixed::from_int(5), 4);
    }

    #[test]
    fn test_floor_positive() {
        assert_eq!(Fixed::from_ratio(3, 2).floor(), Fixed::from_int(1)); // 1.5 → 1
        assert_eq!(Fixed::from_int(3).floor(), Fixed::from_int(3)); // integer exact
    }

    #[test]
    fn test_floor_negative() {
        let neg15 = -(Fixed::from_ratio(3, 2)); // -1.5
        assert_eq!(neg15.floor(), Fixed::from_int(-2)); // -1.5 → -2
        assert_eq!(Fixed::from_int(-2).floor(), Fixed::from_int(-2)); // exact
    }

    #[test]
    fn test_ceil_positive() {
        assert_eq!(Fixed::from_ratio(3, 2).ceil(), Fixed::from_int(2)); // 1.5 → 2
        assert_eq!(Fixed::from_int(2).ceil(), Fixed::from_int(2)); // exact
    }

    #[test]
    fn test_ceil_negative() {
        let neg15 = -(Fixed::from_ratio(3, 2)); // -1.5
        assert_eq!(neg15.ceil(), Fixed::from_int(-1)); // -1.5 → -1
    }

    #[test]
    fn test_round_half_rounds_up() {
        assert_eq!(Fixed::from_ratio(1, 2).round(), Fixed::from_int(1)); // 0.5 → 1
        assert_eq!(Fixed::from_ratio(3, 2).round(), Fixed::from_int(2)); // 1.5 → 2
    }

    // --- rounding-mode pins (see the "Rounding" section of the module docs) ---

    #[test]
    fn test_mul_rounds_toward_negative_infinity() {
        // mul uses an arithmetic `>>`, which floors (rounds toward -inf) for
        // negatives. raw 1 (= 2^-16) times raw -1: the true product's raw is
        // -1, and -1 >> 16 == -1. Truncation toward zero would give 0.
        assert_eq!(
            Fixed(1).mul(Fixed(-1)).raw(),
            -1,
            "mul floors negatives toward -inf"
        );
        // Positive operands: floor and truncation agree, so no surprise there.
        assert_eq!(
            Fixed(1).mul(Fixed(1)).raw(),
            0,
            "tiny positive product floors to 0"
        );
    }

    #[test]
    fn test_div_truncates_toward_zero() {
        // div uses integer `/`, which truncates toward zero. -1 / 3 in Q16.16
        // is -0.3333…; its exact raw is -21845.33…, and integer division
        // truncates the magnitude to 21845 → raw -21845. Flooring toward -inf
        // would instead give -21846.
        assert_eq!(
            Fixed::from_int(-1).div(Fixed::from_int(3)).raw(),
            -21845,
            "div truncates negatives toward zero (not floor)"
        );
        // The positive counterpart truncates to the same magnitude.
        assert_eq!(
            Fixed::from_int(1).div(Fixed::from_int(3)).raw(),
            21845,
            "div truncates positives toward zero"
        );
    }

    #[test]
    fn test_from_ratio_truncates_toward_zero_like_div() {
        // from_ratio shares div's integer-division path, so it truncates too.
        assert_eq!(Fixed::from_ratio(-1, 3).raw(), -21845);
        assert_eq!(Fixed::from_ratio(1, 3).raw(), 21845);
    }

    #[test]
    fn test_to_int_trunc_floors_negatives() {
        // Despite its name, to_int_trunc is a bare arithmetic `>> 16`, so it
        // floors toward -inf rather than truncating toward zero. This pins
        // that documented behaviour (and its divergence from the name) so a
        // well-meaning "fix" toward true truncation can't silently shift the
        // pinned-hash simulation.
        assert_eq!(
            (-Fixed::from_ratio(3, 2)).to_int_trunc(),
            -2,
            "-1.5 floors to -2"
        );
        assert_eq!(
            Fixed::from_ratio(3, 2).to_int_trunc(),
            1,
            "1.5 truncates to 1"
        );
        assert_eq!(
            Fixed::from_int(-2).to_int_trunc(),
            -2,
            "exact negatives are unchanged"
        );
    }

    #[test]
    fn test_round_negative_half() {
        // -0.5 rounds to 0 (half-up convention: -0.5 + 0.5 = 0, then floor = 0)
        let neg_half = -Fixed::from_ratio(1, 2);
        assert_eq!(neg_half.round(), Fixed::ZERO);
    }

    #[test]
    fn test_fract_positive() {
        let f = Fixed::from_ratio(3, 2).fract(); // fract(1.5) = 0.5
        assert_eq!(f, Fixed::from_ratio(1, 2));
    }

    #[test]
    fn test_fract_negative() {
        let f = (-(Fixed::from_ratio(3, 2))).fract(); // fract(-1.5) = 0.5 (floor convention)
        assert_eq!(f, Fixed::from_ratio(1, 2));
    }

    #[test]
    fn test_fract_integer_is_zero() {
        assert_eq!(Fixed::from_int(3).fract(), Fixed::ZERO);
        assert_eq!(Fixed::from_int(-2).fract(), Fixed::ZERO);
    }

    #[test]
    fn test_to_int_round_half_rounds_up() {
        // 0.5 → 1, -0.5 → 0 (floor-convention ties).
        assert_eq!(Fixed::from_ratio(1, 2).to_int_round(), 1);
        assert_eq!((-Fixed::from_ratio(1, 2)).to_int_round(), 0);
    }

    #[test]
    fn test_to_int_round_integer_unchanged() {
        assert_eq!(Fixed::from_int(3).to_int_round(), 3);
        assert_eq!(Fixed::from_int(-2).to_int_round(), -2);
    }

    #[test]
    fn test_to_int_round_fraction_truncated() {
        assert_eq!(Fixed::from_ratio(7, 4).to_int_round(), 2); // 1.75 → 2
        assert_eq!(Fixed::from_ratio(5, 4).to_int_round(), 1); // 1.25 → 1
    }

    #[test]
    fn test_min_max_return_extremes() {
        let a = Fixed::from_int(3);
        let b = Fixed::from_int(7);
        assert_eq!(a.min(b), a);
        assert_eq!(a.max(b), b);
        assert_eq!(b.min(a), a);
        assert_eq!(b.max(a), b);
    }

    #[test]
    fn test_min_max_equal_returns_self() {
        let v = Fixed::from_ratio(3, 4);
        assert_eq!(v.min(v), v);
        assert_eq!(v.max(v), v);
    }

    #[test]
    fn test_recip_of_two() {
        // 1 / 2.0 = 0.5
        let result = Fixed::from_int(2).recip();
        let expected = Fixed::from_ratio(1, 2);
        let diff = (result.raw() - expected.raw()).abs();
        assert!(diff <= 2, "recip(2) off by {diff} LSBs");
    }

    #[test]
    fn test_recip_of_zero_saturates() {
        // 1 / 0 → Fixed::MAX (saturating divide)
        assert_eq!(Fixed::ZERO.recip(), Fixed::MAX);
    }

    #[test]
    fn test_hypot_3_4_gives_5() {
        let a = Fixed::from_int(3);
        let b = Fixed::from_int(4);
        let h = Fixed::hypot(a, b);
        // sqrt(9 + 16) = 5.0 exactly in Q16.16
        assert_eq!(h, Fixed::from_int(5));
    }

    #[test]
    fn test_hypot_zero_gives_zero() {
        assert_eq!(Fixed::hypot(Fixed::ZERO, Fixed::ZERO), Fixed::ZERO);
    }

    #[test]
    fn test_hypot_symmetric() {
        let a = Fixed::from_int(6);
        let b = Fixed::from_int(8);
        assert_eq!(Fixed::hypot(a, b), Fixed::hypot(b, a));
    }

    #[test]
    fn test_pow_zero_is_one() {
        assert_eq!(Fixed::from_int(7).pow(0), Fixed::ONE);
        assert_eq!(Fixed::ZERO.pow(0), Fixed::ONE);
    }

    #[test]
    fn test_pow_one_is_self() {
        let v = Fixed::from_ratio(3, 2);
        assert_eq!(v.pow(1), v);
    }

    #[test]
    fn test_pow_squares_and_cubes() {
        assert_eq!(Fixed::from_int(3).pow(2), Fixed::from_int(9));
        assert_eq!(Fixed::from_int(2).pow(3), Fixed::from_int(8));
        // (1/2)^2 = 1/4
        assert_eq!(Fixed::from_ratio(1, 2).pow(2), Fixed::from_ratio(1, 4));
    }

    #[test]
    fn test_pow_saturates_not_wraps() {
        // 30000^2 far exceeds the Q16.16 range → pins to MAX, never flips sign.
        let r = Fixed::from_int(30000).pow(2);
        assert_eq!(r.raw(), i32::MAX);
    }

    #[test]
    fn test_is_zero_only_for_zero() {
        assert!(Fixed::ZERO.is_zero());
        assert!(!Fixed::ONE.is_zero());
        assert!(!Fixed::from_int(-1).is_zero());
    }

    #[test]
    fn test_is_positive_for_positive_values() {
        assert!(Fixed::ONE.is_positive());
        assert!(!Fixed::ZERO.is_positive());
        assert!(!Fixed::from_int(-1).is_positive());
    }

    #[test]
    fn test_is_negative_for_negative_values() {
        assert!(Fixed::from_int(-1).is_negative());
        assert!(!Fixed::ZERO.is_negative());
        assert!(!Fixed::ONE.is_negative());
    }

    #[test]
    fn test_is_integer_whole_numbers() {
        assert!(Fixed::from_int(0).is_integer());
        assert!(Fixed::from_int(5).is_integer());
        assert!(Fixed::from_int(-3).is_integer());
    }

    #[test]
    fn test_is_integer_fractional_returns_false() {
        assert!(!Fixed::from_ratio(1, 2).is_integer());
        assert!(!Fixed::from_ratio(3, 4).is_integer());
        assert!(!Fixed::from_ratio(-1, 3).is_integer());
    }

    #[test]
    fn test_is_integer_floor_and_ceil() {
        let v = Fixed::from_ratio(7, 2);
        assert!(!v.is_integer());
        assert!(v.floor().is_integer());
        assert!(v.ceil().is_integer());
    }

    #[test]
    fn test_clamp01_clamps_below_zero() {
        assert_eq!(Fixed::from_int(-5).clamp01(), Fixed::ZERO);
    }

    #[test]
    fn test_clamp01_clamps_above_one() {
        assert_eq!(Fixed::from_int(3).clamp01(), Fixed::ONE);
    }

    #[test]
    fn test_clamp01_interior_unchanged() {
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(half.clamp01(), half);
    }

    #[test]
    fn test_abs_diff_positive_operands() {
        let a = Fixed::from_int(5);
        let b = Fixed::from_int(3);
        assert_eq!(a.abs_diff(b), Fixed::from_int(2));
        assert_eq!(b.abs_diff(a), Fixed::from_int(2));
    }

    #[test]
    fn test_abs_diff_across_zero() {
        let a = Fixed::from_int(-2);
        let b = Fixed::from_int(3);
        assert_eq!(a.abs_diff(b), Fixed::from_int(5));
    }

    #[test]
    fn test_abs_diff_equal_is_zero() {
        let a = Fixed::from_ratio(7, 4);
        assert_eq!(a.abs_diff(a), Fixed::ZERO);
    }

    // --- pow2 ---

    #[test]
    fn test_pow2_integer_two() {
        assert_eq!(Fixed::from_int(2).pow2(), Fixed::from_int(4));
    }

    #[test]
    fn test_pow2_negative_same_as_positive() {
        let pos = Fixed::from_int(3).pow2();
        let neg = Fixed::from_int(-3).pow2();
        assert_eq!(pos, neg);
    }

    #[test]
    fn test_pow2_zero_is_zero() {
        assert_eq!(Fixed::ZERO.pow2(), Fixed::ZERO);
    }

    // --- step_toward ---

    #[test]
    fn test_step_toward_advances_by_step() {
        let v = Fixed::from_int(0).step_toward(Fixed::from_int(10), Fixed::from_int(3));
        assert_eq!(v, Fixed::from_int(3));
    }

    #[test]
    fn test_step_toward_does_not_overshoot() {
        let v = Fixed::from_int(8).step_toward(Fixed::from_int(10), Fixed::from_int(5));
        assert_eq!(v, Fixed::from_int(10));
    }

    #[test]
    fn test_step_toward_already_at_target() {
        let v = Fixed::from_int(7).step_toward(Fixed::from_int(7), Fixed::from_int(3));
        assert_eq!(v, Fixed::from_int(7));
    }

    #[test]
    fn test_step_toward_decreasing() {
        let v = Fixed::from_int(10).step_toward(Fixed::from_int(0), Fixed::from_int(4));
        assert_eq!(v, Fixed::from_int(6));
    }

    // --- in_range ---

    #[test]
    fn test_in_range_inclusive_bounds() {
        let lo = Fixed::from_int(1);
        let hi = Fixed::from_int(5);
        assert!(Fixed::from_int(1).in_range(lo, hi));
        assert!(Fixed::from_int(5).in_range(lo, hi));
        assert!(Fixed::from_int(3).in_range(lo, hi));
    }

    #[test]
    fn test_in_range_outside_returns_false() {
        let lo = Fixed::from_int(1);
        let hi = Fixed::from_int(5);
        assert!(!Fixed::from_int(0).in_range(lo, hi));
        assert!(!Fixed::from_int(6).in_range(lo, hi));
    }

    #[test]
    fn test_in_range_inverted_bounds_always_false() {
        assert!(!Fixed::from_int(3).in_range(Fixed::from_int(5), Fixed::from_int(1)));
    }

    // --- mid ---

    #[test]
    fn test_mid_integers() {
        let a = Fixed::from_int(2);
        let b = Fixed::from_int(8);
        assert_eq!(a.mid(b), Fixed::from_int(5));
    }

    #[test]
    fn test_mid_symmetric() {
        let a = Fixed::from_int(3);
        let b = Fixed::from_int(9);
        assert_eq!(a.mid(b), b.mid(a));
    }

    #[test]
    fn test_mid_self_is_identity() {
        let v = Fixed::from_int(7);
        assert_eq!(v.mid(v), v);
    }

    #[test]
    fn test_mid_odd_sum_floors_toward_neg_infinity() {
        // An odd raw sum must floor toward -inf (matching `floor`/`fract`),
        // NOT truncate toward zero. mid(raw -1, raw 0) == raw -1, not 0.
        assert_eq!(Fixed(-1).mid(Fixed(0)).raw(), -1, "negative odd sum floors");
        assert_eq!(Fixed(-3).mid(Fixed(0)).raw(), -2, "(-3+0)>>1 == -2");
        // Positive odd sum: floor and truncate agree.
        assert_eq!(Fixed(1).mid(Fixed(0)).raw(), 0, "(1+0)>>1 == 0");
        assert_eq!(Fixed(3).mid(Fixed(0)).raw(), 1, "(3+0)>>1 == 1");
    }

    #[test]
    fn test_mid_extremes_no_overflow() {
        // The i64 intermediate must prevent overflow at the extremes.
        assert_eq!(Fixed::MAX.mid(Fixed::MAX), Fixed::MAX);
        assert_eq!(Fixed::MIN.mid(Fixed::MIN), Fixed::MIN);
        // Midpoint of the full span floors to raw -1 ((MAX+MIN)>>1 = -1>>1 = -1).
        assert_eq!(Fixed::MAX.mid(Fixed::MIN).raw(), -1);
    }

    #[test]
    fn test_signum_positive() {
        assert_eq!(Fixed::from_int(5).signum(), 1);
        assert_eq!(Fixed(0x0001).signum(), 1);
    }

    #[test]
    fn test_signum_negative() {
        assert_eq!(Fixed::from_int(-5).signum(), -1);
        assert_eq!(Fixed(-0x0001).signum(), -1);
    }

    #[test]
    fn test_signum_zero() {
        assert_eq!(Fixed::ZERO.signum(), 0);
    }

    #[test]
    fn test_signum_extremes() {
        assert_eq!(Fixed::MAX.signum(), 1);
        assert_eq!(Fixed::MIN.signum(), -1);
    }

    // --- checked/wrapping/overflowing arithmetic matrix ---

    #[test]
    fn test_checked_add_normal_and_overflow() {
        assert_eq!(
            Fixed::from_int(2).checked_add(Fixed::from_int(3)),
            Some(Fixed::from_int(5))
        );
        // i32::MAX + 1 overflows the raw i32: must be None, not a saturated Some.
        assert_eq!(Fixed(i32::MAX).checked_add(Fixed::from_int(1)), None);
    }

    #[test]
    fn test_checked_sub_normal_and_overflow() {
        assert_eq!(
            Fixed::from_int(5).checked_sub(Fixed::from_int(3)),
            Some(Fixed::from_int(2))
        );
        // i32::MIN - 1 overflows the raw i32: must be None, not a saturated Some.
        assert_eq!(Fixed(i32::MIN).checked_sub(Fixed::from_int(1)), None);
    }

    #[test]
    fn test_saturating_sub_matches_operator_and_normal_case() {
        assert_eq!(
            Fixed::from_int(5).saturating_sub(Fixed::from_int(3)),
            Fixed::from_int(2)
        );
        // Named method must agree exactly with the `-` operator, including at
        // the saturating boundary.
        let small = Fixed(i32::MIN);
        assert_eq!(
            small.saturating_sub(Fixed::from_int(1)),
            small - Fixed::from_int(1)
        );
        assert_eq!(small.saturating_sub(Fixed::from_int(1)).raw(), i32::MIN);
    }

    #[test]
    fn test_wrapping_add_normal_and_wraps() {
        assert_eq!(
            Fixed::from_int(2).wrapping_add(Fixed::from_int(3)),
            Fixed::from_int(5)
        );
        // i32::MAX + 1 wraps around to i32::MIN, unlike the saturating `+`.
        assert_eq!(Fixed(i32::MAX).wrapping_add(Fixed(1)), Fixed(i32::MIN));
    }

    #[test]
    fn test_wrapping_sub_normal_and_wraps() {
        assert_eq!(
            Fixed::from_int(5).wrapping_sub(Fixed::from_int(3)),
            Fixed::from_int(2)
        );
        // i32::MIN - 1 wraps around to i32::MAX, unlike the saturating `-`.
        assert_eq!(Fixed(i32::MIN).wrapping_sub(Fixed(1)), Fixed(i32::MAX));
    }

    #[test]
    fn test_overflowing_add_normal_and_overflow() {
        assert_eq!(
            Fixed::from_int(2).overflowing_add(Fixed::from_int(3)),
            (Fixed::from_int(5), false)
        );
        assert_eq!(
            Fixed(i32::MAX).overflowing_add(Fixed(1)),
            (Fixed(i32::MIN), true)
        );
    }

    #[test]
    fn test_overflowing_sub_normal_and_overflow() {
        assert_eq!(
            Fixed::from_int(5).overflowing_sub(Fixed::from_int(3)),
            (Fixed::from_int(2), false)
        );
        assert_eq!(
            Fixed(i32::MIN).overflowing_sub(Fixed(1)),
            (Fixed(i32::MAX), true)
        );
    }

    #[test]
    fn test_checked_mul_normal_and_overflow() {
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(half.checked_mul(half), Some(Fixed::from_ratio(1, 4)));
        let big = Fixed::from_int(30000);
        assert_eq!(big.checked_mul(big), None);
    }

    #[test]
    fn test_checked_div_normal_zero_and_overflow() {
        let a = Fixed::from_int(6);
        let b = Fixed::from_int(3);
        assert_eq!(a.checked_div(b), Some(Fixed::from_int(2)));
        // Zero divisor: `div` saturates by sign, `checked_div` must be None.
        assert_eq!(Fixed::from_int(5).checked_div(Fixed::ZERO), None);
        // MAX / 0.5 doubles the raw value past i32::MAX: `div` would saturate,
        // `checked_div` must return None instead.
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(Fixed(i32::MAX).checked_div(half), None);
    }

    // --- i64 intermediate width is provably sufficient (see module docs) ---

    /// Every raw value that can sit at a sign/magnitude extreme, plus a few
    /// ordinary ones. If the `i64` product bound holds here it holds
    /// everywhere, since `|a·b|` is maximised at the extremes.
    const EXTREME_RAWS: [i32; 9] = [
        i32::MIN,
        i32::MIN + 1,
        -ONE,
        -1,
        0,
        1,
        ONE,
        i32::MAX - 1,
        i32::MAX,
    ];

    #[test]
    fn test_mul_i64_intermediate_cannot_overflow() {
        // The claim: widening both raws to i64 and multiplying never overflows.
        // checked_mul returning Some for every extreme pair is the machine
        // proof of |a·b| <= 2^62 < i64::MAX.
        for &a in &EXTREME_RAWS {
            for &b in &EXTREME_RAWS {
                assert!(
                    (a as i64).checked_mul(b as i64).is_some(),
                    "i64 product overflowed for raws {a} * {b}"
                );
            }
        }
        // The worst case is exactly i32::MIN squared = 2^62.
        let worst = (i32::MIN as i64) * (i32::MIN as i64);
        assert_eq!(worst, 1i64 << 62);
        assert!(worst < i64::MAX, "2^62 fits in i64 with a bit to spare");
    }

    #[test]
    fn test_div_i64_intermediate_cannot_overflow() {
        // The numerator is the raw widened then shifted left by FRAC_BITS.
        for &a in &EXTREME_RAWS {
            let shifted = (a as i64)
                .checked_shl(FRAC_BITS)
                .expect("raw << 16 must fit in i64");
            assert!(
                shifted.abs() <= 1i64 << 47,
                "|raw << 16| must stay within 2^47, got {shifted}"
            );
            // i64::MIN / -1 is the only overflowing division; the numerator
            // never reaches i64::MIN, so it is unreachable.
            assert_ne!(shifted, i64::MIN);
            for &b in &EXTREME_RAWS {
                if b != 0 {
                    assert!(
                        shifted.checked_div(b as i64).is_some(),
                        "i64 division overflowed for raws {a} / {b}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_mul_at_extremes_matches_widened_reference() {
        // Behavioural counterpart: at every extreme the public op agrees with
        // the exact i64 computation it claims to perform. Running to completion
        // is itself the evidence that no intermediate overflowed.
        for &a in &EXTREME_RAWS {
            for &b in &EXTREME_RAWS {
                let reference = Fixed::from_wide(((a as i64) * (b as i64)) >> FRAC_BITS);
                assert_eq!(
                    Fixed(a).mul(Fixed(b)),
                    reference,
                    "mul disagreed with the widened reference at raws {a} * {b}"
                );
            }
        }
        // A product far outside the representable range clamps to MAX/MIN.
        assert_eq!(Fixed::MAX.mul(Fixed::MAX), Fixed::MAX);
        assert_eq!(Fixed::MAX.mul(Fixed::MIN), Fixed::MIN);
    }

    #[test]
    fn test_div_at_extremes_matches_widened_reference() {
        for &a in &EXTREME_RAWS {
            for &b in &EXTREME_RAWS {
                let out = Fixed(a).div(Fixed(b));
                if b == 0 {
                    // Documented divide-by-zero-safe: saturates by the
                    // numerator's sign rather than panicking, with a
                    // non-negative numerator (including zero) giving MAX.
                    let expect = if a >= 0 { Fixed::MAX } else { Fixed::MIN };
                    assert_eq!(out, expect, "div by zero at raw {a}");
                } else {
                    let reference = Fixed::from_wide(((a as i64) << FRAC_BITS) / (b as i64));
                    assert_eq!(
                        out, reference,
                        "div disagreed with the widened reference at raws {a} / {b}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_mul_i64_bound_holds_over_a_wide_sweep() {
        // Beyond the extremes: sweep a large spread of raw magnitudes and
        // confirm the bound never comes close to the i64 ceiling.
        let mut rng = crate::rng::SplitMix64::new(0xF1_7ED);
        for _ in 0..20_000 {
            let a = rng.next_u64() as i32;
            let b = rng.next_u64() as i32;
            let product = (a as i64)
                .checked_mul(b as i64)
                .expect("i64 product must never overflow");
            assert!(product.abs() <= 1i64 << 62);
            // And the public op agrees with the widened reference computation.
            let expect = Fixed::from_wide(product >> FRAC_BITS);
            assert_eq!(Fixed(a).mul(Fixed(b)), expect);
        }
    }
}
